/**
 * 后端实体（api-types）与 vue-flow 视图模型（VFNode/VFEdge）之间的转换。
 * 视图层与后端实体之间的结构换算统一收敛于此模块，组件内禁止内联构造。
 */
import { MarkerType } from "@vue-flow/core";
import type { Node as VFNode, Edge as VFEdge } from "@vue-flow/core";
import type { Node, NodeVO, Edge } from "@/api-types";

/** DataNode 组件的 data 载荷 */
export interface DataNodeData {
  /** 节点标题 */
  title: string;
  /** 节点副标题 */
  subTitle: string;
  /** 引用的子画布 id，仅画布节点有值；影子节点恒为 null（影子的原始节点只能是普通节点） */
  canvasRefId: string | null;
  /** 节点自定义颜色字符串，空串 = 默认 */
  color: string;
  /** 影子节点根本体节点的 id；null 表示普通节点（用于判断是否影子节点、以及影子节点点击跳转/迁移落点时的定位锚点） */
  shadowId: string | null;
  /** 影子节点的原始节点是否已被逻辑删除（普通节点恒为 false） */
  shadowOriginDeleted: boolean;
  /** 影子节点的方向（inflow=入向，只有出度；outflow=出向，只有入度）；普通节点为 null */
  shadowDirection: "inflow" | "outflow" | null;
}

/**
 * 后端 Node 转 VFNode（data-node 类型）。position 可覆盖原坐标（如拖拽恢复到落点）。
 *
 * 入参 `node` 的实际类型由调用方决定：
 * - `user_database_node_list` 返回 `NodeVO`，携带影子扩展字段；
 * - `create` / `restore` 等返回普通 `Node`，不带扩展字段。
 * 这里按 `Partial<NodeVO>` 兜底，影子相关字段不存在时按默认值处理（普通节点）。
 */
export function toVFNode(node: Node, position?: { x: number; y: number }): VFNode {
  // node_list 返回的 NodeVO 带有影子扩展字段；create 等返回的普通 Node 没有，按默认值兜底。
  const vo = node as Partial<NodeVO>;
  return {
    id: node.id,
    type: "data-node",
    position: position ?? { x: node.x, y: node.y },
    data: {
      title: node.title,
      subTitle: node.sub_title,
      canvasRefId: node.canvas_ref_id,
      color: node.color,
      shadowId: vo.shadow_origin_id ?? null,
      shadowOriginDeleted: vo.shadow_origin_deleted ?? false,
      shadowDirection: vo.shadow_direction ?? null,
    } satisfies DataNodeData,
  };
}

/** 后端 Edge 转 VFEdge（含箭头端点）。data 中包含 title 和 description 供 CustomEdge 使用。 */
export function toVFEdge(edge: Edge): VFEdge {
  return {
    id: edge.id,
    source: edge.source_id,
    target: edge.target_id,
    sourceHandle: edge.source_port,
    targetHandle: edge.target_port,
    type: "custom",
    data: {
      title: edge.title,
      description: edge.description,
    },
    markerEnd: { type: MarkerType.ArrowClosed },
  };
}

/** VFEdge 转后端 Edge。canvas_id 不存在于视图模型，需显式提供；handle 即后端的 port。 */
export function fromVFEdge(vfEdge: VFEdge, canvasId: string): Edge {
  return {
    id: vfEdge.id,
    canvas_id: canvasId,
    source_id: vfEdge.source,
    target_id: vfEdge.target,
    source_port: vfEdge.sourceHandle ?? "",
    target_port: vfEdge.targetHandle ?? "",
    title: vfEdge.data?.title ?? "",
    description: vfEdge.data?.description ?? "",
  };
}
