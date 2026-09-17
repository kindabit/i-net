/**
 * 布局算法公共类型、默认参数与共享常量（纯数据定义，不含逻辑）。
 */

import type { Handle } from "@/api-types";

/** 连接桩（上下左右各一个），定义见 @/api-types。 */
export type { Handle };

/** 布局输入：一个节点的 id 与渲染尺寸。 */
export interface AutoLayoutNode {
  /** 节点 id，同一输入内必须唯一。 */
  id: string;
  /** 节点渲染宽度（px）。 */
  width: number;
  /** 节点渲染高度（px）。 */
  height: number;
}

/**
 * 布局输入：一条有向边。
 *
 * 注意：边的原始方向不直接决定层级——方向规范化 pass（orientation.ts）
 * 可能对"汇聚星型"分量整体反转；最终以规范方向为准（source 为父节点，
 * 先于 target 放置）。
 */
export interface AutoLayoutEdge {
  /** 源节点 id。 */
  source: string;
  /** 目标节点 id。 */
  target: string;
  /** 可选：边离开源节点的连接桩（如 right 表示从源节点右侧引出）。 */
  sourceHandle?: Handle;
  /** 可选：边进入目标节点的连接桩（如 left 表示从目标节点左侧进入）。 */
  targetHandle?: Handle;
}

/** 布局参数。 */
export interface AutoLayoutConfig {
  /** 父子节点的基础间距（中心距，px）。 */
  ringSpacing: number;
  /** 相邻节点的最小间距（px），用于兄弟排距、根行距与碰撞边距。 */
  nodeMargin: number;
  /** 不同连通分量包围圆之间的间距（px）。 */
  componentGap: number;
  /** 孤立节点网格的单元格边长（px）。 */
  isolatedCell: number;
}

/** 布局输出：节点中心坐标。 */
export interface AutoLayoutPoint {
  cx: number;
  cy: number;
}

/** 默认布局参数，适用于普通画布（数据节点固定尺寸，见 node-size.ts）与画布宇宙。 */
export const DEFAULT_AUTO_LAYOUT_CONFIG: AutoLayoutConfig = {
  ringSpacing: 300,
  nodeMargin: 40,
  componentGap: 200,
  isolatedCell: 240,
};

/** 连接桩 → 方向角（弧度，画布坐标系：+x 向右、+y 向下）。 */
export const HANDLE_ANGLE: Record<Handle, number> = {
  right: 0,
  bottom: Math.PI / 2,
  left: Math.PI,
  top: -Math.PI / 2,
};

/** 一条入边在布局中需要的信息：父节点 id 与可选连接桩。 */
export interface IncomingEdge {
  /** 父节点 id。 */
  parent: string;
  /** 可选：边离开父节点的连接桩。 */
  sourceHandle?: Handle;
  /** 可选：边进入本节点的连接桩。 */
  targetHandle?: Handle;
}
