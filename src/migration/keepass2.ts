/**
 * KeePass 2.0 数据库（.kdbx）前端解析模块。
 *
 * 职责：接收文件字节与 Master Password，在前端完成解密与解析（kdbxweb，KDBX 3.1 与 4.0），
 * 按导入方式产出两种待导入数据：
 * - 单画布（parseKeepass2Database）：深度优先遍历 group/entry 树，产出树形节点数据
 *   （ImportedTree：节点 + 父子边，坐标为占位 0，由 layoutImportedTree 写入树形布局坐标），
 *   根节点表示数据库本身（对应 KeePass 根 group），父节点在左、子节点在右；
 * - 多画布（parseKeepass2DatabaseCanvases）：每个 group 产出一张画布（含根 group），
 *   group 的 entry 产出画布内的数据节点、子 group 产出子画布与父画布内的画布数据节点引用，
 *   坐标为占位 0，由 layoutImportedCanvases 写入两级布局坐标（画布宇宙中的树形布局
 *   与画布内的矩阵布局）。
 * 不负责文件读取（走后端字节通道）与写库（走后端聚合导入接口）。
 * 字段名文案由调用方按当前语言传入，本模块不做国际化。
 *
 * 错误约定：parseKeepass2Database 与 parseKeepass2DatabaseCanvases 失败时直接 throw
 * Keepass2ParseError 字符串值（非 Error 对象），调用方以 catch 到的值判别失败原因。
 */
import * as kdbxweb from "kdbxweb";
import { argon2dAsync, argon2iAsync, argon2idAsync } from "@noble/hashes/argon2.js";
import type { ImportedCanvasVO, ImportedEdgeVO, ImportedNodeVO, NodeFieldVO } from "@/api-types";

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
  nodes: ImportedNodeVO[];
  /** 父子边列表（下标引用 nodes） */
  edges: ImportedEdgeVO[];
}

/** installArgon2 是否已注入过 argon2 实现（幂等标记） */
let argon2Installed = false;

/** 树形布局的列间距（相邻两层节点的 x 差） */
const TREE_COLUMN_SPACING = 240;
/** 树形布局的行间距（相邻两行叶子节点的 y 差） */
const TREE_ROW_SPACING = 160;
/** 宇宙树形布局的列间距（相邻两层画布的 x 差，父左子右） */
const UNIVERSE_COLUMN_SPACING = 240;
/** 宇宙树形布局的行间距（相邻两行叶子画布的 y 差） */
const UNIVERSE_ROW_SPACING = 160;
/** 宇宙空闲点搜索的最小间距（候选点与全部现存画布中心的欧氏距离下限，与后端环形布局一致） */
const UNIVERSE_SEARCH_MIN_DISTANCE = 200;
/** 画布内矩阵布局的列间距（相邻两列节点的 x 差） */
const MATRIX_COLUMN_SPACING = 240;
/** 画布内矩阵布局的行间距（相邻两行节点的 y 差） */
const MATRIX_ROW_SPACING = 160;

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
 * 解密并载入 KeePass 2.0 数据库（KDBX 3.1 与 4.0）。
 * Kdbx.load 需要 ArrayBuffer；bytes.buffer 可能大于 bytes 视图（字节偏移非 0 时），
 * slice() 拷贝出恰好等长的独立缓冲再取 buffer 最稳妥。
 * @param bytes 数据库文件字节
 * @param masterPassword 数据库的 Master Password（不做 trim，KeePass 密码可含首尾空格）
 * @returns 解密后的数据库对象
 * @throws Keepass2ParseError 字符串值（非 Error 对象）：InvalidKey 是密码错误的典型表现
 * （KDBX 3.1 的流起始字节校验与 KDBX 4 的头部 HMAC 校验、AES 解密失败都会抛出），
 * 其余视为文件无效。
 */
