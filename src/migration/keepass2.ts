/**
 * KeePass 2.0 数据库（.kdbx）前端解析模块。
 *
 * 职责：接收文件字节与 Master Password，在前端完成解密与解析（kdbxweb，KDBX 3.1 与 4.0），
 * 深度优先遍历 group/entry 树，产出待导入的树形节点数据（ImportedTree：节点 + 父子边，
 * 坐标为占位 0，由 layoutImportedTree 写入树形布局坐标）。
 * 树结构的根节点表示数据库本身（对应 KeePass 根 group），父节点在左、子节点在右。
 * 不负责文件读取（走后端字节通道）与写库（走后端聚合导入接口）。
 * 字段名文案由调用方按当前语言传入，本模块不做国际化。
 *
 * 错误约定：parseKeepass2Database 失败时直接 throw Keepass2ParseError 字符串值
 * （非 Error 对象），调用方以 catch 到的值判别失败原因。
 */
import * as kdbxweb from "kdbxweb";
import { argon2dAsync, argon2iAsync, argon2idAsync } from "@noble/hashes/argon2.js";
import type { ImportedEdge, ImportedNode, NodeFieldVO } from "@/api-types";

/** 字段名文案集合：三项分别为 KeePass 的 Password（密码）、URL（访问链接）、
 * Notes（备注）字段的字段名文案。 */
export interface Keepass2FieldText {
  /** Password 字段（密码）的字段名文案 */
  password: string;
  /** URL 字段（访问链接）的字段名文案 */
  url: string;
  /** Notes 字段（备注）的字段名文案 */
  notes: string;
}

/** 解析失败的原因码："incorrect-password" 为密码错误，"invalid-file" 为文件无效（无法解析/解密） */
export type Keepass2ParseError = "incorrect-password" | "invalid-file";

/**
 * 解析产出的待导入树：nodes 按深度优先先序排列（父节点下标恒小于子节点），
 * nodes[0] 表示数据库本身（树的根）；edges 以下标对表达父子关系
 * （source_index 为父、target_index 为子）。
 */
export interface ImportedTree {
  /** 待导入的节点列表（含表示数据库本身的根节点） */
  nodes: ImportedNode[];
  /** 父子边列表（下标引用 nodes） */
  edges: ImportedEdge[];
}

/** installArgon2 是否已注入过 argon2 实现（幂等标记） */
let argon2Installed = false;

/** 树形布局的列间距（相邻两层节点的 x 差） */
const TREE_COLUMN_SPACING = 240;
/** 树形布局的行间距（相邻两行叶子节点的 y 差） */
const TREE_ROW_SPACING = 160;

/**
 * 向 kdbxweb 注入基于 @noble/hashes 的 argon2 实现（KDBX 4 的 KDF 用）：
 * 按回调的 type 参数选择 argon2d/argon2i/argon2id，参数映射
 * t=iterations（迭代次数）、m=memory（KiB）、p=parallelism（并行度）、dkLen=length（输出长度）。
 * 幂等，重复调用无副作用。
 */
export function installArgon2(): void {
  if (argon2Installed) return;
  argon2Installed = true;
  kdbxweb.CryptoEngine.setArgon2Impl(
    async (password, salt, memory, iterations, length, parallelism, type, version) => {
      // kdbxweb 传入的 type 数值即 argon2 变体编号：0 = argon2d、1 = argon2i、2 = argon2id。
      const kind = type as number;
      const derive = kind === 0 ? argon2dAsync : kind === 1 ? argon2iAsync : argon2idAsync;
      const hash = await derive(new Uint8Array(password), new Uint8Array(salt), {
        t: iterations,
        m: memory,
        p: parallelism,
        dkLen: length,
        version,
      });
      return hash.slice().buffer;
    },
  );
}

