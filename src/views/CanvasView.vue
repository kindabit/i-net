<!--
  画布页面。

  渲染单个画布内的有向无环图，包括节点和边的可视化。
   左键拖动空白处框选节点（框选后拖动任一选中节点可批量移动），右键/中键拖动平移视口，视口变化自动持久化。
   路由携带 nodeId 时视角居中到目标节点。
   集成节点编辑、逻辑删除与回收站功能。
   支持自动布局（罗盘锚定分层）。
    集成节点移动和迁移系统：按住 Alt 拖拽节点到画布节点、影子节点或面包屑祖先片段可跨画布迁移，
    并在落点处显示允许/禁止高亮；非法迁移尝试（落点有效但节点集不可迁移）弹出针对性错误提示；
    迁移成功后节点与内部边失焦淡出消失；Alt 按下时禁止画布边缘自动滚动以免误触迁移。
-->
<script setup lang="ts">
import { ref, onMounted, onUnmounted, type Ref } from "vue";
import { useRoute } from "vue-router";
import { isString } from "lodash";
import { VueFlow, useVueFlow } from "@vue-flow/core";
import type { Node as VFNode, Edge as VFEdge, Connection, VueFlowStore, NodeDragEvent } from "@vue-flow/core";
import { Background } from "@vue-flow/background";
import { Controls } from "@vue-flow/controls";
import { t } from "@/i18n";
import {
  userDatabaseNodeList,
  userDatabaseEdgeList,
  userDatabaseNodeMoveNode,
  userDatabaseNodeMoveNodes,
  userDatabaseNodeRelocateNodes,
  userDatabaseNodeCreate,
  userDatabaseNodeCopy,
  userDatabaseEdgeCreate,
  userDatabaseEdgeUpdate,
  userDatabaseCanvasList,
  userDatabaseViewportGet,
} from "@/api";
import type { Node, Edge, MoveNodeVO } from "@/api-types";
import { toVFNode, toVFEdge } from "@/vf-convert";
import { DATA_NODE_WIDTH, DATA_NODE_HEIGHT, DATA_NODE_HALF_WIDTH, DATA_NODE_HALF_HEIGHT } from "@/node-size";
import { snackbarErrorCode, snackbarText } from "@/composables/use-snackbar";
import { isErrorCode } from "@/error-code";
import { useViewportPersistence } from "@/composables/use-viewport";
import { useAutoLayout } from "@/composables/use-auto-layout";
import nodeMoveAndRelocate, { type Mode, type RelocatingLegality, type RelocatingTarget } from "@/composables/use-node-move-and-relocate.ts";
import { blurOutRelocated } from "@/composables/use-relocate-animation";
import { useRecycleBin } from "@/composables/use-recycle-bin";
import { flyToRecycleBin, fadeInNode, fadeInEdges, ghostOutEdge } from "@/composables/use-canvas-animations";
import DataNode from "./DatabaseComponents/DataNode.vue";
import CustomEdge from "./DatabaseComponents/CustomEdge.vue";
import EdgeContextMenu from "./DatabaseComponents/EdgeContextMenu.vue";
import EditEdgeDialog from "./DatabaseComponents/EditEdgeDialog.vue";
import RecycleBinPanel from "./DatabaseComponents/RecycleBinPanel.vue";
import ConfirmDialog from "@/components/ConfirmDialog.vue";
import EditNodeDialog from "./DatabaseComponents/EditNodeDialog.vue";
import EditDataNodeColorDialog from "@/node-colors/EditDataNodeColorDialog.vue";
import NodeTemplatePanel from "./DatabaseComponents/NodeTemplatePanel.vue";
import TemplateManagerDialog from "./DatabaseComponents/TemplateManagerDialog.vue";
import DictionaryManagerDialog from "./DatabaseComponents/DictionaryManagerDialog.vue";
import AttachmentDialog from "./DatabaseComponents/attachment/AttachmentDialog.vue";
// #if [DEBUG]
import ViewportDebugOverlay from "./DatabaseComponents/ViewportDebugOverlay.vue";
// #endif

// 不变量

