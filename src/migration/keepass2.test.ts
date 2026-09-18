// 测试在 node 环境运行（vitest.config.ts），读取测试资产需要 node 类型；
// 以三斜杠指令局部引入，避免 tsconfig types 全局引入 node 类型污染浏览器代码类型空间。
/// <reference types="node" />
import { readFileSync } from "node:fs";
import * as kdbxweb from "kdbxweb";
import { describe, expect, it } from "vitest";

import {
  installArgon2,
  layoutImportedCanvases,
  layoutImportedTree,
  parseKeepass2Database,
  parseKeepass2DatabaseCanvases,
  type ImportedTree,
  type Keepass2FieldText,
} from "./keepass2";
import type { ImportedCanvasVO, ImportedEdgeVO, ImportedNodeVO } from "@/api-types";

/** 测试用字段名文案（取 zh-CN 表，与旧后端实现的中文文案一致） */
const TEXT: Keepass2FieldText = {
  password: "密码",
  url: "访问链接",
  notes: "备注",
};

/**
 * 读取 KeePass 2.0 测试数据库（KDBX 3.1 格式，Master Password 为 "demopass"，
 * 内容来自 KeePass 官方示例，含回收站 group）。
 * readFileSync 返回的 Buffer 是 Uint8Array 的子类且 slice 不拷贝，
 * 先拷贝为独立的 Uint8Array 避免底层缓冲与视图长度不一致。
 */
function readTestDatabase(): Uint8Array {
  return new Uint8Array(
    readFileSync(new URL("./test_assets/test_db_with_password.kdbx", import.meta.url)),
  );
}

/**
 * 现场构造 KDBX 4.0（Argon2id KDF）数据库字节：kdbxweb 新建数据库默认即
 * KDBX 4.0 格式，setKdf 切换为 Argon2id（KeePass 2.x 新建数据库的默认 KDF）。
 * 默认 KDF 内存参数为 1 GiB，对单测过重，降为 8 MiB（覆盖的代码路径相同）。
 * 内容：根 group（名 "Kdbx4Root"）1 个 entry（UserName 非空，Password 存为保护值、
 * URL 存为明文）与 1 个子 group（内含 1 个 UserName 为空的 entry，仅有备注字段）。
 */
async function createKdbx4Database(): Promise<Uint8Array> {
  // 保存时的 KDF 计算同样依赖注入的 argon2 实现（幂等，重复调用无副作用）。
  installArgon2();
  const credentials = new kdbxweb.Credentials(
    kdbxweb.ProtectedValue.fromString("kdbx4pass"),
  );
  const db = kdbxweb.Kdbx.create(credentials, "Kdbx4Root");
  db.setKdf(kdbxweb.Consts.KdfId.Argon2id);
  const kdfParameters = db.header.kdfParameters;
  if (kdfParameters === undefined) {
    throw new Error("kdbxweb 未生成 Argon2id KDF 参数");
  }
  kdfParameters.set(
    "M",
    kdbxweb.VarDictionary.ValueType.UInt64,
    new kdbxweb.Int64(8 * 1024),
  );

  const root = db.getDefaultGroup();
  const entry = db.createEntry(root);
  entry.fields.set("Title", "Kdbx4 Entry");
  entry.fields.set("UserName", "kdbx4-user");
  entry.fields.set("Password", kdbxweb.ProtectedValue.fromString("kdbx4-secret"));
  entry.fields.set("URL", "https://example.com/");

  const subgroup = db.createGroup(root, "Kdbx4Group");
  const subEntry = db.createEntry(subgroup);
  subEntry.fields.set("Title", "Sub Entry");
  subEntry.fields.set("Notes", "line1\nline2");

  return new Uint8Array(await db.save());
}

/** 构造布局测试用的无字段节点。 */
function blankNode(): ImportedNodeVO {
  return { title: "", subtitle: "", x: 0, y: 0, fields: [] };
}

/**
 * 构造布局测试用的树：count 个无字段节点加给定的边（下标引用节点列表）。
 */
