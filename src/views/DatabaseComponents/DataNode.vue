<!--
  数据节点 / 画布节点组件。

  在画布中将一个敏感数据节点或子画布节点渲染为 vue-flow 节点。
  显示节点标题和副标题，canvasId 非 null 时显示画布图标；双击普通节点打开编辑对话框，双击画布节点进入子画布。
  四个方向均为出口（source）。
  节点为固定宽高（尺寸常量见 node-size.ts，为吸附网格 20px 的整数倍），标题/副标题过长时显示省略号。
  hover 时在节点顶部外侧显示操作按钮排（毛玻璃风格）；普通节点包含编辑、复制、附件、自定义颜色与逻辑删除五个按钮，
  影子节点只显示编辑按钮，画布节点不显示复制按钮。
  支持节点自定义颜色：背景、边框、标题、副标题、图标、handle、悬浮按钮均可单独配色。
  在跨画布迁移（按住 Alt 拖拽）时根据节点集合法性显示"允许/禁止"落点光环。

  影子节点（data.shadowId 非 null）的渲染差异：
  - 边框使用虚线，整体略降透明度，提示其为对画布外原始节点的引用。
  - 当 data.shadowDirection 非 null 时，在节点对应外侧渲染一条带行进动画的虚拟边，
    表示节点的某一度数指向画布之外（inflow：入度来自画布之外；outflow：出度指向画布之外）；
    点击该虚拟边可快捷跳转至父画布并尽量定位原始节点。
  - 当 data.shadowOriginDeleted 为 true 时，卡片灰化并显示删除图标提示原始节点已在回收站中。
  - data.canvasId 对影子节点恒为 null（影子的原始节点只能是普通节点），
    因此双击影子节点走 canvasId === null 分支（打开编辑对话框）。
-->
<script setup lang="ts">
import { ref, computed } from "vue";
import { Handle, Position, useNode } from "@vue-flow/core";
import { useRoute, useRouter } from "vue-router";
import { isString } from "lodash";
import { t } from "@/i18n";
import { userDatabaseCanvasList } from "@/api";
import { snackbarErrorCode } from "@/composables/use-snackbar";
import type { DataNodeData } from "@/vf-convert";
import nodeMoveAndRelocate from "@/composables/use-node-move-and-relocate";
import { setCanvasNavIntent } from "./canvas-route-transition";
import { deserializeNodeColor } from "@/node-colors";
import { DATA_NODE_WIDTH_REM, DATA_NODE_HEIGHT_REM } from "@/node-size";
import { currentThemeIsDark } from "@/themes";
// #if [DEBUG]
import NodeDebugOverlay from "./NodeDebugOverlay.vue";
// #endif

const router = useRouter();
const route = useRoute();

const props = defineProps<{
  id: string;
  data: DataNodeData;
  selected: boolean;
}>();

const emit = defineEmits<{ delete: [id: string]; edit: [id: string]; copy: [id: string]; attachment: [id: string]; color: [id: string] }>();

/** 当前节点对象（响应式，含 position）。替代已 deprecated 的 slot position 属性。DEBUG 关闭时模板不引用，无副作用。 */
const { node } = useNode();

// 控制 hover 时操作按钮排的显隐
const actionsVisible = ref(false);

/** 解析当前主题下的自定义颜色属性（键缺失即默认值，外观交还组件 CSS 兜底） */
const colors = computed(() => {
  const scheme = deserializeNodeColor(props.data.color);
  return currentThemeIsDark.value ? scheme.dark : scheme.light;
});

/** handle 自定义颜色样式：无自定义色时不传内联样式，保留组件默认外观 */
const handleStyle = computed(() => {
  const handle = colors.value.handle;
  return handle ? { background: handle, borderColor: handle } : undefined;
});

/** 迁移落点高亮状态：仅当本节点成为迁移目标（状态机只会以影子节点/画布节点为目标）且处于 relocate 模式时非 null */
const dropState = computed(() => {
  if (nodeMoveAndRelocate.mode.value !== "relocate") return null;
  const target = nodeMoveAndRelocate.relocatingTarget.value;
  if (!target || target.type === "breadcrumb-segment") return null;
  if (target.nodeId !== props.id) return null;
  return nodeMoveAndRelocate.nodeSetRelocatingLegality.value === "legal" ? "allow" : "forbid";
});

function onDblClick() {
  // 普通节点双击打开编辑对话框，画布节点双击进入子画布
  // 影子节点的 canvasId 恒为 null（影子的原始节点只能是普通节点），因此走 canvasId === null 分支（打开编辑对话框）
  if (props.data.canvasId === null) {
    emit("edit", props.id);
    return;
  }
  setCanvasNavIntent("drill-in");
  router.push({ name: "canvas", params: { canvasId: props.data.canvasId } });
}

/**
 * 点击影子节点虚拟边：跳转至当前画布的父画布，并尽量通过 nodeId 定位影子对应的原始节点。
 * 原始节点已逻辑删除或父画布不存在（数据不一致）时仅完成可确定的跳转或静默不跳转。
 * 输入：无。
 * 返回：无返回值。
 */
