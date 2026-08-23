/**
 * Pass 3 · 连通分量划分：并查集按边（无视方向）把节点聚合为连通分量。
 *
 * 方向规范化（orientation.ts）不改变连通性，故本 pass 可在其之前执行，
 * 规范化前后复用同一划分结果。
 */

import type { RadialLayoutEdge, RadialLayoutNode } from "./types";
import { UnionFind } from "./utils";

/**
 * 划分连通分量。
 *
 * @param nodes 节点列表。
 * @param edges 有效边列表。
 * @returns 分量根 id → 分量内节点 id 列表（含单节点"分量"，即孤立节点）。
 */
export function groupComponents(
  nodes: RadialLayoutNode[],
  edges: RadialLayoutEdge[],
): Map<string, string[]> {
  const unionFind = new UnionFind();
  for (const edge of edges) {
    unionFind.union(edge.source, edge.target);
  }
  const componentIds = new Map<string, string[]>();
  for (const node of nodes) {
    const root = unionFind.find(node.id);
    const list = componentIds.get(root) ?? [];
    list.push(node.id);
    componentIds.set(root, list);
  }
  return componentIds;
}
