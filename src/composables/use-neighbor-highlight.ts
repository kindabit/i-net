/**
 * 邻居高亮状态：选中节点时高亮所有与之相连的边，选中边时高亮其两端节点。
 *
 * 模块级单例：同一时刻仅有一个画布页面挂载（由路由切换保证），无多实例冲突；
 * 与 use-fatal-error、use-snackbar 等模块级状态的语义一致。
 * 画布页面卸载时自动重置模块级状态，避免残留状态泄漏到下一个画布。
 *
 * 派生逻辑抽为纯函数（computeHighlightedEdgeIds / computeHighlightedNodeIds）以便单元测试，
 * 模块级 computed 仅做薄包装。
 */
import { computed, ref, watchEffect, onUnmounted, type ComputedRef, type Ref } from "vue";
import { useVueFlow } from "@vue-flow/core";
import type { Edge as VFEdge } from "@vue-flow/core";

/** 当前画布中选中的节点 id 集合（由 setupNeighborHighlight 从 vue-flow store 同步） */
const selectedNodeIds = ref<ReadonlySet<string>>(new Set());

/** 当前画布中选中的边端点列表（由 setupNeighborHighlight 从 vue-flow store 同步） */
const selectedEdgeEndpoints = ref<ReadonlyArray<{ source: string; target: string }>>([]);

/** 当前画布的边列表快照（由 setupNeighborHighlight 同步，仅取派生所需的三个字段） */
const currentEdges = ref<ReadonlyArray<{ id: string; source: string; target: string }>>([]);

/**
 * 由选中节点集合派生需要高亮的边 id 集合：与任一选中节点相连（作为 source 或 target）的边全部高亮。
 * @param selectedNodeIds 选中节点 id 集合
 * @param edges 当前画布的边列表（仅读取 id/source/target 字段）
 * @returns 高亮边 id 集合
 */
export function computeHighlightedEdgeIds(
  selectedNodeIds: ReadonlySet<string>,
  edges: ReadonlyArray<{ id: string; source: string; target: string }>,
): Set<string> {
  if (selectedNodeIds.size === 0) return new Set();
  const result = new Set<string>();
  for (const e of edges) {
    if (selectedNodeIds.has(e.source) || selectedNodeIds.has(e.target)) {
      result.add(e.id);
    }
  }
  return result;
}

/**
 * 由选中边列表派生需要高亮的节点 id 集合：选中边的所有 source 与 target 端点全部高亮（多条边共享端点时去重）。
 * @param selectedEdges 选中边的端点列表（仅读取 source/target 字段）
 * @returns 高亮节点 id 集合
 */
export function computeHighlightedNodeIds(
  selectedEdges: ReadonlyArray<{ source: string; target: string }>,
): Set<string> {
  if (selectedEdges.length === 0) return new Set();
  const result = new Set<string>();
  for (const e of selectedEdges) {
    result.add(e.source);
    result.add(e.target);
  }
  return result;
}

/** 高亮边 id 集合（响应式）：随选中节点与边列表变化自动重算，供 CustomEdge 判断自身是否高亮 */
export const highlightedEdgeIds: ComputedRef<ReadonlySet<string>> = computed(() =>
  computeHighlightedEdgeIds(selectedNodeIds.value, currentEdges.value),
);

/** 高亮节点 id 集合（响应式）：随选中边变化自动重算，供 DataNode 判断自身是否高亮 */
export const highlightedNodeIds: ComputedRef<ReadonlySet<string>> = computed(() =>
  computeHighlightedNodeIds(selectedEdgeEndpoints.value),
);

/**
 * 在画布页面的 setup 中调用：建立 vue-flow 选中状态到模块级状态的同步，组件卸载时停止同步并重置。
 * @param edges 当前画布的边列表（响应式）
 * @returns 无返回值
 */
export function setupNeighborHighlight(edges: Ref<VFEdge[]>): void {
  const { getSelectedNodes, getSelectedEdges } = useVueFlow();

  watchEffect(() => {
    currentEdges.value = edges.value.map((e) => ({ id: e.id, source: e.source, target: e.target }));
  });
  watchEffect(() => {
    selectedNodeIds.value = new Set(getSelectedNodes.value.map((n) => n.id));
  });
  watchEffect(() => {
    selectedEdgeEndpoints.value = getSelectedEdges.value.map((e) => ({ source: e.source, target: e.target }));
  });

  onUnmounted(() => {
    selectedNodeIds.value = new Set();
    selectedEdgeEndpoints.value = [];
    currentEdges.value = [];
  });
}