function blankTree(nodeCount: number, edges: ImportedEdgeVO[]): ImportedTree {
  return {
    nodes: Array.from({ length: nodeCount }, () => blankNode()),
    edges,
  };
}

/** 构造布局测试用的空画布（无数据节点、无画布数据节点）。 */
function blankCanvas(parentIndex: number | null): ImportedCanvasVO {
  return { name: "", x: 0, y: 0, parent_index: parentIndex, nodes: [], canvas_nodes: [] };
}

describe("parseKeepass2Database", () => {
  it("成功解析官方测试数据库为树形结构", async () => {
    // 意图：成功路径。根 group 产出表示数据库本身的根节点（索引 0），回收站 group
    // 及其内容不产出；entry 与 group 节点按深度优先先序排列，共 10 个节点、
    // 9 条父子边（每个非根节点恰好一条父边）。
    const tree = await parseKeepass2Database(readTestDatabase(), "demopass", TEXT);
    const { nodes, edges } = tree;

    expect(nodes).toHaveLength(10);
    expect(nodes.every((node) => node.fields.every((f) => f.dictionary_id === null))).toBe(true);

    // 索引 0：表示数据库本身的根节点（title=根 group 名、无字段）。
    expect(nodes[0].title).toBe("sample");
    expect(nodes[0].subtitle).toBe("");
    expect(nodes[0].fields).toEqual([]);

    // 索引 1：UserName 非空 → title 取 UserName、subtitle 取 Title，3 个字段。
    expect(nodes[1].title).toBe("User Name");
    expect(nodes[1].subtitle).toBe("Sample Entry");
    expect(nodes[1].fields).toEqual([
      { name: "密码", field_type: "string:password", value: "Password", dictionary_id: null },
      {
        name: "访问链接",
        field_type: "string:url",
        value: "http://keepass.info/",
        dictionary_id: null,
      },
      { name: "备注", field_type: "string:multiple-line", value: "Notes", dictionary_id: null },
    ]);

    // 索引 2：UserName 与 Title 均空 → title/subtitle 均为空串，仅 1 个备注字段。
    expect(nodes[2].title).toBe("");
    expect(nodes[2].subtitle).toBe("");
    expect(nodes[2].fields).toEqual([
      {
        name: "备注",
        field_type: "string:multiple-line",
        value: "This entry has an empty title, username, and password",
        dictionary_id: null,
      },
    ]);

    // 索引 3：group 节点（title=group 名、无字段）。
    expect(nodes[3].title).toBe("General");
    expect(nodes[3].subtitle).toBe("");
    expect(nodes[3].fields).toEqual([]);

    // 索引 4：无备注 → 仅密码与访问链接 2 个字段。
    expect(nodes[4].title).toBe("Michael321");
    expect(nodes[4].subtitle).toBe("Sample Entry #2");
    expect(nodes[4].fields).toEqual([
      { name: "密码", field_type: "string:password", value: "12345", dictionary_id: null },
      {
        name: "访问链接",
        field_type: "string:url",
        value: "http://keepass.info/help/kb/testform.html",
        dictionary_id: null,
      },
    ]);

    // 索引 6：子 group 节点。
    expect(nodes[6].title).toBe("Subgroup");
    expect(nodes[6].fields).toEqual([]);

    // 索引 9：仅 1 个密码字段（无访问链接与备注）。
    expect(nodes[9].title).toBe("asdf");
    expect(nodes[9].subtitle).toBe("asdf");
    expect(nodes[9].fields).toEqual([
      {
        name: "密码",
        field_type: "string:password",
        value: "K8JexrYVUD6Av1OsWguo",
        dictionary_id: null,
      },
    ]);

    // 父子边完整断言：根节点（0）的两个 entry（1、2）与两个子 group（3 General、
    // 8 Internet）挂在根下；General 的两个 entry（4、5）与子 group（6 Subgroup）
    // 挂在 3 下；Subgroup 的 entry（7）挂在 6 下；Internet 的 entry（9）挂在 8 下。
    expect(edges).toEqual([
      { source_index: 0, target_index: 1 },
      { source_index: 0, target_index: 2 },
      { source_index: 0, target_index: 3 },
      { source_index: 3, target_index: 4 },
      { source_index: 3, target_index: 5 },
      { source_index: 3, target_index: 6 },
      { source_index: 6, target_index: 7 },
      { source_index: 0, target_index: 8 },
      { source_index: 8, target_index: 9 },
    ]);
  });

  it("错误密码报 incorrect-password", async () => {
    // 意图：失败路径。Master Password 不正确时抛出 "incorrect-password"。
    await expect(
      parseKeepass2Database(readTestDatabase(), "wrong", TEXT),
    ).rejects.toBe("incorrect-password");
  });

  it("垃圾字节报 invalid-file", async () => {
    // 意图：失败路径。文件签名无法识别时抛出 "invalid-file"。
    const garbage = new TextEncoder().encode("this is not a kdbx file");
    await expect(parseKeepass2Database(garbage, "demopass", TEXT)).rejects.toBe("invalid-file");
  });
});

