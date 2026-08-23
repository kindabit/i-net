/**
 * Pass 1 · 输入清理：过滤无效边（端点缺失、自环、重复）并构建邻接表。
 *
 * 后续所有 pass 均以本 pass 产出的干净图数据为输入，不再各自做防御。
 */

import type {
  IncomingEdge,
  RadialLayoutEdge,
  RadialLayoutNode,
} from "./types";

/** 清理后的图数据：节点索引、干净边集与邻接表。 */
export interface SanitizedGraph {
  /** 节点 id → 节点输入。 */
  nodeById: Map<string, RadialLayoutNode>;
  /** 有效边列表（自环、端点缺失与重复边已被剔除）。 */
  edges: RadialLayoutEdge[];
  /** 节点 id → 出边目标 id 列表。 */
  outgoing: Map<string, string[]>;
  /** 节点 id → 入边信息列表。 */
  incoming: Map<string, IncomingEdge[]>;
  /** 节点 id → 入度。 */
  indegree: Map<string, number>;
}

/**
 * 清理输入并构建邻接表。
 *
 * @param nodes 节点列表，id 必须唯一。
 * @param edges 边列表；端点不存在、自环或重复的边会被忽略。
 * @returns 干净图数据。
 */
export function sanitizeGraph(
  nodes: RadialLayoutNode[],
  edges: RadialLayoutEdge[],
): SanitizedGraph {
  const nodeById = new Map(nodes.map((node) => [node.id, node]));

  const validEdges: RadialLayoutEdge[] = [];
  const seenEdgeKeys = new Set<string>();
  for (const edge of edges) {
    if (edge.source === edge.target) {
      continue;
    }
    if (!nodeById.has(edge.source) || !nodeById.has(edge.target)) {
      continue;
    }
    const key = `${edge.source}→${edge.target}`;
    if (seenEdgeKeys.has(key)) {
      continue;
    }
    seenEdgeKeys.add(key);
    validEdges.push(edge);
  }

  const outgoing = new Map<string, string[]>();
  const incoming = new Map<string, IncomingEdge[]>();
  const indegree = new Map<string, number>();
  for (const node of nodes) {
    outgoing.set(node.id, []);
    incoming.set(node.id, []);
    indegree.set(node.id, 0);
  }
  for (const edge of validEdges) {
    outgoing.get(edge.source)!.push(edge.target);
    incoming.get(edge.target)!.push({
      parent: edge.source,
      sourcePort: edge.sourcePort,
      targetPort: edge.targetPort,
    });
    indegree.set(edge.target, indegree.get(edge.target)! + 1);
  }

  return { nodeById, edges: validEdges, outgoing, incoming, indegree };
}