async function onShadowVirtualEdgeClick() {
  if (!props.data.shadowId || !props.data.shadowDirection) return;
  const canvasId = route.params.canvasId;
  if (!isString(canvasId) || canvasId === "") return;
  try {
    const canvases = await userDatabaseCanvasList(false);
    const current = canvases.find((c) => c.id === canvasId);
    if (!current || current.parent_id === null) return;
    setCanvasNavIntent("drill-out");
    await router.push({
      name: "canvas",
      params: { canvasId: current.parent_id },
      query: { nodeId: props.data.shadowId },
    });
  } catch (e) {
    snackbarErrorCode(e);
  }
}
</script>

<template>
  <div
    class="data-node-card"
    :class="{
      'data-node-card--selected': selected,
      'data-node-card--shadow': !!data.shadowId,
      'data-node-card--origin-deleted': !!data.shadowOriginDeleted,
      'data-node-card--drop-allow': dropState === 'allow',
      'data-node-card--drop-forbid': dropState === 'forbid',
    }"
    :style="{
      width: DATA_NODE_WIDTH_REM,
      height: DATA_NODE_HEIGHT_REM,
      backgroundColor: colors.background,
      color: colors.title,
      borderColor: selected
        ? colors.borderSelected
        : colors.borderUnselected,
    }"
    @dblclick="onDblClick"
    @mouseenter="actionsVisible = true"
    @mouseleave="actionsVisible = false"
  >
    <Transition name="node-actions">
      <div v-if="actionsVisible" class="data-node-actions frosted-glass">
        <VBtn
          icon="mdi-pencil-outline"
          size="x-small"
          variant="text"
          density="comfortable"
          :title="t('database.canvas.edit-node')"
          :style="{ color: colors.action }"
          @click.stop="emit('edit', props.id)"
          @pointerdown.stop
          @mousedown.stop
          @dblclick.stop
        />
        <VBtn
          v-if="!data.shadowId && !data.canvasId"
          icon="mdi-content-copy"
          size="x-small"
          variant="text"
          density="comfortable"
          :title="t('database.canvas.copy-node')"
          :style="{ color: colors.action }"
          @click.stop="emit('copy', props.id)"
          @pointerdown.stop
          @mousedown.stop
          @dblclick.stop
        />
        <VBtn
          v-if="!data.shadowId"
          icon="mdi-paperclip"
          size="x-small"
          variant="text"
          density="comfortable"
          :title="t('database.canvas.attachment.manage')"
          :style="{ color: colors.action }"
          @click.stop="emit('attachment', props.id)"
          @pointerdown.stop
          @mousedown.stop
          @dblclick.stop
        />
        <VBtn
          v-if="!data.shadowId"
          icon="mdi-palette-outline"
          size="x-small"
          variant="text"
          density="comfortable"
          :title="t('database.canvas.customize-color')"
          :style="{ color: colors.action }"
          @click.stop="emit('color', props.id)"
          @pointerdown.stop
          @mousedown.stop
          @dblclick.stop
        />
        <VBtn
          v-if="!data.shadowId"
          icon="mdi-delete-outline"
          size="x-small"
          variant="text"
          density="comfortable"
          :title="t('database.canvas.delete-node')"
          :style="{ color: colors.action }"
          @click.stop="emit('delete', props.id)"
          @pointerdown.stop
          @mousedown.stop
          @dblclick.stop
        />
      </div>
    </Transition>
    <div
      v-if="data.shadowDirection"
      class="shadow-virtual-edge"
      :class="`shadow-virtual-edge--${data.shadowDirection}`"
      :title="t(`database.canvas.shadow-${data.shadowDirection}-hint`)"
      @click.stop="onShadowVirtualEdgeClick"
      @pointerdown.stop
      @mousedown.stop
      @dblclick.stop
    >
      <svg viewBox="0 0 40 10" width="2.5rem" height="0.625rem">
        <line x1="0" y1="5" x2="32" y2="5" class="shadow-virtual-edge-line" />
        <path d="M32 1 L40 5 L32 9 Z" class="shadow-virtual-edge-arrow" />
      </svg>
    </div>
    <VIcon
      v-if="data.shadowOriginDeleted"
      icon="mdi-delete-outline"
      size="x-small"
      class="shadow-origin-deleted-badge"
      :title="t('database.canvas.shadow-origin-deleted')"
    />
    <Handle type="source" :position="Position.Top" id="top" :style="handleStyle" />
    <Handle type="source" :position="Position.Bottom" id="bottom" :style="handleStyle" />
    <Handle type="source" :position="Position.Left" id="left" :style="handleStyle" />
    <Handle type="source" :position="Position.Right" id="right" :style="handleStyle" />
    <!-- #if [DEBUG] -->
    <NodeDebugOverlay v-if="actionsVisible" :x="node.position.x" :y="node.position.y" />
    <!-- #endif -->
    <div v-if="data.canvasId" class="data-node-title-row">
      <VIcon icon="mdi-vector-square" size="18" class="data-node-icon" :style="{ color: colors.icon }" />
      <div class="data-node-title">{{ data.title }}</div>
    </div>
    <div v-else class="data-node-title">{{ data.title }}</div>
    <div v-if="data.subTitle" class="data-node-subtitle" :style="{ color: colors.subtitle }">{{ data.subTitle }}</div>
  </div>
