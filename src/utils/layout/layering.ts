/**
 * Pass 2 · 最长路径分层（Kahn 拓扑排序）。
 *
 * layer[v] = max(layer[u] + 1)，保证任意边的 source 层号严格小于 target，
 * 即父节点必然先于子节点放置（布局管线的不变量①）。
 * 拓扑排序无法覆盖的节点（环及其下游）不出现在结果中，
 * 由后续 pass 降级为孤立节点（环防御）。
 */

import type { SanitizedGraph } from "./sanitize";
import { compareId } from "./utils";

/**
 * 计算最长路径分层。
 *
 * @param nodeIds 节点 id 列表。
 * @param graph 干净图数据。
 * @returns 成功分层的节点 id → 层号；环节点不在其中。
 */
export function assignLayers(
  nodeIds: string[],
  graph: SanitizedGraph,
): Map<string, number> {
  const { outgoing, indegree } = graph;
  const layer = new Map<string, number>();
  const remainingIndegree = new Map(indegree);
  const queue = nodeIds
    .filter((id) => indegree.get(id) === 0)
    .sort(compareId);
  while (queue.length > 0) {
    const id = queue.shift()!;
    const currentLayer = layer.get(id) ?? 0;
    layer.set(id, currentLayer);
    for (const next of outgoing.get(id)!) {
      const candidate = currentLayer + 1;
      if (candidate > (layer.get(next) ?? 0)) {
        layer.set(next, candidate);
      }
      const degree = remainingIndegree.get(next)! - 1;
      remainingIndegree.set(next, degree);
      if (degree === 0) {
        queue.push(next);
      }
    }
  }
  return layer;
}
