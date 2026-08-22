<!--
  画布宇宙页面。

  以 vue-flow 渲染画布宇宙中的所有画布节点（每个画布作为一个节点）。
  左键拖动空白处框选节点（框选后拖动任一选中节点可批量移动），右键/中键拖动平移视口，
  画布节点可拖拽移动并自动持久化位置，可重命名（根画布除外）。
  视口变化以 500ms 防抖保存至后端，恢复时自动定位至上一次浏览位置。
  集成画布回收站功能：逻辑删除、恢复、物理删除、拖拽恢复及飞入飞出动画。
  支持自动布局（罗盘锚定分层）。
-->
<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, type Ref } from "vue";
import { VueFlow, useVueFlow, MarkerType } from "@vue-flow/core";
import type { Node as VFNode, Edge as VFEdge } from "@vue-flow/core";
import { Background } from "@vue-flow/background";
import { Controls } from "@vue-flow/controls";
import { t } from "@/i18n";
import {
  userDatabaseCanvasList,
  userDatabaseCanvasMoveCanvas,
  userDatabaseCanvasMoveCanvases,
  userDatabaseCanvasRename,
} from "@/api";
import type { Canvas } from "@/api-types";
import { snackbarErrorCode } from "@/composables/use-snackbar";
import { useViewportPersistence } from "@/composables/use-viewport";
import { useAutoLayout } from "@/composables/use-auto-layout";
import { useCanvasRecycleBin } from "@/composables/use-canvas-recycle-bin";
import { flyToRecycleBin, fadeInNode } from "@/composables/use-canvas-animations";
import { CANVAS_NODE_FALLBACK_WIDTH, CANVAS_NODE_FALLBACK_HEIGHT } from "@/node-size";
import CanvasNode from "./DatabaseComponents/CanvasNode.vue";
import CanvasRecycleBinPanel from "./DatabaseComponents/CanvasRecycleBinPanel.vue";
import NameInputDialog from "@/components/NameInputDialog.vue";
import EditCanvasNodeColorDialog from "@/node-colors/EditCanvasNodeColorDialog.vue";
// #if [DEBUG]
import ViewportDebugOverlay from "./DatabaseComponents/ViewportDebugOverlay.vue";
// #endif

const nodes: Ref<VFNode[]> = ref([]);
const edges: Ref<VFEdge[]> = ref([]);
const canvases: Ref<Canvas[]> = ref([]);
const loaded = ref(false);
const containerRef = ref<HTMLElement>();
const viewport = useViewportPersistence(null, containerRef);
const recycleBin = useCanvasRecycleBin();
const recycleBinPanelRef = ref<InstanceType<typeof CanvasRecycleBinPanel>>();
const recycleBinBtnRef = ref<any>(null);
const nameInputDialogRef = ref<InstanceType<typeof NameInputDialog>>();
const nodeColorDialogRef = ref<InstanceType<typeof EditCanvasNodeColorDialog>>();
const { screenToFlowCoordinate, updateNodeInternals, getNodes: getVFNodes, getEdges: getVFEdges, viewport: vfViewport } = useVueFlow();
const snapGrid: [number, number] = [20, 20];
const { isLayouting, applyAutoLayout } = useAutoLayout({
  // 节点/边必须取自 vue-flow store（GraphNode 的 position 是响应式活数据）：
  // 拖拽后 vue-flow 会替换 store 节点的 position 对象，本组件的 nodes 数组中已是过期坐标。
  getNodes: () => getVFNodes.value,
  getEdges: () => getVFEdges.value,
  persist: userDatabaseCanvasMoveCanvases,
  snapGrid,
  fallbackSize: { width: CANVAS_NODE_FALLBACK_WIDTH, height: CANVAS_NODE_FALLBACK_HEIGHT },
  // 自动布局动画结束、persist 之前把新坐标整体替换回父组件 nodes.position，
  // 避免后续增删画布时被 vue-flow 的 parseNode 用 props 中的旧坐标回滚 store。
  onNodesMoved: (items) => {
    const map = new Map(items.map((i) => [i.id, i]));
    for (const node of nodes.value) {
      const m = map.get(node.id);
      if (!m) continue;
      node.position = { x: m.x, y: m.y };
    }
  },
});

