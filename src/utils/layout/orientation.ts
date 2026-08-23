/**
 * Pass 4 · 边方向规范化：识别"汇聚星型"连通分量并反转其全部边，
 * 使视觉枢纽成为分层根，保证布局对边方向翻转对称。
 *
 * 【动机】分层与根行由边方向决定（source 为父、先放置）。当多个纯源
 * 从不同端口方位汇聚到少数纯汇时（如十字：N bottom→C top、W right→C left、
 * E left→C right、S top→C bottom），端口语义要求汇点同时位于各源的多个
 * 不同方位，一维分层 + 水平根行 + 锚点均值无法满足，汇点最终被矛盾
 * 约束挤到角落（一行节点的斜对角）。反转该分量全部边后，汇点成为
 * 分层根（第 0 层），各源绕其四方位锚定，得到对称的十字布局；
 * 而反向输入（发散型，纯源 < 纯汇）不触发反转——两种边方向因此
 * 收敛到同一布局。
 *
 * 【反转判据】（分量级，全部满足才反转）：
 * 1. 分量内全部节点可拓扑分层（无环；环分量交给孤立降级路径，
 *    保持既有环防御行为不被反转扰动）；
 * 2. 纯源数 > 纯汇数（汇聚型；发散型、链型、菱形等纯源 ≤ 纯汇的
 *    结构保持原方向）；
 * 3. 纯源出边的锚向方位 ≥ 2 种（方位发散）。全部一致时（如双 bottom
 *    汇入表示"源在上、汇在下"的明确一致语义）不反转，保持
 *    "根行 + 均值"的现有正确行为。
 *
 * 【正确性】反转操作交换 source/target 与 sourcePort/targetPort；
 * 锚向解析（见 anchor.ts）本身对称——同一条物理连接无论哪个节点
 * 作为父节点，解析出的相对方位语义一致，故反转不改变端口语义。
 */

import { portDirectionOf } from "./anchor";
import type { SanitizedGraph } from "./sanitize";
import type { RadialLayoutEdge, RadialPortDirection } from "./types";

/**
 * 判定一个连通分量是否需要反转边方向。
 *
 * @param ids 分量内节点 id 列表。
 * @param graph 干净图数据（度数按原始方向计算）。
 * @param layer 原始方向的分层结果（用于环检测）。
 * @returns 是否反转。
 */
function shouldReverseComponent(
  ids: string[],
  graph: SanitizedGraph,
  layer: Map<string, number>,
): boolean {
  // 环分量（存在拓扑排序无法覆盖的节点）不参与反转。
  if (ids.some((id) => !layer.has(id))) {
    return false;
  }
  let pureSourceCount = 0;
  let pureSinkCount = 0;
  for (const id of ids) {
    const hasIncoming = graph.incoming.get(id)!.length > 0;
    const hasOutgoing = graph.outgoing.get(id)!.length > 0;
    if (hasIncoming === hasOutgoing) {
      // 中间节点（有入有出）或无边节点（不会出现在多节点分量中）。
      continue;
    }
    if (hasOutgoing) {
      pureSourceCount++;
    } else {
      pureSinkCount++;
    }
  }
  if (pureSourceCount <= pureSinkCount) {
    return false;
  }
  // 纯源出边的锚向方位集合：≥ 2 种视为方位发散（汇聚中心应成为视觉根）。
  const pureSourceIds = new Set(
    ids.filter((id) => graph.incoming.get(id)!.length === 0),
  );
  const directions = new Set<RadialPortDirection>();
  for (const edge of graph.edges) {
    if (!pureSourceIds.has(edge.source)) {
      continue;
    }
    const direction = portDirectionOf({
      parent: edge.source,
      sourcePort: edge.sourcePort,
      targetPort: edge.targetPort,
    });
    if (direction !== undefined) {
      directions.add(direction);
    }
  }
  return directions.size >= 2;
}

/**
 * 对汇聚星型分量执行边方向规范化。
 *
 * @param graph 干净图数据。
 * @param layer 原始方向的分层结果（环检测依据）。
 * @param componentIds 连通分量划分结果。
 * @returns 规范方向后的新边集；若没有任何分量需要反转则返回 null
 *   （调用方可跳过重建邻接表与重新分层）。
 */
export function normalizeOrientation(
  graph: SanitizedGraph,
  layer: Map<string, number>,
  componentIds: Map<string, string[]>,
): RadialLayoutEdge[] | null {
  const reverseNodeIds = new Set<string>();
  for (const ids of componentIds.values()) {
    if (ids.length < 2) {
      continue;
    }
    if (shouldReverseComponent(ids, graph, layer)) {
      for (const id of ids) {
        reverseNodeIds.add(id);
      }
    }
  }
  if (reverseNodeIds.size === 0) {
    return null;
  }
  // 边的两端必属同一分量，检查 source 一端即可判断该边是否在反转分量内。
  return graph.edges.map((edge) => {
    if (!reverseNodeIds.has(edge.source)) {
      return edge;
    }
    return {
      source: edge.target,
      target: edge.source,
      sourcePort: edge.targetPort,
      targetPort: edge.sourcePort,
    };
  });
}