const route = useRoute();
const canvasId = route.params.canvasId as string;
const containerRef = ref<HTMLElement>();
const edgeContextMenuRef = ref<InstanceType<typeof EdgeContextMenu>>();
const recycleBinPanelRef = ref<InstanceType<typeof RecycleBinPanel>>();
const editNodeDialogRef = ref<InstanceType<typeof EditNodeDialog>>();
const nodeColorDialogRef = ref<InstanceType<typeof EditDataNodeColorDialog>>();
const editEdgeDialogRef = ref<InstanceType<typeof EditEdgeDialog>>();
const templatePanelRef = ref<InstanceType<typeof NodeTemplatePanel>>();
const templateManagerDialogRef = ref<InstanceType<typeof TemplateManagerDialog>>();
const dictionaryManagerDialogRef = ref<InstanceType<typeof DictionaryManagerDialog>>();
const attachmentDialogRef = ref<InstanceType<typeof AttachmentDialog>>();
const confirmDialogRef = ref<InstanceType<typeof ConfirmDialog>>();
const recycleBinBtnRef = ref<any>(null);
const { screenToFlowCoordinate, updateNodeInternals, updateEdgeData, getNodes: getVFNodes, getEdges: getVFEdges, viewport: vfViewport } = useVueFlow();
const snapGrid: [number, number] = [20, 20];
const { isLayouting, applyAutoLayout } = useAutoLayout({
  // 节点/边必须取自 vue-flow store（GraphNode 的 position 是响应式活数据）：
  // 拖拽后 vue-flow 会替换 store 节点的 position 对象，本组件的 nodes 数组中已是过期坐标。
  getNodes: () => getVFNodes.value,
  getEdges: () => getVFEdges.value,
  persist: userDatabaseNodeMoveNodes,
  snapGrid,
  fallbackSize: { width: DATA_NODE_WIDTH, height: DATA_NODE_HEIGHT },
  // 自动布局动画结束、persist 之前把新坐标整体替换回父组件 nodes.position，
  // 避免后续增删节点时被 vue-flow 的 parseNode 用 props 中的旧坐标回滚 store。
  onNodesMoved: (items) => {
    const map = new Map(items.map((i) => [i.id, i]));
    for (const node of nodes.value) {
      const m = map.get(node.id);
      if (!m) continue;
      node.position = { x: m.x, y: m.y };
    }
  },
});

// 状态

const nodes: Ref<VFNode[]> = ref([]);
const edges: Ref<VFEdge[]> = ref([]);
const viewport = useViewportPersistence(canvasId, containerRef);
const recycleBin = useRecycleBin(canvasId);
const loaded = ref(false);

// 加载和卸载
onMounted(async () => {
  const [nodesResult, edgesResult] = await Promise.allSettled([
    userDatabaseNodeList(canvasId, false),
    userDatabaseEdgeList(canvasId),
    viewport.load(),
    recycleBin.load(),
  ]);

  if (nodesResult.status === "fulfilled") {
    nodes.value = nodesResult.value.map((n: Node) => toVFNode(n));
  } else {
    snackbarErrorCode(nodesResult.reason);
  }

  if (edgesResult.status === "fulfilled") {
    const loadedNodeIds = new Set(nodes.value.map((n) => n.id));
    edges.value = edgesResult.value
      .filter((e: Edge) => loadedNodeIds.has(e.source_id) && loadedNodeIds.has(e.target_id))
      .map((e: Edge) => toVFEdge(e));
  } else {
    snackbarErrorCode(edgesResult.reason);
  }

  loaded.value = true;

  nodeMoveAndRelocate.setQueryNodeAtPosition(queryNodeAtPosition);
  nodeMoveAndRelocate.listenKeyboardEvents();
  nodeMoveAndRelocate.listenOnDragStop(onNodeDragStopEffect);
});

onUnmounted(() => {
  viewport.flush();
  nodeMoveAndRelocate.unlistenKeyboardEvents();
  nodeMoveAndRelocate.unlistenOnDragStop(onNodeDragStopEffect);
});

/**
 * VueFlow 实例初始化回调：在持久化视口恢复完成后，若路由携带 nodeId 查询参数，
 * 则将视角以动画飞行方式居中到目标节点（节点为固定尺寸，见 node-size.ts；坐标为左上角，故偏移半个宽高）。
 * 输入：instance VueFlow 实例。
 * 返回：无返回值。
 */
function onFlowInit(instance: VueFlowStore) {
  const nodeId = route.query.nodeId;
  if (!isString(nodeId) || nodeId === "") return;
  const target = nodes.value.find((n) => n.id === nodeId);
  if (!target) return;
  instance.setCenter(target.position.x + DATA_NODE_HALF_WIDTH, target.position.y + DATA_NODE_HALF_HEIGHT, {
    zoom: viewport.current.value.zoom,
    duration: 300,
  });
}

// 节点

/**
 * 打开指定节点的编辑对话框。
 *
 * 普通节点以编辑模式打开；影子节点（data.shadowId 非 null）以只读模式打开原始节点的对话框，
 * 字段数据通过传入的原始节点 id 从 userDatabaseNodeFieldGet 加载。只读模式不会
 * resolve 出非 null 值，因此影子节点不会走到下方的标题回写。
 * @param id 节点 id（影子节点是画布中的虚拟节点 id，原始节点 id 通过 data.shadowId 获取）
 * @returns 无返回值
 */
