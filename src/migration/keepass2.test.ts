// 测试在 node 环境运行（vitest.config.ts），读取测试资产需要 node 类型；
// 以三斜杠指令局部引入，避免 tsconfig types 全局引入 node 类型污染浏览器代码类型空间。
/// <reference types="node" />
import { readFileSync } from "node:fs";
import * as kdbxweb from "kdbxweb";
import { describe, expect, it } from "vitest";

import {
  installArgon2,
  layoutImportedTree,
  parseKeepass2Database,
  type ImportedTree,
  type Keepass2FieldText,
} from "./keepass2";
import type { ImportedEdge, ImportedNode } from "@/api-types";

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
function blankNode(): ImportedNode {
  return { title: "", sub_title: "", x: 0, y: 0, fields: [] };
}

/**
 * 构造布局测试用的树：count 个无字段节点加给定的边（下标引用节点列表）。
 */
function blankTree(nodeCount: number, edges: ImportedEdge[]): ImportedTree {
  return {
    nodes: Array.from({ length: nodeCount }, () => blankNode()),
    edges,
  };
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
    expect(nodes[0].sub_title).toBe("");
    expect(nodes[0].fields).toEqual([]);

    // 索引 1：UserName 非空 → title 取 UserName、sub_title 取 Title，3 个字段。
    expect(nodes[1].title).toBe("User Name");
    expect(nodes[1].sub_title).toBe("Sample Entry");
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

    // 索引 2：UserName 与 Title 均空 → title/sub_title 均为空串，仅 1 个备注字段。
    expect(nodes[2].title).toBe("");
    expect(nodes[2].sub_title).toBe("");
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
    expect(nodes[3].sub_title).toBe("");
    expect(nodes[3].fields).toEqual([]);

    // 索引 4：无备注 → 仅密码与访问链接 2 个字段。
    expect(nodes[4].title).toBe("Michael321");
    expect(nodes[4].sub_title).toBe("Sample Entry #2");
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
    expect(nodes[9].sub_title).toBe("asdf");
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

    // 索引 1：根 group 的 entry（UserName 非空 → title=UserName、sub_title=Title），
    // 密码（ProtectedValue 存储）与访问链接（明文存储）两种形式均应取出明文。
    expect(nodes[1].title).toBe("kdbx4-user");
    expect(nodes[1].sub_title).toBe("Kdbx4 Entry");
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
    expect(nodes[2].sub_title).toBe("");
    expect(nodes[2].fields).toEqual([]);

    // 索引 3：子 group 内 UserName 为空的 entry → title=Title，仅 1 个备注字段。
    expect(nodes[3].title).toBe("Sub Entry");
    expect(nodes[3].sub_title).toBe("");
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
