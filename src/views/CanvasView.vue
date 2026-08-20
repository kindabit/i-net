<!--
  画布页面。

  渲染单个画布内的有向无环图，包括节点和边的可视化。
   左键拖动空白处框选节点（框选后拖动任一选中节点可批量移动），右键/中键拖动平移视口，视口变化自动持久化。
   路由携带 nodeId 时视角居中到目标节点。
   集成节点编辑、逻辑删除与回收站功能。
   支持自动布局（罗盘锚定分层）。
-->
<script setup lang="ts">
import { ref, onMounted, onUnmounted, type Ref } from "vue";
import { useRoute } from "vue-router";
import { isString } from "lodash";
import { VueFlow, useVueFlow } from "@vue-flow/core";
import type { Node as VFNode, Edge as VFEdge, Connection, VueFlowStore } from "@vue-flow/core";
import { Background } from "@vue-flow/background";
import { Controls } from "@vue-flow/controls";
import { t } from "@/i18n";
import {
  userDatabaseNodeList,
  userDatabaseEdgeList,
  userDatabaseNodeMoveNode,
  userDatabaseNodeMoveNodes,
  userDatabaseNodeCreate,
  userDatabaseNodeCopy,
  userDatabaseEdgeCreate,
  userDatabaseEdgeUpdate,
} from "@/api";
import type { Node, Edge } from "@/api-types";
import { toVFNode, toVFEdge } from "@/vf-convert";
import { snackbarErrorCode } from "@/composables/use-snackbar";
import { useViewportPersistence } from "@/composables/use-viewport";
import { useAutoLayout } from "@/composables/use-auto-layout";

import { useRecycleBin } from "@/composables/use-recycle-bin";
import { flyToRecycleBin, fadeInNode, fadeInEdges, ghostOutEdge } from "@/composables/use-canvas-animations";
import DataNode from "./DatabaseComponents/DataNode.vue";
import CustomEdge from "./DatabaseComponents/CustomEdge.vue";
import EdgeContextMenu from "./DatabaseComponents/EdgeContextMenu.vue";
import EditEdgeDialog from "./DatabaseComponents/EditEdgeDialog.vue";
import RecycleBinPanel from "./DatabaseComponents/RecycleBinPanel.vue";
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
  fallbackSize: { width: 160, height: 80 },
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
});

onUnmounted(() => {
  viewport.flush();
});

/**
 * VueFlow 实例初始化回调：在持久化视口恢复完成后，若路由携带 nodeId 查询参数，
 * 则将视角以动画飞行方式居中到目标节点（节点为 160×80 的固定尺寸，坐标为左上角，故偏移半个宽高）。
 * 输入：instance VueFlow 实例。
 * 返回：无返回值。
 */
function onFlowInit(instance: VueFlowStore) {
  const nodeId = route.query.nodeId;
  if (!isString(nodeId) || nodeId === "") return;
  const target = nodes.value.find((n) => n.id === nodeId);
  if (!target) return;
  instance.setCenter(target.position.x + 80, target.position.y + 40, {
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
 * 节点拖拽停止回调（含多选拖拽）。
 *
 * vue-flow store 已持有每个被拖动节点的最终 position（含 snap、clamp、parentNode 修正）；
 * 此处把每个新 position 整体替换回父组件 nodes.value 中对应节点，避免后续 filter / push
 * 操作触发 parseNode 把 store 中已拖动位置覆盖回 props 的初始坐标。
 * 持久化：单节点走单条 API；多节点走批量 API（userDatabaseNodeMoveNodes）。
 * @param event vue-flow NodeDragEvent（event / node / nodes）
 * @returns 无返回值
 */
function onNodeDragStop(event: { event: MouseEvent | TouchEvent; node: VFNode; nodes: VFNode[] }) {
  const moved = event.nodes.length > 0 ? event.nodes : [event.node];
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

async function onConnect(connection: Connection) {
  try {
    const newEdge = await userDatabaseEdgeCreate(
      canvasId,
      connection.source,
      connection.sourceHandle ?? "",
      connection.target,
      connection.targetHandle ?? "",
    );
    edges.value.push(toVFEdge(newEdge));
  } catch (e) {
    snackbarErrorCode(e);
  }
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
 * 连接合法性校验：不允许自环；出向影子只能作为目标（只有入度），入向影子只能作为源（只有出度）；
 * 影子节点不允许与画布节点相连（避免产生影子的影子；后端有 InvalidShadowEdge 兜底）。
 * @param connection vue-flow 连接对象
 * @returns 是否允许建立该连接
 */
function isValidConnection(connection: Connection): boolean {
  if (connection.source === connection.target) return false;
  const source = nodes.value.find((n) => n.id === connection.source);
  const target = nodes.value.find((n) => n.id === connection.target);
  if (!source || !target) return false;
  if (source.data.shadowDirection === "outflow") return false;
  if (target.data.shadowDirection === "inflow") return false;
  // "画布节点"判定须排除影子节点：影子节点的 canvasId 是后端从原始节点合并来的，
  // 它自身的 canvas_ref_id 为 null，后端语义上不是画布节点。
  const sourceIsShadow = !!source.data.shadowId;
  const targetIsShadow = !!target.data.shadowId;
  const sourceIsCanvasNode = !sourceIsShadow && !!source.data.canvasId;
  const targetIsCanvasNode = !targetIsShadow && !!target.data.canvasId;
  if ((sourceIsShadow && targetIsCanvasNode) || (targetIsShadow && sourceIsCanvasNode)) return false;
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
      :is-valid-connection="isValidConnection"
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
      @physical-delete="recycleBin.physicalDelete"
      @empty="recycleBin.empty"
    />
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