async function onNodeEdit(id: string) {
  const node = nodes.value.find((n) => n.id === id);
  if (!node) return;
  // 影子节点以只读形式打开原始节点的编辑对话框：传入原始节点 id，字段从原始节点加载；
  // 只读模式不会 resolve 出非 null 值，因此影子节点不会走到下方的标题回写。
  const shadowId = node.data.shadowId as string | null;
  const result = await editNodeDialogRef.value?.open(
    { id: shadowId ?? id, title: node.data.title, subTitle: node.data.subTitle },
    { readonly: !!shadowId },
  );
  if (!result) return;
  node.data.title = result.title;
  node.data.subTitle = result.subTitle;
}

async function onNodeLogicalDelete(id: string) {
  if (!nodes.value.some((n) => n.id === id)) return;

  const ok = await recycleBin.logicalDelete(id);
  if (!ok) return;

  // 相连边：克隆残影播放淡出（fire-and-forget），数据立即移除
  const leavingEdges = edges.value.filter((e) => e.source === id || e.target === id);
  for (const e of leavingEdges) ghostOutEdge(e.id);
  edges.value = edges.value.filter((e) => e.source !== id && e.target !== id);

  // 播放飞向回收站动画（克隆体覆盖原位）
  const btnEl = recycleBinBtnRef.value?.$el as HTMLElement | undefined;
  if (btnEl) {
    flyToRecycleBin(id, btnEl);
  }

  // 立即从画布移除节点
  nodes.value = nodes.value.filter((n) => n.id !== id);
}

/**
 * 复制指定节点到画布视口正中央。
 *
 * 落点取画布容器中心的屏幕坐标，经 screenToFlowCoordinate 换算为画布坐标后
 * 手动按 snapGrid 取整（vue-flow 的 snap 只在拖拽时生效，程序写入的坐标需自行对齐）。
 * @param id 被复制的节点 id
 * @returns 无返回值
 */
function onNodeCopy(id: string): void {
  const el = containerRef.value;
  if (!el) return;
  const rect = el.getBoundingClientRect();
  const center = screenToFlowCoordinate({
    x: rect.left + rect.width / 2,
    y: rect.top + rect.height / 2,
  });
  const x = Math.round(center.x / snapGrid[0]) * snapGrid[0];
  const y = Math.round(center.y / snapGrid[1]) * snapGrid[1];
  userDatabaseNodeCopy(id, x, y)
    .then((created) => {
      nodes.value.push(toVFNode(created));
    })
    .catch(snackbarErrorCode);
}

/**
 * 打开指定节点的附件管理对话框。
 * @param id 节点 id
 * @returns 无返回值
 */
function onNodeAttachment(id: string): void {
  const node = nodes.value.find((n) => n.id === id);
  if (!node) return;
  attachmentDialogRef.value?.open(id, node.data.title);
}

/**
 * 打开节点自定义颜色对话框，保存成功后更新节点颜色（持久化由对话框内部完成）。
 * @param id 节点 id
 * @returns 无返回值
 */
async function onNodeColor(id: string): Promise<void> {
  const node = nodes.value.find((n) => n.id === id);
  if (!node) return;
  const dialog = nodeColorDialogRef.value;
  if (!dialog) return;
  const result = await dialog.open(id, node.data.title, node.data.subTitle, node.data.color);
  if (result === null) return;
  node.data.color = result;
}

/**
 * 切换模板面板显示状态，并在面板打开时关闭回收站面板（两面板互斥）。
 * 无输入参数，无返回值。
 */
function onTemplatePanelToggle(): void {
  templatePanelRef.value?.toggle();
  if (templatePanelRef.value?.visible) {
    recycleBinPanelRef.value?.close();
  }
}

/**
 * 切换回收站面板显示状态，并在面板打开时关闭模板面板（两面板互斥）。
 * 无输入参数，无返回值。
 */
function onRecycleBinToggle(): void {
  recycleBinPanelRef.value?.toggle();
  if (recycleBinPanelRef.value?.visible) {
    templatePanelRef.value?.close();
  }
}

// 回收站

