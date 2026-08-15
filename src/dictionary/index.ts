/**
 * 字典模块：字典树数据结构与全局状态。
 * 全局状态为前端字典数据的唯一事实来源，打开数据库时初始化。
 */
import { computed, shallowRef } from "vue";
import { userDatabaseDictionaryList, userDatabaseDictionarySet } from "@/api";
import type { Dictionary } from "@/api-types";

/** 字典树节点。 */
export interface DictionaryTreeNode {
  entry: Dictionary;
  children: DictionaryTreeNode[];
}

/**
 * 将字典条目列表构建为树形森林，按 parent_id 组织。
 * 同级节点按 entry.order 升序排列；parent_id 指向不存在条目的节点视为根节点。
 * @param entries 字典条目列表
 * @returns 字典树根节点列表
 */
function buildTree(entries: Dictionary[]): DictionaryTreeNode[] {
  const map = new Map<string, DictionaryTreeNode>();
  const roots: DictionaryTreeNode[] = [];

  for (const entry of entries) {
    map.set(entry.id, { entry, children: [] });
  }

  for (const entry of entries) {
    const node = map.get(entry.id)!;
    if (entry.parent_id !== null && map.has(entry.parent_id)) {
      map.get(entry.parent_id)!.children.push(node);
    } else {
      roots.push(node);
    }
  }

  const sortByOrder = (a: DictionaryTreeNode, b: DictionaryTreeNode) =>
    a.entry.order - b.entry.order;

  for (const node of map.values()) {
    node.children.sort(sortByOrder);
  }
  roots.sort(sortByOrder);

  return roots;
}

/**
 * 将字典树拍平为条目列表（先序遍历，order 取同级下标）。
 * @param nodes 字典树节点列表
 * @param parentId 父节点 id，根节点为 null
 * @returns 拍平后的字典条目列表
 */
function flattenTreeNodes(
  nodes: DictionaryTreeNode[],
  parentId: string | null,
): Dictionary[] {
  const result: Dictionary[] = [];
  for (let i = 0; i < nodes.length; i++) {
    result.push({
      id: nodes[i].entry.id,
      parent_id: parentId,
      value: nodes[i].entry.value,
      order: i,
    });
    result.push(...flattenTreeNodes(nodes[i].children, nodes[i].entry.id));
  }
  return result;
}

/** 全量字典树状态。 */
const dictionaryTree = shallowRef<DictionaryTreeNode[]>([]);

/** id→节点索引，不导出，使用 shallowRef 使依赖它的 computed 可追踪。 */
const nodeIndex = shallowRef<Map<string, DictionaryTreeNode>>(new Map());

/**
 * 递归剪除所有叶子，保留有子节点的节点（entry 引用可复用，children 为剪枝后的新数组）。
 * @param nodes 字典树节点列表
 * @returns 剪枝后的节点列表
 */
function pruneLeaves(nodes: DictionaryTreeNode[]): DictionaryTreeNode[] {
  const result: DictionaryTreeNode[] = [];
  for (const node of nodes) {
    const prunedChildren = pruneLeaves(node.children);
    if (node.children.length > 0) {
      result.push({ entry: node.entry, children: prunedChildren });
    }
  }
  return result;
}

/**
 * 遍历树中所有节点。
 * @param nodes 字典树节点列表
 * @param fn 对每个节点执行的回调
 */
function iterateTree(
  nodes: DictionaryTreeNode[],
  fn: (node: DictionaryTreeNode) => void,
) {
  for (const node of nodes) {
    fn(node);
    iterateTree(node.children, fn);
  }
}

/** 剪枝后的字典树（移除所有叶子，仅保留有子节点的节点）。 */
export const prunedDictionaryTree = computed(() =>
  pruneLeaves(dictionaryTree.value),
);

/**
 * 拉取全量字典数据并构建字典树与节点索引。
 * 不自行 catch 错误，由调用方处理。
 */
export async function loadDictionary(): Promise<void> {
  const entries = await userDatabaseDictionaryList();
  const tree = buildTree(entries);
  const map = new Map<string, DictionaryTreeNode>();
  iterateTree(tree, (node) => map.set(node.entry.id, node));
  nodeIndex.value = map;
  dictionaryTree.value = tree;
}

/**
 * 保存字典树：拍平后全量写库，再重新拉取并重置全局状态。
 * 不自行 catch 错误，由调用方处理。
 * @param forest 校验通过的字典树
 */
export async function saveDictionaryTree(
  forest: DictionaryTreeNode[],
): Promise<void> {
  await userDatabaseDictionarySet(flattenTreeNodes(forest, null));
  await loadDictionary();
}

/** 重置字典树和节点索引为空。 */
export function clearDictionary(): void {
  dictionaryTree.value = [];
  nodeIndex.value = new Map();
}

/**
 * 深拷贝当前字典树。
 * @returns 字典树的深拷贝
 */
export function cloneDictionaryTree(): DictionaryTreeNode[] {
  return structuredClone(dictionaryTree.value);
}

/**
 * 获取指定节点 id 的直接子节点的 value 列表。
 * @param id 父节点 id
 * @returns 子节点 value 列表，节点不存在或没有子节点时返回空数组
 */
export function getDictionaryDirectChildren(id: string): string[] {
  return (
    nodeIndex.value.get(id)?.children.map((c) => c.entry.value) ?? []
  );
}
