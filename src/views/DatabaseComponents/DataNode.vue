<!--
  数据节点 / 画布节点组件。

  在画布中将一个敏感数据节点或子画布节点渲染为 vue-flow 节点。
  显示节点标题和副标题，canvasId 非 null 时显示画布图标；双击普通节点打开编辑对话框，双击画布节点进入子画布。
  四个方向均为出口（source）。
  节点为固定宽高（160×80，背景网格 20px 的整数倍），标题/副标题过长时显示省略号。
  hover 时在节点顶部外侧显示操作按钮排（毛玻璃风格），包含编辑、附件、自定义颜色与逻辑删除按钮。
  支持节点自定义颜色：背景、边框、标题、副标题、图标、handle、悬浮按钮均可单独配色。
-->
<script setup lang="ts">
import { ref, computed } from "vue";
import { Handle, Position } from "@vue-flow/core";
import { useRouter } from "vue-router";
import { t } from "@/i18n";
import type { DataNodeData } from "@/vf-convert";
import { setCanvasNavIntent } from "./canvas-route-transition";
import { deserializeNodeColor } from "@/node-colors";
import { currentThemeIsDark } from "@/themes";

const router = useRouter();

const props = defineProps<{
  id: string;
  data: DataNodeData;
  selected: boolean;
}>();

const emit = defineEmits<{ delete: [id: string]; edit: [id: string]; attachment: [id: string]; color: [id: string] }>();

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

function onDblClick() {
  // 普通节点双击打开编辑对话框，画布节点双击进入子画布
  if (props.data.canvasId === null) {
    emit("edit", props.id);
    return;
  }
  setCanvasNavIntent("drill-in");
  router.push({ name: "canvas", params: { canvasId: props.data.canvasId } });
}
</script>

<template>
  <div
    class="data-node-card"
    :class="{ 'data-node-card--selected': selected }"
    :style="{
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
    <Handle type="source" :position="Position.Top" id="top" :style="handleStyle" />
    <Handle type="source" :position="Position.Bottom" id="bottom" :style="handleStyle" />
    <Handle type="source" :position="Position.Left" id="left" :style="handleStyle" />
    <Handle type="source" :position="Position.Right" id="right" :style="handleStyle" />
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
  width: 10rem;
  height: 5rem;

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
</style>

<style>
.vue-flow__node:has(.data-node-card:hover) {
  z-index: 10 !important;
}
</style>