async function onNodeRestore(node: Node, position?: { x: number; y: number }) {
  const pos = position ?? { x: node.x, y: node.y };
  const ok = await recycleBin.restore(node, pos.x, pos.y);
  if (!ok) return;

  nodes.value.push(toVFNode(node, pos));
  // 等待入场动画播完再校正 handle 测量：缩放动画会干扰 getBoundingClientRect 测量，
  // 必须先播完再 updateNodeInternals，之后添加的边才能拿到正确的连接点
  await fadeInNode(node.id);
  updateNodeInternals([node.id]);

  // 恢复与当前节点相连、且另一端也在画布上的边（删除时相关边已立即移除，此处直接补全）
  try {
    const allEdges = await userDatabaseEdgeList(canvasId);
    const visibleNodeIds = new Set(nodes.value.map((n) => n.id));
    const restored = allEdges.filter(
      (e: Edge) =>
        (e.source_id === node.id || e.target_id === node.id) &&
        visibleNodeIds.has(e.source_id) &&
        visibleNodeIds.has(e.target_id),
    );
    edges.value.push(...restored.map((e: Edge) => toVFEdge(e)));
    fadeInEdges(restored.map((e: Edge) => e.id));
  } catch (e) {
    snackbarErrorCode(e);
  }
}

// 拖拽恢复

function onDragOver(event: DragEvent) {
  event.preventDefault();
  if (event.dataTransfer) {
    if (event.dataTransfer.types.includes("application/x-inet-recycle-node")) {
      event.dataTransfer.dropEffect = "move";
    } else {
      event.dataTransfer.dropEffect = "copy";
    }
  }
}

function onDrop(event: DragEvent) {
  const id = event.dataTransfer?.getData("application/x-inet-recycle-node");
  if (id) {
    const node = recycleBin.deletedNodes.value.find((n) => n.id === id);
    if (node) {
      const pos = screenToFlowCoordinate({ x: event.clientX, y: event.clientY });
      onNodeRestore(node, pos);
      return;
    }
  }
  const raw = event.dataTransfer?.getData("application/x-i-net-template") ?? "";
  if (raw !== "") {
    const point = screenToFlowCoordinate({ x: event.clientX, y: event.clientY });
    const template_id = raw === "blank" ? null : raw;
    const subTitle = event.dataTransfer?.getData("application/x-i-net-template-name") ?? "";
    const createCanvas = event.dataTransfer?.getData("application/x-i-net-create-canvas") === "true";
    userDatabaseNodeCreate(
      canvasId,
      createCanvas ? t("database.canvas.new-canvas-name") : t("database.canvas.new-node-title"),
      subTitle,
      point.x,
      point.y,
      template_id,
      createCanvas,
    ).then((created) => {
      nodes.value.push(toVFNode(created));
      templatePanelRef.value?.close();
    }).catch(snackbarErrorCode);
  }
}

// 边

/**
 * 拖拽连线建立边：同向已有边时后端直接更新连接桩（边 id 不变，无断连确认）；
 * 反向已有边时后端执行删旧建新替换，若替换会断开影子节点的关联连接，
 * 后端返回 EdgeDeleteDisconnectsNodes，弹出确认框，用户确认后以 confirmed=true 重调；
 * 无断连影响时静默替换。
 * @param connection vue-flow 连接对象
 * @returns 无返回值
 */
async function onConnect(connection: Connection) {
  try {
    const newEdge = await userDatabaseEdgeCreate(
      canvasId,
      connection.source,
      connection.sourceHandle ?? "",
      connection.target,
      connection.targetHandle ?? "",
      false,
    );
    upsertEdgeLocally(newEdge);
  } catch (e) {
    if (isErrorCode(e, "EdgeDeleteDisconnectsNodes")) {
      const rawNodes = e.data?.nodes;
      const nodes: string[] = Array.isArray(rawNodes) ? rawNodes.map(String) : [];
      const separator = t("database.canvas.delete-edge-disconnect-separator");
      const confirmed = await confirmDialogRef.value?.open({
        title: t("database.canvas.replace-edge-disconnect-title"),
        text: t("database.canvas.replace-edge-disconnect-text", {
          nodes: nodes.join(separator),
        }),
        confirmText: t("database.canvas.replace-edge-confirm"),
        confirmColor: "error",
      });
      if (!confirmed) return;
      try {
        const newEdge = await userDatabaseEdgeCreate(
          canvasId,
          connection.source,
          connection.sourceHandle ?? "",
          connection.target,
          connection.targetHandle ?? "",
          true,
        );
        upsertEdgeLocally(newEdge);
      } catch (e2) {
        snackbarErrorCode(e2);
      }
      return;
    }
    snackbarErrorCode(e);
  }
}

/**
 * 把后端返回的新边写入本地边集：替换语义下先移除与新边同向或反向的旧边，
 * 再加入新边；普通新建时无旧边可移除，等价于直接追加。
 * @param newEdge 后端返回的新边
 * @returns 无返回值
 */