describe("parseKeepass2Database（KDBX 4.0 / Argon2id）", () => {
  it("成功解析现场构造的 KDBX 4.0 数据库为树形结构", async () => {
    // 意图：成功路径。验证 KDBX 4.0 格式与 Argon2id KDF（installArgon2 注入的
    // argon2 实现）可用；树结构规则与 KDBX 3.1 一致：根 group 产出根节点，
    // 共 4 个节点、3 条父子边。
    const tree = await parseKeepass2Database(await createKdbx4Database(), "kdbx4pass", TEXT);
    const { nodes, edges } = tree;

    expect(nodes).toHaveLength(4);

    // 索引 0：表示数据库本身的根节点（title=根 group 名 "Kdbx4Root"）。
    expect(nodes[0].title).toBe("Kdbx4Root");
    expect(nodes[0].fields).toEqual([]);

    // 索引 1：根 group 的 entry（UserName 非空 → title=UserName、subtitle=Title），
    // 密码（ProtectedValue 存储）与访问链接（明文存储）两种形式均应取出明文。
    expect(nodes[1].title).toBe("kdbx4-user");
    expect(nodes[1].subtitle).toBe("Kdbx4 Entry");
    expect(nodes[1].fields).toEqual([
      { name: "密码", field_type: "string:password", value: "kdbx4-secret", dictionary_id: null },
      {
        name: "访问链接",
        field_type: "string:url",
        value: "https://example.com/",
        dictionary_id: null,
      },
    ]);

    // 索引 2：子 group 节点（title=group 名、无字段）。
    expect(nodes[2].title).toBe("Kdbx4Group");
    expect(nodes[2].subtitle).toBe("");
    expect(nodes[2].fields).toEqual([]);

    // 索引 3：子 group 内 UserName 为空的 entry → title=Title，仅 1 个备注字段。
    expect(nodes[3].title).toBe("Sub Entry");
    expect(nodes[3].subtitle).toBe("");
    expect(nodes[3].fields).toEqual([
      {
        name: "备注",
        field_type: "string:multiple-line",
        value: "line1\nline2",
        dictionary_id: null,
      },
    ]);

    // 父子边：根 → entry、根 → 子 group、子 group → 子 entry。
    expect(edges).toEqual([
      { source_index: 0, target_index: 1 },
      { source_index: 0, target_index: 2 },
      { source_index: 2, target_index: 3 },
    ]);
  });

  it("KDBX 4.0 数据库错误密码报 incorrect-password", async () => {
    // 意图：失败路径。KDBX 4 头部 HMAC 校验失败应映射为 "incorrect-password"。
    await expect(
      parseKeepass2Database(await createKdbx4Database(), "wrong", TEXT),
    ).rejects.toBe("incorrect-password");
  });
});