/**
 * 解析 KeePass 2.0 数据库并产出待导入的树形节点数据。
 *
 * 根 group 产出表示数据库本身的根节点（title=根 group 名，可能为空串，由调用方兜底），
 * 置于 nodes[0]；随后深度优先（先序）遍历：每个 group 先按原序产出其 entry 节点，
 * 再按原序遍历子 group；KeePass 回收站 group 的整个子树跳过（回收站无值时全部导入）。
 * 每产出一个节点同时产出一条其父节点指向它的边。
 * entry 转换规则：UserName 非空（undefined/"" 均视为空）时 title=UserName、
 * sub_title=Title；UserName 为空时 title=Title、sub_title=""。
 * 字段构造顺序固定：密码 → 访问链接 → 备注，为空的（undefined/""）跳过。
 * @param bytes 数据库文件字节
 * @param masterPassword 数据库的 Master Password（不做 trim，KeePass 密码可含首尾空格）
 * @param text 字段名文案集合
 * @returns 待导入树（节点坐标为占位 0，由 layoutImportedTree 写入布局坐标）
 */
export async function parseKeepass2Database(
  bytes: Uint8Array,
  masterPassword: string,
  text: Keepass2FieldText,
): Promise<ImportedTree> {
  installArgon2();
  // Kdbx.load 需要 ArrayBuffer；bytes.buffer 可能大于 bytes 视图（字节偏移非 0 时），
  // slice() 拷贝出恰好等长的独立缓冲再取 buffer 最稳妥。
  const data = bytes.slice().buffer;
  let db: kdbxweb.Kdbx;
  try {
    const credentials = new kdbxweb.Credentials(
      kdbxweb.ProtectedValue.fromString(masterPassword),
    );
    db = await kdbxweb.Kdbx.load(data, credentials);
  } catch (error) {
    // InvalidKey 是密码错误的典型表现（KDBX 3.1 的流起始字节校验与
    // KDBX 4 的头部 HMAC 校验、AES 解密失败都会抛出），其余视为文件无效。
    if (
      error instanceof kdbxweb.KdbxError &&
      error.code === kdbxweb.Consts.ErrorCodes.InvalidKey
    ) {
      throw "incorrect-password";
    }
    throw "invalid-file";
  }
  const tree: ImportedTree = { nodes: [], edges: [] };
  const rootGroup = db.getDefaultGroup();
  // 根 group 产出表示数据库本身的根节点（title 可能为空串，由调用方以文件名兜底）。
  tree.nodes.push({ title: rootGroup.name ?? "", sub_title: "", x: 0, y: 0, fields: [] });
  collectGroup(rootGroup, 0, db.meta.recycleBinUuid, text, tree);
  return tree;
}

/**
 * 就地写入树形布局坐标：x = 深度 × 列间距（父左子右）；叶子节点按深度优先
 * 先序占据连续行槽（y = 行号 × 行间距），父节点纵向居中于其首个与末个子节点之间。
 * 从 nodes[0]（根节点）出发沿 edges 递归，树由解析方构造、必然无环。
 * @param tree 待布局的导入树（nodes[0] 为根节点）
 */
export function layoutImportedTree(tree: ImportedTree): void {
  // 由边列表构建子节点邻接表；边按产出顺序排列，子节点顺序即 KeePass 原序。
  const children: number[][] = tree.nodes.map(() => []);
  for (const edge of tree.edges) {
    children[edge.source_index].push(edge.target_index);
  }
  /** 下一个可用的叶子行号 */
  let nextRow = 0;

  /**
   * 后序递归布局单个节点：先定 x（深度列），叶子直接占行，
   * 内部节点在全部子节点布局完成后取首尾子节点 y 的均值。
   * @param index 节点在 tree.nodes 中的下标
   * @param depth 节点深度（根为 0）
   */
  function layoutNode(index: number, depth: number): void {
    const node = tree.nodes[index];
    node.x = depth * TREE_COLUMN_SPACING;
    const childIndices = children[index];
    if (childIndices.length === 0) {
      node.y = nextRow * TREE_ROW_SPACING;
      nextRow += 1;
      return;
    }
    for (const childIndex of childIndices) {
      layoutNode(childIndex, depth + 1);
    }
    node.y =
      (tree.nodes[childIndices[0]].y + tree.nodes[childIndices[childIndices.length - 1]].y) / 2;
  }

  layoutNode(0, 0);
}