function upsertEdgeLocally(newEdge: Edge) {
  edges.value = edges.value.filter(
    (e) =>
      !(
        (e.source === newEdge.source_id && e.target === newEdge.target_id) ||
        (e.source === newEdge.target_id && e.target === newEdge.source_id)
      ),
  );
  edges.value.push(toVFEdge(newEdge));
}

function onEdgeContextMenuFromFlow(e: { edge: { id: string }; event: MouseEvent | TouchEvent }) {
  if (!(e.event instanceof MouseEvent)) return;
  e.event.preventDefault();
  onEdgeContextMenu(e.edge.id, { x: e.event.clientX, y: e.event.clientY });
}

/** 节点右键：阻止浏览器原生菜单 */
function onNodeContextMenu({ event }: { event: MouseEvent | TouchEvent }) {
  if (!(event instanceof MouseEvent)) return;
  event.preventDefault();
}

/** 画布背景右键：阻止浏览器原生菜单 */
function onPaneContextMenu(event: MouseEvent) {
  event.preventDefault();
}

function onEdgeContextMenu(id: string, pos: { x: number; y: number }) {
  edgeContextMenuRef.value?.open(id, pos, edges);
}

/**
 * 连接合法性校验：不允许自环；影子节点之间不能互相连接（后端 ShadowToShadowEdge 兜底）；
 * 画布节点之间不能互相连接（后端 CanvasToCanvasEdge 兜底）；
 * 不允许两端连接桩相同；方向守卫要求出向影子只能作为目标（只有入度），
 * 入向影子只能作为源（只有出度），后端有 InvalidShadowEdge 兜底。
 * @param connection vue-flow 连接对象
 * @returns 是否允许建立该连接
 */
function isValidConnection(connection: Connection): boolean {
  if (connection.source === connection.target) return false;
  const source = nodes.value.find((n) => n.id === connection.source);
  const target = nodes.value.find((n) => n.id === connection.target);
  if (!source || !target) return false;
  if (source.data.shadowId !== null && target.data.shadowId !== null) return false;
  // 画布节点之间不能互相连接（后端 CanvasToCanvasEdge 兜底）；
  // 判断需排除影子：影子不是画布节点，其 canvasId 不参与本判断。
  const sourceIsCanvas = source.data.canvasId !== null && source.data.shadowId === null;
  const targetIsCanvas = target.data.canvasId !== null && target.data.shadowId === null;
  if (sourceIsCanvas && targetIsCanvas) return false;
  if (source.data.shadowDirection === "outflow") return false;
  if (target.data.shadowDirection === "inflow") return false;
  // 不允许两端连接桩相同（与 onConnect 的 ?? "" 兜底保持一致，避免 null 误判为"无 port 即合法"）。
  if ((connection.sourceHandle ?? "") === (connection.targetHandle ?? "")) return false;
  return true;
}

/**
 * 编辑边：打开编辑对话框，用户确认后调用 API 更新边的标题和详情。
 * 使用 Vue Flow 的 updateEdgeData 更新边数据并触发重新渲染。
 * @param id 边 id
 * @returns 无返回值
 */
async function onEdgeEdit(id: string): Promise<void> {
  const edge = edges.value.find((e) => e.id === id);
  if (!edge) return;
  const result = await editEdgeDialogRef.value?.open({
    title: edge.data?.title ?? "",
    description: edge.data?.description ?? "",
  });
  if (!result) return;
  try {
    await userDatabaseEdgeUpdate(id, result.title, result.description);
    edge.data = { ...edge.data, title: result.title, description: result.description };
    updateEdgeData(id, edge.data);
  } catch (e) {
    snackbarErrorCode(e);
  }
}

// 节点移动和迁移系统

/**
 * 查询指定屏幕坐标位置下的所有节点。
 *
 * 基于 document.elementsFromPoint 做 DOM 命中测试：vue-flow 节点容器带
 * vue-flow__node 类和 data-id 属性，重叠节点与被拖动节点下方的节点都会被
 * 收集，按自顶向下的顺序返回。被拖动的节点也会被返回——其 data 上没有
 * canvasId/shadowId 时会被状态机的迁移目标计算自然跳过，无需在此排除。
 * @param position 屏幕坐标（clientX / clientY）
 * @returns 该位置处的节点数组，按 DOM 自顶向下排序
 */
function queryNodeAtPosition(position: { x: number, y: number }): VFNode[] {
  const byId = new Map(nodes.value.map((n) => [n.id, n]));
  const result: VFNode[] = [];
  for (const el of document.elementsFromPoint(position.x, position.y)) {
    if (el instanceof HTMLElement && el.classList.contains("vue-flow__node")) {
      const node = el.dataset.id ? byId.get(el.dataset.id) : undefined;
      if (node) result.push(node);
    }
  }
  return result;
}

