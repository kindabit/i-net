/**
 * 布局算法公共类型、默认参数与共享常量（纯数据定义，不含逻辑）。
 */

/** 布局输入：一个节点的 id 与渲染尺寸。 */
export interface RadialLayoutNode {
  /** 节点 id，同一输入内必须唯一。 */
  id: string;
  /** 节点渲染宽度（px）。 */
  width: number;
  /** 节点渲染高度（px）。 */
  height: number;
}

/** 端口方向：边在节点上的进出方位（画布坐标系，+x 向右、+y 向下）。 */
export type RadialPortDirection = "top" | "bottom" | "left" | "right";

/**
 * 布局输入：一条有向边。
 *
 * 注意：边的原始方向不直接决定层级——方向规范化 pass（orientation.ts）
 * 可能对"汇聚星型"分量整体反转；最终以规范方向为准（source 为父节点，
 * 先于 target 放置）。
 */
export interface RadialLayoutEdge {
  /** 源节点 id。 */
  source: string;
  /** 目标节点 id。 */
  target: string;
  /** 可选：边离开源节点的端口方向（如 right 表示从源节点右侧引出）。 */
  sourcePort?: RadialPortDirection;
  /** 可选：边进入目标节点的端口方向（如 left 表示从目标节点左侧进入）。 */
  targetPort?: RadialPortDirection;
}

/** 布局参数。 */
export interface RadialLayoutConfig {
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
export interface RadialLayoutPoint {
  cx: number;
  cy: number;
}

/** 默认布局参数，适用于普通画布（数据节点固定尺寸，见 node-size.ts）与画布宇宙。 */
export const DEFAULT_RADIAL_LAYOUT_CONFIG: RadialLayoutConfig = {
  ringSpacing: 300,
  nodeMargin: 40,
  componentGap: 200,
  isolatedCell: 240,
};

/** 端口方向 → 方向角（弧度，画布坐标系：+x 向右、+y 向下）。 */
export const PORT_ANGLE: Record<RadialPortDirection, number> = {
  right: 0,
  bottom: Math.PI / 2,
  left: Math.PI,
  top: -Math.PI / 2,
};

/** 一条入边在布局中需要的信息：父节点 id 与可选端口方向。 */
export interface IncomingEdge {
  /** 父节点 id。 */
  parent: string;
  /** 可选：边离开父节点的端口方向。 */
  sourcePort?: RadialPortDirection;
  /** 可选：边进入本节点的端口方向。 */
  targetPort?: RadialPortDirection;
}