async function loadKdbx(bytes: Uint8Array, masterPassword: string): Promise<kdbxweb.Kdbx> {
  installArgon2();
  const data = bytes.slice().buffer;
  try {
    const credentials = new kdbxweb.Credentials(
      kdbxweb.ProtectedValue.fromString(masterPassword),
    );
    return await kdbxweb.Kdbx.load(data, credentials);
  } catch (error) {
    if (
      error instanceof kdbxweb.KdbxError &&
      error.code === kdbxweb.Consts.ErrorCodes.InvalidKey
    ) {
      throw "incorrect-password";
    }
    throw "invalid-file";
  }
}

/**
 * 解析 KeePass 2.0 数据库并产出待导入的树形节点数据。
 *
 * 根 group 产出表示数据库本身的根节点（title=根 group 名，可能为空串，由调用方兜底），
 * 置于 nodes[0]；随后深度优先（先序）遍历：每个 group 先按原序产出其 entry 节点，
 * 再按原序遍历子 group；KeePass 回收站 group 的整个子树跳过（回收站无值时全部导入）。
 * 每产出一个节点同时产出一条其父节点指向它的边。
 * entry 转换规则：UserName 非空（undefined/"" 均视为空）时 title=UserName、
 * subtitle=Title；UserName 为空时 title=Title、subtitle=""。
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
  const db = await loadKdbx(bytes, masterPassword);
  const tree: ImportedTree = { nodes: [], edges: [] };
  const rootGroup = db.getDefaultGroup();
  // 根 group 产出表示数据库本身的根节点（title 可能为空串，由调用方以文件名兜底）。
  tree.nodes.push({ title: rootGroup.name ?? "", subtitle: "", x: 0, y: 0, fields: [] });
  collectGroup(rootGroup, 0, db.meta.recycleBinUuid, text, tree);
  return tree;
}

/**
 * 解析 KeePass 2.0 数据库并按多画布导入方式产出待导入的画布列表。
 *
 * 每个 group 产出一张画布（含根 group）：根 group 画布置于 canvases[0]
 * （name=根 group 名，可能为空串，由调用方以文件名兜底；parent_index=null，
 * 由后端挂到根画布）；随后深度优先（先序）遍历：每个 group 先按原序产出其 entry
 * 数据节点，再按原序遍历子 group——KeePass 回收站 group 的整个子树跳过
 * （回收站无值时全部导入），其余子 group 各产出一张画布（name=group 名，
 * 可能为空串，由调用方兜底）并在父画布的 canvas_nodes 中记录对它的引用。
 * 列表按深度优先先序排列，父画布下标恒小于子画布。
 * entry 转换规则与字段构造顺序同 parseKeepass2Database。
 * @param bytes 数据库文件字节
 * @param masterPassword 数据库的 Master Password（不做 trim，KeePass 密码可含首尾空格）
 * @param text 字段名文案集合
 * @returns 待导入画布列表（坐标为占位 0，由 layoutImportedCanvases 写入布局坐标）
 */
export async function parseKeepass2DatabaseCanvases(
  bytes: Uint8Array,
  masterPassword: string,
  text: Keepass2FieldText,
): Promise<ImportedCanvasVO[]> {
  const db = await loadKdbx(bytes, masterPassword);
  const canvases: ImportedCanvasVO[] = [];
  const rootGroup = db.getDefaultGroup();
  // 根 group 产出根 group 画布（name 可能为空串，由调用方以文件名兜底）。
  canvases.push({
    name: rootGroup.name ?? "",
    x: 0,
    y: 0,
    parent_index: null,
    nodes: [],
    canvas_nodes: [],
  });
  collectGroupCanvases(rootGroup, 0, db.meta.recycleBinUuid, text, canvases);
  return canvases;
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
 * 布局多画布导入的画布集合（就地写入坐标），含两级布局：
 * 1) 宇宙树形布局：以根画布坐标为圆心环形搜索空闲点作为整棵画布树的根位置
 *    （canvases[0] 的坐标），其余画布沿 parent_index 层级展开——x = 树根 x + 深度 × 列间距
 *    （父左子右），叶子画布按深度优先先序占据连续行槽，父画布纵向居中于其首个与
 *    末个子画布之间；
 * 2) 画布内矩阵布局：每个画布的数据节点与画布数据节点（数据节点在前、画布数据节点在后，
 *    与解析产出顺序一致）按 ceil(sqrt(n)) 列的网格平铺，坐标原点为 (0, 0)。
 * @param canvases 待布局的导入画布列表（canvases[0] 为根 group 画布，树由解析方构造、必然无环）
 * @param rootCanvas 根画布（新画布树的挂点）坐标
 * @param existingCanvases 现存全部画布的坐标（含根画布与已逻辑删除的画布，用于空闲点搜索）
 */