/**
 * 从 vue-flow 拖拽事件中取出实际被拖动的节点集。
 * @param event vue-flow NodeDragEvent
 * @returns 被拖动的节点数组（多选拖拽时 nodes 非空，否则回退为单个 node）
 */
function draggedNodesOf(event: NodeDragEvent): VFNode[] {
  return event.nodes.length > 0 ? event.nodes : [event.node];
}

/**
 * 节点拖拽开始回调：仅鼠标事件接入状态机；触摸事件不接入（触屏只支持画布内移动）。
 * @param event vue-flow NodeDragEvent
 * @returns 无返回值
 */
function onNodeDragStart(event: NodeDragEvent) {
  if (!(event.event instanceof MouseEvent)) return;
  nodeMoveAndRelocate.onDragStart(draggedNodesOf(event), getVFEdges.value, event.event);
}

/**
 * 节点拖拽过程回调：仅鼠标事件接入状态机。
 * @param event vue-flow NodeDragEvent
 * @returns 无返回值
 */
function onNodeDrag(event: NodeDragEvent) {
  if (!(event.event instanceof MouseEvent)) return;
  nodeMoveAndRelocate.onDrag(draggedNodesOf(event), getVFEdges.value, event.event);
}

/**
 * 节点拖拽停止回调：鼠标事件交给状态机（持久化由拖拽结束监听器统一处理）；
 * 触摸事件不经过状态机，直接按画布内移动持久化。
 * @param event vue-flow NodeDragEvent
 * @returns 无返回值
 */
function onNodeDragStop(event: NodeDragEvent) {
  const moved = draggedNodesOf(event);
  if (event.event instanceof MouseEvent) {
    nodeMoveAndRelocate.onDragStop(moved, getVFEdges.value, event.event);
  } else {
    persistMove(moved);
  }
}

/**
 * 画布内移动持久化：先把被拖动节点的最终 position 回写进 nodes.value
 * （避免后续 filter/push 触发 parseNode 用 props 旧坐标回滚 store），
 * 再单节点走单条 API、多节点走批量 API。
 * @param moved 被拖动的节点数组
 * @returns 无返回值
 */
function persistMove(moved: VFNode[]) {
  const movedMap = new Map(moved.map((n) => [n.id, n]));
  for (const node of nodes.value) {
    const m = movedMap.get(node.id);
    if (!m) continue;
    node.position = { x: m.position.x, y: m.position.y };
  }
  const items = moved.map((n) => ({ id: n.id, x: n.position.x, y: n.position.y }));
  if (items.length === 1) {
    userDatabaseNodeMoveNode(items[0].id, items[0].x, items[0].y).catch(snackbarErrorCode);
  } else if (items.length > 1) {
    userDatabaseNodeMoveNodes(items).catch(snackbarErrorCode);
  }
}

/**
 * 解析迁移目标画布 id。
 *
 * canvas-node / breadcrumb-segment 目标直接携带 canvasId；
 * shadow-node 目标取当前画布的父画布 id（影子 origin 恒位于父画布——该不变量由后端
 * 迁移校验保证：与画布节点有边的节点永远无法通过合法性校验，故影子 origin 不会被迁走）。
 * @param target 状态机算出的迁移目标
 * @returns 目标画布 id；解析失败（当前画布无父画布或查询出错）时返回 null
 */
async function resolveRelocateTargetCanvasId(target: RelocatingTarget): Promise<string | null> {
  if (target.type === "canvas-node" || target.type === "breadcrumb-segment") {
    return target.canvasId;
  }
  try {
    const canvases = await userDatabaseCanvasList(false);
    const current = canvases.find((c) => c.id === canvasId);
    return current?.parent_id ?? null;
  } catch (e) {
    snackbarErrorCode(e);
    return null;
  }
}

/**
 * 计算迁移落点：节点区域包围盒（节点固定尺寸，见 node-size.ts）中心平移到目标锚点，
 * 平移量按吸附网格逐轴取整——源坐标本就网格对齐，取整后的平移量保证结果仍对齐，
 * 且节点之间的相对位置关系不变。
 * @param draggedNodes 被拖动的节点数组
 * @param center 目标锚点（目标画布视口中心的画布坐标）
 * @returns 迁移条目列表（id + 最终坐标）
 */