/** 加载正常画布列表并重建节点和边（画布宇宙的边由 parent_id 派生） */
async function loadCanvases(): Promise<void> {
  try {
    canvases.value = await userDatabaseCanvasList(false);
  } catch (e) {
    snackbarErrorCode(e);
    return;
  }
  nodes.value = canvases.value.map((c) => ({
    id: c.id,
    type: "canvas-node",
    position: { x: c.x, y: c.y },
    data: { name: c.name, canvasId: c.id, isRoot: c.parent_id === null, color: c.color },
  }));
  edges.value = canvases.value
    .filter((c) => c.parent_id !== null)
    .map((c) => ({
      id: `${c.parent_id}->${c.id}`,
      source: c.parent_id as string,
      target: c.id,
      sourceHandle: "source-right",
      targetHandle: "target-left",
      markerEnd: { type: MarkerType.ArrowClosed },
      selectable: false,
      deletable: false,
      focusable: false,
    }));
}

onMounted(async () => {
  await Promise.allSettled([loadCanvases(), viewport.load(), recycleBin.load()]);
  loaded.value = true;
});

onUnmounted(() => {
  viewport.flush();
});

/**
 * 已删除画布 id → 祖先路径文本（面板 tooltip）。
 * 合并正常与已删画布构建 id 映射，沿 parent_id 上溯拼接名称；
 * 根画布显示为本地化"根画布"文案，上溯遇缺失祖先停止，visited 集合防环。
 */
const paths = computed<Record<string, string>>(() => {
  const byId = new Map<string, Canvas>();
  for (const c of [...canvases.value, ...recycleBin.deletedCanvases.value]) {
    byId.set(c.id, c);
  }
  const result: Record<string, string> = {};
  for (const c of recycleBin.deletedCanvases.value) {
    const names: string[] = [];
    const visited = new Set<string>([c.id]);
    let parentId = c.parent_id;
    while (parentId !== null && !visited.has(parentId)) {
      const parent = byId.get(parentId);
      if (!parent) break;
      names.unshift(parent.parent_id === null ? t("database.canvas.root-canvas") : parent.name);
      visited.add(parent.id);
      parentId = parent.parent_id;
    }
    if (names.length > 0) result[c.id] = names.join(" / ");
  }
  return result;
});

/**
 * 打开画布自定义颜色对话框，保存成功后更新本地画布颜色并同步 vf node data（持久化由对话框内部完成）。
 * @param id 画布 id
 * @returns 无返回值
 */
async function onCanvasColor(id: string): Promise<void> {
  const canvas = canvases.value.find((c) => c.id === id);
  if (!canvas) return;
  const dialog = nodeColorDialogRef.value;
  if (!dialog) return;
  const result = await dialog.open(
    id,
    canvas.parent_id === null
      ? t("database.canvas.root-canvas")
      : canvas.name,
    canvas.color,
  );
  if (result === null) return;
  canvas.color = result;
  const vfNode = nodes.value.find((n) => n.id === id);
  if (vfNode) vfNode.data.color = result;
}

/** 重命名画布：弹出名称输入对话框，确认后调 API 并整体重载正常列表 */
async function onCanvasRename(id: string) {
  const canvas = canvases.value.find((c) => c.id === id);
  if (!canvas) return;
  const newName = await nameInputDialogRef.value?.open({
    title: t("database.canvas-universe.rename-canvas"),
    label: t("database.canvas-universe.canvas-name-label"),
    initialValue: canvas.name,
  });
  if (!newName) return;
  try {
    await userDatabaseCanvasRename(id, newName);
  } catch (e) {
    snackbarErrorCode(e);
    return;
  }
  await loadCanvases();
}

/** 逻辑删除画布：API 成功后播放飞向回收站动画，随后整体重载正常列表消化子树级联 */
async function onCanvasLogicalDelete(id: string) {
  if (!nodes.value.some((n) => n.id === id)) return;
  const ok = await recycleBin.logicalDelete(id);
  if (!ok) return;
  const btnEl = recycleBinBtnRef.value?.$el as HTMLElement | undefined;
  if (btnEl) {
    flyToRecycleBin(id, btnEl, ".canvas-node-card", ".canvas-node-actions");
  }
  await loadCanvases();
}

/** 恢复画布：API 成功后重载正常列表并播放入场动画，完成后校正 handle 测量 */
async function onCanvasRestore(canvas: Canvas, position?: { x: number; y: number }) {
  const pos = position ?? { x: canvas.x, y: canvas.y };
  const ok = await recycleBin.restore(canvas, pos.x, pos.y);
  if (!ok) return;
  await loadCanvases();
  await fadeInNode(canvas.id);
  updateNodeInternals([canvas.id]);
}