describe("layoutImportedTree", () => {
  it("单根节点布局在原点", async () => {
    // 意图：树只有表示数据库本身的根节点（无边）时落在 (0, 0)。
    const tree = blankTree(1, []);
    layoutImportedTree(tree);
    expect([tree.nodes[0].x, tree.nodes[0].y]).toEqual([0, 0]);
  });

  it("子节点排在右侧列并占据连续行，父节点纵向居中", async () => {
    // 意图：根节点的 3 个子节点位于第 1 列（x=240），y 按行间距 160 连续递增；
    // 根节点 x=0，y 取首尾子节点的均值（(0+320)/2=160）。
    const tree = blankTree(4, [
      { source_index: 0, target_index: 1 },
      { source_index: 0, target_index: 2 },
      { source_index: 0, target_index: 3 },
    ]);
    layoutImportedTree(tree);
    expect([tree.nodes[1].x, tree.nodes[1].y]).toEqual([240, 0]);
    expect([tree.nodes[2].x, tree.nodes[2].y]).toEqual([240, 160]);
    expect([tree.nodes[3].x, tree.nodes[3].y]).toEqual([240, 320]);
    expect([tree.nodes[0].x, tree.nodes[0].y]).toEqual([0, 160]);
  });

  it("不均衡子树中父节点仍纵向居中于其首尾子节点", async () => {
    // 意图：root[c1[g1, g2], c2] 的不均衡树：叶子 g1、g2、c2 依次占行
    // 0/1/2（y=0/160/320），c1 居中于 g1 与 g2（y=80），root 居中于 c1 与 c2
    //（y=(80+320)/2=200）；列深依次为 x=0/240/480。
    const tree = blankTree(5, [
      { source_index: 0, target_index: 1 },
      { source_index: 0, target_index: 2 },
      { source_index: 1, target_index: 3 },
      { source_index: 1, target_index: 4 },
    ]);
    layoutImportedTree(tree);
    expect([tree.nodes[3].x, tree.nodes[3].y]).toEqual([480, 0]);
    expect([tree.nodes[4].x, tree.nodes[4].y]).toEqual([480, 160]);
    expect([tree.nodes[1].x, tree.nodes[1].y]).toEqual([240, 80]);
    expect([tree.nodes[2].x, tree.nodes[2].y]).toEqual([240, 320]);
    expect([tree.nodes[0].x, tree.nodes[0].y]).toEqual([0, 200]);
  });
});