</template>

<style lang="scss" scoped>
.data-node-card {
  position: relative;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 0.25rem;
  padding: 0.75rem 1.25rem;
  border-radius: 0.5rem;
  background-color: rgb(var(--v-theme-surface));
  color: rgb(var(--v-theme-on-surface));
  border: 0.125rem solid transparent;
  box-shadow: 0 0.125rem 0.5rem rgba(0, 0, 0, 0.12);
  cursor: grab;
  user-select: none;
  transition: border-color 0.2s, box-shadow 0.2s;

  // 桥接节点与按钮排之间的间隙，避免鼠标移向按钮排时触发 mouseleave 收起按钮排
  &::before {
    content: "";
    position: absolute;
    bottom: 100%;
    left: 0;
    right: 0;
    height: 0.375rem;
  }

  &:hover {
    box-shadow: 0 0.25rem 1rem rgba(0, 0, 0, 0.18);
  }

  &--selected {
    border-color: rgb(var(--v-theme-primary));
    box-shadow: 0 0.25rem 1rem rgba(0, 0, 0, 0.2);
  }

  // 影子节点：虚线边框 + 略降透明度，dashed 与选中边框色正交
  &--shadow {
    border-style: dashed;
    opacity: 0.8;
  }

  // 影子节点的原始节点已被逻辑删除：灰化 + 进一步降透明度
  &--origin-deleted {
    filter: grayscale(1);
    opacity: 0.55;
  }

  // 迁移落点高亮：允许=success 光环，禁止=error 光环
  &--drop-allow {
    outline: 0.125rem solid rgb(var(--v-theme-success));
    outline-offset: 0.125rem;
  }

  &--drop-forbid {
    outline: 0.125rem solid rgb(var(--v-theme-error));
    outline-offset: 0.125rem;
  }
}

.data-node-icon {
  opacity: 0.6;
  flex-shrink: 0;
  color: rgb(var(--v-theme-on-surface));
}

.data-node-title-row {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 0.25rem;
  width: 100%;
}

.data-node-title {
  font-size: 0.875rem;
  font-weight: 500;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  max-width: 100%;
  min-width: 0;
}

.data-node-subtitle {
  font-size: 0.75rem;
  opacity: 0.6;
  color: rgb(var(--v-theme-on-surface));
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  max-width: 100%;
}

.data-node-actions {
  position: absolute;
  bottom: 100%;
  right: 0;
  margin-bottom: 0.375rem;
  display: flex;
  gap: 0.125rem;
  padding: 0.125rem;
  border-radius: 0.375rem;
  color: rgb(var(--v-theme-on-surface));
}

.node-actions-enter-active,
.node-actions-leave-active {
  transition: opacity 0.15s ease, transform 0.15s ease;
}

.node-actions-enter-from,
.node-actions-leave-to {
  opacity: 0;
  transform: translateY(-0.25rem);
}

/** 影子节点的虚拟边：表示指向画布之外的度数；可点击跳转父画布，位置在卡片外侧且不覆盖 Handle */
.shadow-virtual-edge {
  position: absolute;
  top: 50%;
  transform: translateY(-50%);
  pointer-events: auto;
  cursor: pointer;
  color: rgb(var(--v-theme-on-surface));
  opacity: 0.7;
  display: flex;
  align-items: center;
  transition: opacity 0.15s ease;

  &:hover {
    opacity: 1;
  }

  // inflow：入向影子，入度来自画布之外，渲染在节点左侧（right: 100%），箭头朝右进入节点
  &--inflow {
    right: 100%;
    margin-right: 0.125rem;
  }

  // outflow：出向影子，出度指向画布之外，渲染在节点右侧（left: 100%），箭头自然朝右离开节点
  &--outflow {
    left: 100%;
    margin-left: 0.125rem;
  }
}

/** 虚拟边线段：虚线 + 行进动画 */
.shadow-virtual-edge-line {
  stroke: currentColor;
  stroke-width: 1;
  stroke-dasharray: 0.25rem 0.125rem;
  fill: none;
  animation: shadow-virtual-edge-flow 0.6s linear infinite;
}

/** 虚拟边箭头 */
.shadow-virtual-edge-arrow {
  stroke: currentColor;
  stroke-width: 0.5;
  stroke-linejoin: round;
  fill: currentColor;
  fill-opacity: 0.8;
}

@keyframes shadow-virtual-edge-flow {
  from {
    stroke-dashoffset: 0.375rem;
  }
  to {
    stroke-dashoffset: 0;
  }
}

/** 影子节点原始节点已删除的角标：右上角绝对定位，不参与交互 */
.shadow-origin-deleted-badge {
  position: absolute;
  top: 0.125rem;
  right: 0.25rem;
  color: rgb(var(--v-theme-error));
  opacity: 0.85;
  pointer-events: none;
}
</style>

<style>
.vue-flow__node:has(.data-node-card:hover) {
  z-index: 10 !important;
}
</style>