/** 节点右键：阻止浏览器原生菜单（右键平移视口后松开在节点上也会触发） */
function onNodeContextMenu({ event }: { event: MouseEvent | TouchEvent }) {
  if (!(event instanceof MouseEvent)) return;
  event.preventDefault();
}

function onDragOver(event: DragEvent) {
  event.preventDefault();
  if (event.dataTransfer) {
    event.dataTransfer.dropEffect = "move";
  }
}

function onDrop(event: DragEvent) {
  const id = event.dataTransfer?.getData("application/x-inet-recycle-canvas");
  if (!id) return;
  const canvas = recycleBin.deletedCanvases.value.find((c) => c.id === id);
  if (!canvas) return;
  const pos = screenToFlowCoordinate({ x: event.clientX, y: event.clientY });
  onCanvasRestore(canvas, pos);
}

/**
 * 画布节点拖拽停止回调（含多选拖拽）。
 *
 * vue-flow store 已持有每个被拖动节点的最终 position；此处把每个新 position 整体替换回
 * 父组件 nodes.value 中对应节点，避免后续 filter / push 操作触发 parseNode 把 store 中
 * 已拖动位置覆盖回 props 的初始坐标。
 * 持久化：单节点 userDatabaseCanvasMoveCanvas；多节点 userDatabaseCanvasMoveCanvases。
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
    userDatabaseCanvasMoveCanvas(items[0].id, items[0].x, items[0].y).catch(snackbarErrorCode);
  } else if (items.length > 1) {
    userDatabaseCanvasMoveCanvases(items).catch(snackbarErrorCode);
  }
}
</script>

<template>
  <div ref="containerRef" class="canvas-universe">
    <div v-if="loaded" class="canvas-floating-menu frosted-glass">
      <VBtn
        icon="mdi-auto-fix"
        size="small"
        variant="text"
        :title="t('database.canvas-universe.auto-layout')"
        :disabled="isLayouting"
        @click="applyAutoLayout"
      />
      <VBadge
        :content="recycleBin.deletedCanvases.value.length"
        :model-value="recycleBin.deletedCanvases.value.length > 0"
        color="error"
        offset-x="4"
        offset-y="4"
      >
        <VBtn
          ref="recycleBinBtnRef"
          icon="mdi-trash-can-outline"
          size="small"
          variant="text"
          :title="t('database.canvas-universe.recycle-bin')"
          @click="recycleBinPanelRef?.toggle()"
        />
      </VBadge>
    </div>
    <div v-if="!loaded" class="canvas-universe-loading">
      <VProgressCircular indeterminate color="primary" />
    </div>
    <div v-else-if="nodes.length === 0" class="canvas-universe-empty">
      <VIcon icon="mdi-vector-square" size="48" />
      <p class="text-body-1">{{ t("database.canvas-universe.empty") }}</p>
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
      @node-drag-stop="onNodeDragStop"
      @node-context-menu="onNodeContextMenu"
      @viewport-change="viewport.save"
      @drop="onDrop"
      @dragover="onDragOver"
    >
      <Background pattern="dots" :gap="20" :size="1" />
      <Controls class="theme-controls frosted-glass" />
      <template #node-canvas-node="{ id, data, selected }">
        <CanvasNode :id="id" :data="data" :selected="selected" @delete="onCanvasLogicalDelete" @rename="onCanvasRename" @color="onCanvasColor" />
      </template>
    </VueFlow>
    <CanvasRecycleBinPanel
      ref="recycleBinPanelRef"
      :canvases="recycleBin.deletedCanvases.value"
      :paths="paths"
      @restore="onCanvasRestore"
      @physical-delete="recycleBin.physicalDelete"
      @empty="recycleBin.empty"
    />
    <NameInputDialog ref="nameInputDialogRef" />
    <EditCanvasNodeColorDialog ref="nodeColorDialogRef" />
    <!-- #if [DEBUG] -->
    <ViewportDebugOverlay :viewport="vfViewport" />
    <!-- #endif -->
  </div>
</template>

<style lang="scss" scoped>
.canvas-universe {
  width: 100%;
  height: 100%;
  position: relative;
}

.canvas-universe-loading {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 100%;
  height: 100%;
}

.canvas-universe-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  width: 100%;
  height: 100%;
  color: rgb(var(--v-theme-on-surface));
  opacity: 0.5;
  gap: 0.5rem;
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
</style>