function computeRelocateItems(draggedNodes: VFNode[], center: { x: number; y: number }): MoveNodeVO[] {
  const minX = Math.min(...draggedNodes.map((n) => n.position.x));
  const minY = Math.min(...draggedNodes.map((n) => n.position.y));
  const maxX = Math.max(...draggedNodes.map((n) => n.position.x));
  const maxY = Math.max(...draggedNodes.map((n) => n.position.y));
  const dx = Math.round((center.x - (minX + maxX + DATA_NODE_WIDTH) / 2) / snapGrid[0]) * snapGrid[0];
  const dy = Math.round((center.y - (minY + maxY + DATA_NODE_HEIGHT) / 2) / snapGrid[1]) * snapGrid[1];
  return draggedNodes.map((n) => ({ id: n.id, x: n.position.x + dx, y: n.position.y + dy }));
}

/**
 * 拖拽结束监听器（注册进状态机）：以统一形式落实节点的画布内移动与跨画布迁移。
 *
 * 迁移条件：relocate 模式 + 节点集合法 + 有迁移目标；其余一律按画布内移动持久化。
 * 非法迁移尝试（relocate 模式 + 有迁移目标 + 节点集非法）按非法原因弹出针对性错误提示，
 * 随后仍按画布内移动持久化。
 * 迁移失败（含目标解析失败、视口查询失败、API 报错）时回退为画布内移动持久化——
 * 节点已在视觉上移位，坐标必须落库，否则刷新后位置跳变。
 * 迁移成功后先对被迁移节点与两端都在集合内的内部边播放失焦淡出动画，
 * 动画结束再从本地移除（对齐逻辑删除的本地移除模式）。
 * @param mode 拖拽结束时的模式
 * @param draggedNodes 被拖动的节点数组
 * @param legality 节点集的迁移合法性
 * @param _pointerPosition 落点屏幕坐标（本实现未使用）
 * @param target 迁移目标
 * @returns 无返回值
 */
async function onNodeDragStopEffect(mode: Mode, draggedNodes: VFNode[], legality: RelocatingLegality, _pointerPosition: { x: number; y: number }, target: RelocatingTarget | null) {
  if (mode === "relocate" && target !== null) {
    if (legality !== "legal") {
      // 用户按住 Alt 把节点拖到有效迁移目标上，但节点集不满足迁移条件：按非法原因提示
      const key =
        legality === "has-shadow" ? "database.canvas.relocate-has-shadow"
        : legality === "has-canvas" ? "database.canvas.relocate-has-canvas"
        : "database.canvas.relocate-has-external"; // has-external
      snackbarText(t(key), "error");
    } else {
      const targetCanvasId = await resolveRelocateTargetCanvasId(target);
      if (targetCanvasId !== null) {
        try {
          // 视口的持久化语义为"视口中心"：中心画布坐标 = (-x / zoom, -y / zoom)；
          // 目标画布无视口记录时 GET 返回默认值 (0, 0, 1)，代入即画布原点 (0, 0)
          const vp = await userDatabaseViewportGet(targetCanvasId);
          const center = { x: -vp.x / vp.zoom, y: -vp.y / vp.zoom };
          const items = computeRelocateItems(draggedNodes, center);
          await userDatabaseNodeRelocateNodes(items, targetCanvasId);
          const movedIds = new Set(draggedNodes.map((n) => n.id));
          const internalEdgeIds = edges.value
            .filter((e) => movedIds.has(e.source) && movedIds.has(e.target))
            .map((e) => e.id);
          await blurOutRelocated([...movedIds], internalEdgeIds);
          nodes.value = nodes.value.filter((n) => !movedIds.has(n.id));
          edges.value = edges.value.filter((e) => !movedIds.has(e.source) && !movedIds.has(e.target));
          return;
        } catch (e) {
          snackbarErrorCode(e);
        }
      }
    }
  }
  persistMove(draggedNodes);
}

/**
 * 物理删除节点（第二道确认）：RecycleBinPanel 的基础确认通过后进入此函数。
 * 先以 confirmed=false 调用；若后端返回 NodeDeleteDisconnectsNodes（影子子树在其它画布
 * 有关联节点），弹出断连确认对话框，用户确认后以 confirmed=true 重调。
 * @param node 待物理删除的节点
 * @returns 无返回值
 */
async function onNodePhysicalDelete(node: Node): Promise<void> {
  try {
    await recycleBin.physicalDelete(node, false);
    return;
  } catch (e) {
    if (!isErrorCode(e, "NodeDeleteDisconnectsNodes")) {
      snackbarErrorCode(e);
      return;
    }
    const rawNodes = e.data?.nodes;
    const disconnected: string[] = Array.isArray(rawNodes) ? rawNodes.map(String) : [];
    const separator = t("database.canvas.delete-edge-disconnect-separator");
    const confirmed = await confirmDialogRef.value?.open({
      title: t("database.canvas.delete-node-disconnect-title"),
      text: t("database.canvas.delete-node-disconnect-text", {
        title: node.title,
        nodes: disconnected.join(separator),
      }),
      confirmText: t("database.canvas.physical-delete-node"),
      confirmColor: "error",
    });
    if (!confirmed) return;
    await recycleBin.physicalDelete(node, true);
  }
}
</script>