describe("parseKeepass2DatabaseCanvases", () => {
  it("成功解析官方测试数据库为画布列表", async () => {
    // 意图：成功路径。每个 group 产出一张画布（含根 group），回收站 group 及其内容
    // 不产出；画布按深度优先先序排列（父画布下标恒小于子画布），共 4 张画布：
    // 根 group "sample"（2 个 entry + 子 group General、Internet）、General
    //（2 个 entry + 子 group Subgroup）、Subgroup（1 个 entry）、Internet（1 个 entry）。
    const canvases = await parseKeepass2DatabaseCanvases(readTestDatabase(), "demopass", TEXT);

    expect(canvases).toHaveLength(4);

    // 画布 0：根 group 画布（name=根 group 名、parent_index=null），含 2 个 entry
    // 数据节点与 2 个画布数据节点（引用 General 与 Internet，下标引用先序位置）。
    expect(canvases[0].name).toBe("sample");
    expect(canvases[0].parent_index).toBeNull();
    expect(canvases[0].nodes).toHaveLength(2);
    expect(canvases[0].canvas_nodes).toEqual([
      { x: 0, y: 0, ref_index: 1 },
      { x: 0, y: 0, ref_index: 3 },
    ]);

    // 根 group 的 entry 转换规则与单画布路线一致：UserName 非空 → title=UserName、
    // subtitle=Title，字段顺序 密码 → 访问链接 → 备注。
    expect(canvases[0].nodes[0].title).toBe("User Name");
    expect(canvases[0].nodes[0].subtitle).toBe("Sample Entry");
    expect(canvases[0].nodes[0].fields).toEqual([
      { name: "密码", field_type: "string:password", value: "Password", dictionary_id: null },
      {
        name: "访问链接",
        field_type: "string:url",
        value: "http://keepass.info/",
        dictionary_id: null,
      },
      { name: "备注", field_type: "string:multiple-line", value: "Notes", dictionary_id: null },
    ]);

    // 画布 1：General（父画布为 0），含 2 个 entry 与 1 个画布数据节点（引用 Subgroup）。
    expect(canvases[1].name).toBe("General");
    expect(canvases[1].parent_index).toBe(0);
    expect(canvases[1].nodes.map((node) => node.title)).toEqual(["Michael321", "Michael3210"]);
    expect(canvases[1].canvas_nodes).toEqual([{ x: 0, y: 0, ref_index: 2 }]);

    // 画布 2：Subgroup（父画布为 1），含 1 个 entry（UserName 非空 → title=UserName），
    // 无画布数据节点。
    expect(canvases[2].name).toBe("Subgroup");
    expect(canvases[2].parent_index).toBe(1);
    expect(canvases[2].nodes.map((node) => node.title)).toEqual(["jdoe"]);
    expect(canvases[2].canvas_nodes).toEqual([]);

    // 画布 3：Internet（父画布为 0），含 1 个 entry。
    expect(canvases[3].name).toBe("Internet");
    expect(canvases[3].parent_index).toBe(0);
    expect(canvases[3].nodes.map((node) => node.title)).toEqual(["asdf"]);
  });

  it("错误密码报 incorrect-password", async () => {
    // 意图：失败路径。Master Password 不正确时抛出 "incorrect-password"。
    await expect(
      parseKeepass2DatabaseCanvases(readTestDatabase(), "wrong", TEXT),
    ).rejects.toBe("incorrect-password");
  });

  it("垃圾字节报 invalid-file", async () => {
    // 意图：失败路径。文件签名无法识别时抛出 "invalid-file"。
    const garbage = new TextEncoder().encode("this is not a kdbx file");
    await expect(parseKeepass2DatabaseCanvases(garbage, "demopass", TEXT)).rejects.toBe(
      "invalid-file",
    );
  });

  it("成功解析现场构造的 KDBX 4.0 数据库为画布列表", async () => {
    // 意图：成功路径（KDBX 4.0 / Argon2id）。根 group（1 个 entry + 1 个子 group）
    // 产出 2 张画布，子 group 画布挂到根 group 画布（parent_index=0）。
    const canvases = await parseKeepass2DatabaseCanvases(
      await createKdbx4Database(),
      "kdbx4pass",
      TEXT,
    );

    expect(canvases).toHaveLength(2);
    expect(canvases[0].name).toBe("Kdbx4Root");
    expect(canvases[0].parent_index).toBeNull();
    expect(canvases[0].nodes.map((node) => node.title)).toEqual(["kdbx4-user"]);
    expect(canvases[0].canvas_nodes).toEqual([{ x: 0, y: 0, ref_index: 1 }]);
    expect(canvases[1].name).toBe("Kdbx4Group");
    expect(canvases[1].parent_index).toBe(0);
    expect(canvases[1].nodes.map((node) => node.title)).toEqual(["Sub Entry"]);
    expect(canvases[1].canvas_nodes).toEqual([]);
  });
});