export function layoutImportedCanvases(
  canvases: ImportedCanvasVO[],
  rootCanvas: { x: number; y: number },
  existingCanvases: { x: number; y: number }[],
): void {
  const origin = findFreeUniversePosition(existingCanvases, rootCanvas);
  // 由 parent_index 构建子画布邻接表；列表按深度优先先序排列，子画布顺序即 KeePass 原序。
  const children: number[][] = canvases.map(() => []);
  for (let index = 1; index < canvases.length; index++) {
    const parentIndex = canvases[index].parent_index;
    if (parentIndex !== null) {
      children[parentIndex].push(index);
    }
  }
  /** 下一个可用的叶子行号 */
  let nextRow = 0;

  /**
   * 后序递归布局单张画布：先定 x（树根 x + 深度列），叶子直接占行，
   * 内部画布在全部子画布布局完成后取首尾子画布 y 的均值。
   * @param index 画布在 canvases 中的下标
   * @param depth 画布深度（根 group 画布为 0）
   */
  function layoutCanvas(index: number, depth: number): void {
    const canvas = canvases[index];
    canvas.x = origin.x + depth * UNIVERSE_COLUMN_SPACING;
    const childIndices = children[index];
    if (childIndices.length === 0) {
      canvas.y = origin.y + nextRow * UNIVERSE_ROW_SPACING;
      nextRow += 1;
      return;
    }
    for (const childIndex of childIndices) {
      layoutCanvas(childIndex, depth + 1);
    }
    canvas.y =
      (canvases[childIndices[0]].y + canvases[childIndices[childIndices.length - 1]].y) / 2;
  }

  layoutCanvas(0, 0);

  // 画布内矩阵布局：数据节点与画布数据节点合成一个序列（数据节点在前），
  // 按 ceil(sqrt(n)) 列的网格平铺。
  for (const canvas of canvases) {
    const count = canvas.nodes.length + canvas.canvas_nodes.length;
    if (count === 0) continue;
    const columns = Math.ceil(Math.sqrt(count));
    canvas.nodes.forEach((node, index) => {
      node.x = (index % columns) * MATRIX_COLUMN_SPACING;
      node.y = Math.floor(index / columns) * MATRIX_ROW_SPACING;
    });
    canvas.canvas_nodes.forEach((canvasNode, index) => {
      const sequence = canvas.nodes.length + index;
      canvasNode.x = (sequence % columns) * MATRIX_COLUMN_SPACING;
      canvasNode.y = Math.floor(sequence / columns) * MATRIX_ROW_SPACING;
    });
  }
}

/**
 * 在画布宇宙中为新画布树的根搜索空闲位置：以 rootCanvas 为圆心逐圈向外搜索，
 * 返回第一个与所有现存画布中心的欧氏距离都不小于 200 的候选点（与后端环形布局
 * 同一算法：半径从 240 起每圈增加 120，每圈取 8 个等分角度（起点角度为 0），
 * 最多搜索 32 圈；都找不到时兜底返回圆心向右偏移 240 的位置）。
 * @param existingCanvases 现存全部画布的坐标
 * @param rootCanvas 搜索圆心（根画布坐标）
 * @returns 空闲位置坐标
 */