<template>
  <div ref="containerRef" class="canvas-view">
    <div v-if="loaded" class="canvas-floating-menu frosted-glass">
      <VBtn
        icon="mdi-plus"
        size="small"
        variant="text"
        :title="t('database.canvas.new-node')"
        @click="onTemplatePanelToggle"
      />
      <VBadge
        :content="recycleBin.deletedNodes.value.length"
        :model-value="recycleBin.deletedNodes.value.length > 0"
        color="error"
        offset-x="4"
        offset-y="4"
      >
        <VBtn
          ref="recycleBinBtnRef"
          icon="mdi-trash-can-outline"
          size="small"
          variant="text"
          :title="t('database.canvas.recycle-bin')"
          @click="onRecycleBinToggle"
        />
      </VBadge>
      <VBtn
        icon="mdi-book-open-variant-outline"
        size="small"
        variant="text"
        :title="t('database.dictionary.manager-title')"
        @click="dictionaryManagerDialogRef?.open()"
      />
      <VBtn
        icon="mdi-auto-fix"
        size="small"
        variant="text"
        :title="t('database.canvas.auto-layout')"
        :disabled="isLayouting"
        @click="applyAutoLayout"
      />
    </div>
    <div v-if="!loaded" class="canvas-view-loading">
      <VProgressCircular indeterminate color="primary" />
    </div>
    <VueFlow
      v-else
      :nodes="nodes"
      :edges="edges"
      :default-viewport="viewport.initial.value"
      :max-zoom="4"
      :min-zoom="0.1"
      :snap-to-grid="true"
      :snap-grid="snapGrid"
      :delete-key-code="null"
      :pan-on-drag="[1, 2]"
      :selection-key-code="true"
      :auto-pan-on-node-drag="nodeMoveAndRelocate.mode.value !== 'relocate'"
      :is-valid-connection="isValidConnection"
      @node-drag-start="onNodeDragStart"
      @node-drag="onNodeDrag"
      @node-drag-stop="onNodeDragStop"
      @viewport-change="viewport.save"
      @init="onFlowInit"
      @connect="onConnect"
      @edge-context-menu="onEdgeContextMenuFromFlow"
      @node-context-menu="onNodeContextMenu"
      @pane-context-menu="onPaneContextMenu"
      @drop="onDrop"
      @dragover="onDragOver"
    >
      <Background pattern="dots" :gap="20" :size="1" />
      <Controls class="theme-controls frosted-glass" />
      <template #node-data-node="{ id, data, selected }">
        <DataNode :id="id" :data="data" :selected="selected" @delete="onNodeLogicalDelete" @edit="onNodeEdit" @copy="onNodeCopy" @attachment="onNodeAttachment" @color="onNodeColor" />
      </template>
      <template #edge-custom="edgeProps">
        <CustomEdge v-bind="edgeProps" @contextmenu="(p) => onEdgeContextMenu(p.id, { x: p.x, y: p.y })" />
      </template>
    </VueFlow>
    <EdgeContextMenu ref="edgeContextMenuRef" @edit="onEdgeEdit" />
    <EditEdgeDialog ref="editEdgeDialogRef" />
    <EditNodeDialog ref="editNodeDialogRef" />
        <EditDataNodeColorDialog ref="nodeColorDialogRef" />
    <AttachmentDialog ref="attachmentDialogRef" />
    <NodeTemplatePanel ref="templatePanelRef" @open-template-manager="templateManagerDialogRef?.open()" />
    <TemplateManagerDialog ref="templateManagerDialogRef" />
    <DictionaryManagerDialog ref="dictionaryManagerDialogRef" />
    <RecycleBinPanel
      ref="recycleBinPanelRef"
      :nodes="recycleBin.deletedNodes.value"
      @restore="onNodeRestore"
      @physical-delete="onNodePhysicalDelete"
      @empty="recycleBin.empty"
    />
    <ConfirmDialog ref="confirmDialogRef" />
    <!-- #if [DEBUG] -->
    <ViewportDebugOverlay :viewport="vfViewport" />
    <!-- #endif -->
  </div>
</template>

<style lang="scss" scoped>
.canvas-view {
  width: 100%;
  height: 100%;
  position: relative;
}

.canvas-floating-menu {
  position: absolute;
  top: 0.75rem;
  left: 0.75rem;
  z-index: 10;
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  padding: 0.25rem;
  border-radius: 0.5rem;
}

.canvas-view-loading {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 100%;
  height: 100%;
}
</style>