describe("layoutImportedCanvases", () => {
  it("单张画布布局在空闲点（根画布右侧 240）", async () => {
    // 意图：宇宙布局的树根取环形搜索的第一个空闲点——根画布在 (0, 0) 时，
    // 半径 240、角度 0 的候选点 (240, 0) 空闲；单张画布（无子画布）即落于该点。
    const canvases = [blankCanvas(null)];
    layoutImportedCanvases(canvases, { x: 0, y: 0 }, [{ x: 0, y: 0 }]);
    expect([canvases[0].x, canvases[0].y]).toEqual([240, 0]);
  });

  it("空闲点搜索避让现存画布", async () => {
    // 意图：候选点 (240, 0) 已被现存画布占据、45° 候选点与其距离不足 200 时，
    // 环形搜索继续向外取到 90° 候选点 (0, 240)（与两个现存画布的距离均不小于 200）。
    const canvases = [blankCanvas(null)];
    layoutImportedCanvases(canvases, { x: 0, y: 0 }, [
      { x: 0, y: 0 },
      { x: 240, y: 0 },
    ]);
    expect(canvases[0].x).toBeCloseTo(0, 10);
    expect(canvases[0].y).toBeCloseTo(240, 10);
  });

  it("子画布按层级排在右侧列并占据连续行，父画布纵向居中", async () => {
    // 意图：root[c1[g1, g2], c2] 的不均衡画布树：叶子 g1、g2、c2 依次占行
    // 0/1/2（y=0/160/320），c1 居中于 g1 与 g2（y=80），root 居中于 c1 与 c2
    //（y=(80+320)/2=200）；各画布 x = 树根 x（240）+ 深度 × 240。
    const canvases = [
      blankCanvas(null),
      blankCanvas(0),
      blankCanvas(0),
      blankCanvas(1),
      blankCanvas(1),
    ];
    layoutImportedCanvases(canvases, { x: 0, y: 0 }, [{ x: 0, y: 0 }]);
    expect([canvases[3].x, canvases[3].y]).toEqual([720, 0]);
    expect([canvases[4].x, canvases[4].y]).toEqual([720, 160]);
    expect([canvases[1].x, canvases[1].y]).toEqual([480, 80]);
    expect([canvases[2].x, canvases[2].y]).toEqual([480, 320]);
    expect([canvases[0].x, canvases[0].y]).toEqual([240, 200]);
  });

  it("画布内数据节点与画布数据节点按矩阵平铺（数据节点在前）", async () => {
    // 意图：画布内 3 个数据节点 + 1 个画布数据节点共 4 项，列数 ceil(sqrt(4))=2：
    // 依次落于 (0,0)、(240,0)、(0,160)、(240,160)，画布数据节点排在数据节点之后。
    const canvas = blankCanvas(null);
    canvas.nodes = [blankNode(), blankNode(), blankNode()];
    canvas.canvas_nodes = [{ x: 0, y: 0, ref_index: 0 }];
    const canvases = [canvas];
    layoutImportedCanvases(canvases, { x: 0, y: 0 }, [{ x: 0, y: 0 }]);
    expect([canvas.nodes[0].x, canvas.nodes[0].y]).toEqual([0, 0]);
    expect([canvas.nodes[1].x, canvas.nodes[1].y]).toEqual([240, 0]);
    expect([canvas.nodes[2].x, canvas.nodes[2].y]).toEqual([0, 160]);
    expect([canvas.canvas_nodes[0].x, canvas.canvas_nodes[0].y]).toEqual([240, 160]);
  });

  it("矩阵列数按 ceil(sqrt(n)) 取值", async () => {
    // 意图：5 个数据节点的画布列数 ceil(sqrt(5))=3：前 3 个占满第 0 行
    //（y=0），第 4、5 个落于第 1 行的前两列（y=160）。
    const canvas = blankCanvas(null);
    canvas.nodes = [blankNode(), blankNode(), blankNode(), blankNode(), blankNode()];
    const canvases = [canvas];
    layoutImportedCanvases(canvases, { x: 0, y: 0 }, [{ x: 0, y: 0 }]);
    expect([canvas.nodes[0].x, canvas.nodes[0].y]).toEqual([0, 0]);
    expect([canvas.nodes[1].x, canvas.nodes[1].y]).toEqual([240, 0]);
    expect([canvas.nodes[2].x, canvas.nodes[2].y]).toEqual([480, 0]);
    expect([canvas.nodes[3].x, canvas.nodes[3].y]).toEqual([0, 160]);
    expect([canvas.nodes[4].x, canvas.nodes[4].y]).toEqual([240, 160]);
  });
});
