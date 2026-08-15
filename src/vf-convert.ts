/**
 * 后端实体（api-types）与 vue-flow 视图模型（VFNode/VFEdge）之间的转换。
 * 视图层与后端实体之间的结构换算统一收敛于此模块，组件内禁止内联构造。
 */
import { MarkerType } from "@vue-flow/core";
import type { Node as VFNode, Edge as VFEdge } from "@vue-flow/core";
import type { Node, Edge } from "@/api-types";

/** DataNode 组件的 data 载荷 */
export interface DataNodeData {
  /** 节点标题 */
  title: string;
  /** 节点副标题 */
  subTitle: string;
  /** 引用的子画布 id，仅画布节点有值 */
  canvasId: string | null;
  /** 节点自定义颜色字符串，空串 = 默认 */
  color: string;
}

/** 后端 Node 转 VFNode（data-node 类型）。position 可覆盖原坐标（如拖拽恢复到落点）。 */
export function toVFNode(node: Node, position?: { x: number; y: number }): VFNode {
  return {
    id: node.id,
    type: "data-node",
    position: position ?? { x: node.x, y: node.y },
    data: {
      title: node.title,
      subTitle: node.sub_title,
      canvasId: node.canvas_ref_id,
      color: node.color,
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
