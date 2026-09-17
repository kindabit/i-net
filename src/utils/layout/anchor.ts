/**
 * 锚点系统：把入边的连接桩信息解析为"子节点应处于父节点哪个方位"的锚向角
 * 与锚距，是分量布局（component-layout.ts）与方向规范化判据
 * （orientation.ts）共用的核心语义。
 *
 * 锚向解析是对称的：sourceHandle（父节点出口侧，最强信号）优先，
 * 其次 targetHandle 的反向。因此把一条边的两端互换后重新解析，
 * 得到的相对方位语义不变——这是方向规范化 pass 的正确性基础。
 */

import {
  HANDLE_ANGLE,
  type IncomingEdge,
  type AutoLayoutConfig,
  type AutoLayoutPoint,
  type Handle,
} from "./types";
import { normalizeAngle } from "./utils";

/**
 * 解析入边的连接桩（sourceHandle 优先，其次 targetHandle 反向）。
 *
 * @param edge 入边信息（父节点 id 与可选连接桩）。
 * @returns 连接桩；无连接桩时返回 undefined。
 */
export function handleOf(
  edge: IncomingEdge,
): Handle | undefined {
  if (edge.sourceHandle !== undefined) {
    return edge.sourceHandle;
  }
  switch (edge.targetHandle) {
    case "top":
      return "bottom";
    case "bottom":
      return "top";
    case "left":
      return "right";
    case "right":
      return "left";
    default:
      return undefined;
  }
}

/**
 * 计算节点的流向方向：自身位置 − 各父节点均值位置的方向。
 *
 * @param id 节点 id。
 * @param positions 已放置节点 id → 局部中心坐标。
 * @param incoming 节点 id → 入边信息列表。
 * @returns 流向方向角（弧度）；无父节点、父节点未放置或向量抵消时返回 undefined。
 */
export function flowDirectionOf(
  id: string,
  positions: Map<string, AutoLayoutPoint>,
  incoming: Map<string, IncomingEdge[]>,
): number | undefined {
  const edges = incoming.get(id)!;
  const pos = positions.get(id);
  if (edges.length === 0 || pos === undefined) {
    return undefined;
  }
  // 排序后求和：消除入边输入顺序带来的浮点舍入差异，保证完全确定性。
  const parentPoints = edges
    .map((edge) => positions.get(edge.parent))
    .filter((point): point is AutoLayoutPoint => point !== undefined)
    .sort((a, b) => a.cx - b.cx || a.cy - b.cy);
  if (parentPoints.length === 0) {
    return undefined;
  }
  let sumX = 0;
  let sumY = 0;
  for (const point of parentPoints) {
    sumX += point.cx;
    sumY += point.cy;
  }
  const dx = pos.cx - sumX / parentPoints.length;
  const dy = pos.cy - sumY / parentPoints.length;
  if (dx === 0 && dy === 0) {
    return undefined;
  }
  return Math.atan2(dy, dx);
}

/** 锚向解析结果：方向角 + 锚点距离。 */
export interface AnchorOffset {
  /** 锚向角（弧度，归一化到 [0, 2π)）。 */
  angle: number;
  /** 锚点与父节点的中心距（px）。 */
  distance: number;
}

/**
 * 计算单条入边的锚向与锚距。
 *
 * 锚向优先级：sourceHandle 方向 → targetHandle 反向 → 父节点流向方向 →
 * 无连接桩子代组均布（角度 = 2π×组内序号/组大小，组过密时锚距突破
 * ringSpacing 扩大至 span/(2·sin(π/组大小)) 保证弦长不重叠；
 * 均布方向与父节点有连接桩子代的方向重合时整体偏移半格 π/组大小）。
 *
 * @param edge 入边信息（父节点 id 与可选连接桩）。
 * @param childId 本节点 id（用于定位均布组内序号）。
 * @param positions 已放置节点 id → 局部中心坐标（父节点必然已放置）。
 * @param incoming 节点 id → 入边信息列表。
 * @param spreadGroups 父节点 id → 本层无连接桩子代 id 列表（已按 id 排序）。
 * @param handleAnglesByParent 父节点 id → 其有连接桩子边的锚向角列表。
 * @param span 节点间距（最大节点尺寸 + nodeMargin，px）。
 * @param config 布局参数。
 * @returns 锚向角（弧度，[0, 2π)）与锚距（px）。
 */
export function anchorOffsetOf(
  edge: IncomingEdge,
  childId: string,
  positions: Map<string, AutoLayoutPoint>,
  incoming: Map<string, IncomingEdge[]>,
  spreadGroups: Map<string, string[]>,
  handleAnglesByParent: Map<string, number[]>,
  span: number,
  config: AutoLayoutConfig,
): AnchorOffset {
  const handle = handleOf(edge);
  if (handle !== undefined) {
    return { angle: HANDLE_ANGLE[handle], distance: config.ringSpacing };
  }
  const flow = flowDirectionOf(edge.parent, positions, incoming);
  if (flow !== undefined) {
    return { angle: normalizeAngle(flow), distance: config.ringSpacing };
  }
  // 孤立根节点的无连接桩子代：绕父节点均布。
  const group = spreadGroups.get(edge.parent) ?? [childId];
  const count = group.length;
  const index = group.indexOf(childId);
  const fullCircle = 2 * Math.PI;
  // 均布方向与父节点有连接桩子代的方向重合时，整体偏移半格。
  let phase = 0;
  const handleAngles = handleAnglesByParent.get(edge.parent) ?? [];
  for (const handleAngle of handleAngles) {
    for (let i = 0; i < count; i++) {
      const candidate = (i * fullCircle) / count;
      if (Math.abs(normalizeAngle(candidate - handleAngle)) < 1e-9) {
        phase = Math.PI / count;
      }
    }
  }
  // 锚距按弦长（而非弧长）保证相邻子代不重叠：2r·sin(π/n) ≥ span。
  const distance =
    count >= 2
      ? Math.max(config.ringSpacing, span / (2 * Math.sin(Math.PI / count)))
      : config.ringSpacing;
  return {
    angle: normalizeAngle(phase + (Math.max(index, 0) * fullCircle) / count),
    distance,
  };
}