function findFreeUniversePosition(
  existingCanvases: { x: number; y: number }[],
  rootCanvas: { x: number; y: number },
): { x: number; y: number } {
  const isFree = (x: number, y: number) =>
    existingCanvases.every((canvas) => {
      const dx = canvas.x - x;
      const dy = canvas.y - y;
      return Math.sqrt(dx * dx + dy * dy) >= UNIVERSE_SEARCH_MIN_DISTANCE;
    });
  for (let ring = 0; ring < 32; ring++) {
    const radius = 240 + ring * 120;
    for (let index = 0; index < 8; index++) {
      const angle = (index * 2 * Math.PI) / 8;
      const x = rootCanvas.x + radius * Math.cos(angle);
      const y = rootCanvas.y + radius * Math.sin(angle);
      if (isFree(x, y)) {
        return { x, y };
      }
    }
  }
  return { x: rootCanvas.x + 240, y: rootCanvas.y };
}

/**
 * 遍历单个 group 并产出其后代节点：先按原序产出其 entry 节点，
 * 再按原序遍历子 group：回收站 group 的整个子树被跳过，
 * 其余子 group 产出节点（title=group 名、subtitle=""、无字段）后递归遍历。
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
    tree.nodes.push({ title: subgroup.name ?? "", subtitle: "", x: 0, y: 0, fields: [] });
    collectGroup(subgroup, subgroupIndex, recycleBinUuid, text, tree);
  }
}

/**
 * 多画布路线的 group 遍历：向当前 group 对应的画布填充内容并递归产出子 group 画布。
 * 先按原序产出其 entry 数据节点，再按原序遍历子 group：回收站 group 的整个子树被跳过，
 * 其余子 group 各产出一张画布（name=group 名，可能为空串，由调用方兜底）并在当前画布的
 * canvas_nodes 中记录对它的引用（ref_index 指向新画布）。新画布下标在递归前确定，
 * 父画布下标恒小于子画布。
 * @param group 当前待遍历的 group
 * @param canvasIndex 该 group 对应画布在 canvases 中的下标
 * @param recycleBinUuid KeePass 回收站 group 的 uuid，undefined 表示数据库无回收站
 * @param text 字段名文案集合
 * @param canvases 产出累积的导入画布列表
 */
function collectGroupCanvases(
  group: kdbxweb.KdbxGroup,
  canvasIndex: number,
  recycleBinUuid: kdbxweb.KdbxUuid | undefined,
  text: Keepass2FieldText,
  canvases: ImportedCanvasVO[],
): void {
  const canvas = canvases[canvasIndex];
  for (const entry of group.entries) {
    canvas.nodes.push(planEntry(entry, text));
  }
  for (const subgroup of group.groups) {
    // 回收站 group 及其全部子孙都被跳过。
    if (recycleBinUuid?.equals(subgroup.uuid)) continue;
    const subgroupIndex = canvases.length;
    canvas.canvas_nodes.push({ x: 0, y: 0, ref_index: subgroupIndex });
    canvases.push({
      name: subgroup.name ?? "",
      x: 0,
      y: 0,
      parent_index: canvasIndex,
      nodes: [],
      canvas_nodes: [],
    });
    collectGroupCanvases(subgroup, subgroupIndex, recycleBinUuid, text, canvases);
  }
}

/**
 * 将一个 KeePass entry 转换为待导入的节点：UserName 非空时
 * title = UserName、subtitle = Title；UserName 为空（undefined 或空串）时
 * title = Title、subtitle = 空串。字段按固定顺序（密码 → 访问链接 → 备注）收集，
 * 三者中为空的（undefined 或空串）跳过，字段 dictionary_id 恒为 null。
 * @param entry 待转换的 KeePass entry
 * @param text 字段名文案集合
 * @returns 该 entry 的节点数据（坐标为占位 0）
 */
function planEntry(entry: kdbxweb.KdbxEntry, text: Keepass2FieldText): ImportedNodeVO {
  const entryTitle = fieldText(entry, "Title");
  const username = fieldText(entry, "UserName");
  const title = username === "" ? entryTitle : username;
  const subtitle = username === "" ? "" : entryTitle;

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

  return { title, subtitle, x: 0, y: 0, fields };
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