/**
 * 遍历单个 group 并产出其后代节点：先按原序产出其 entry 节点，
 * 再按原序遍历子 group：回收站 group 的整个子树被跳过，
 * 其余子 group 产出节点（title=group 名、sub_title=""、无字段）后递归遍历。
 * 每产出一个节点同时记录一条 parentIndex 指向它的边。
 * @param group 当前待遍历的 group
 * @param parentIndex 该 group 对应节点在 nodes 中的下标
 * @param recycleBinUuid KeePass 回收站 group 的 uuid，undefined 表示数据库无回收站
 * @param text 字段名文案集合
 * @param tree 产出累积的导入树
 */
function collectGroup(
  group: kdbxweb.KdbxGroup,
  parentIndex: number,
  recycleBinUuid: kdbxweb.KdbxUuid | undefined,
  text: Keepass2FieldText,
  tree: ImportedTree,
): void {
  for (const entry of group.entries) {
    tree.edges.push({ source_index: parentIndex, target_index: tree.nodes.length });
    tree.nodes.push(planEntry(entry, text));
  }
  for (const subgroup of group.groups) {
    // 回收站 group 及其全部子孙都被跳过。
    if (recycleBinUuid?.equals(subgroup.uuid)) continue;
    const subgroupIndex = tree.nodes.length;
    tree.edges.push({ source_index: parentIndex, target_index: subgroupIndex });
    tree.nodes.push({ title: subgroup.name ?? "", sub_title: "", x: 0, y: 0, fields: [] });
    collectGroup(subgroup, subgroupIndex, recycleBinUuid, text, tree);
  }
}

/**
 * 将一个 KeePass entry 转换为待导入的节点：UserName 非空时
 * title = UserName、sub_title = Title；UserName 为空（undefined 或空串）时
 * title = Title、sub_title = 空串。字段按固定顺序（密码 → 访问链接 → 备注）收集，
 * 三者中为空的（undefined 或空串）跳过，字段 dictionary_id 恒为 null。
 * @param entry 待转换的 KeePass entry
 * @param text 字段名文案集合
 * @returns 该 entry 的节点数据（坐标为占位 0）
 */
function planEntry(entry: kdbxweb.KdbxEntry, text: Keepass2FieldText): ImportedNode {
  const entryTitle = fieldText(entry, "Title");
  const username = fieldText(entry, "UserName");
  const title = username === "" ? entryTitle : username;
  const subTitle = username === "" ? "" : entryTitle;

  const fields: NodeFieldVO[] = [];
  const password = fieldText(entry, "Password");
  if (password !== "") {
    fields.push({
      name: text.password,
      field_type: "string:password",
      value: password,
      dictionary_id: null,
    });
  }
  const url = fieldText(entry, "URL");
  if (url !== "") {
    fields.push({
      name: text.url,
      field_type: "string:url",
      value: url,
      dictionary_id: null,
    });
  }
  const notes = fieldText(entry, "Notes");
  if (notes !== "") {
    fields.push({
      name: text.notes,
      field_type: "string:multiple-line",
      value: notes,
      dictionary_id: null,
    });
  }

  return { title, sub_title: subTitle, x: 0, y: 0, fields };
}

/**
 * 读取 entry 字段的明文（ProtectedValue 用 getText() 取明文），undefined 视为空串。
 * @param entry 待读取的 KeePass entry
 * @param name KeePass 字段名（如 "Title"、"UserName"）
 * @returns 字段明文，字段不存在时返回空串
 */
function fieldText(entry: kdbxweb.KdbxEntry, name: string): string {
  const value = entry.fields.get(name);
  if (value === undefined) return "";
  return typeof value === "string" ? value : value.getText();
}
