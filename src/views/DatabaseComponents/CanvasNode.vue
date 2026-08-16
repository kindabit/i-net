<!--
  画布节点组件。

  在画布宇宙中将一个画布渲染为 vue-flow 节点，显示画布名称和图标。支持拖拽移动。
  根画布名称固定显示为本地化"根画布"文案，其余画布显示数据库中的名称。
  hover 时在节点顶部外侧显示操作按钮排（毛玻璃风格），所有画布均显示颜色按钮；
  非根画布额外显示重命名与逻辑删除按钮。
  支持画布自定义颜色：背景、边框、标题、图标、工具按钮五项。
-->
<script setup lang="ts">
import { ref, computed } from "vue";
import { Handle, Position, useNode } from "@vue-flow/core";
import { useRouter } from "vue-router";
import { t } from "@/i18n";
import { deserializeCanvasColor } from "@/node-colors";
import { currentThemeIsDark } from "@/themes";
// #if [DEBUG]
import NodeDebugOverlay from "./NodeDebugOverlay.vue";
// #endif

const router = useRouter();

const props = defineProps<{
  id: string;
  data: { name: string; canvasId: string; isRoot: boolean; color: string };
  selected: boolean;
}>();

const emit = defineEmits<{ delete: [id: string]; rename: [id: string]; color: [id: string] }>();

/** 当前节点对象（响应式，含 position）。替代已 deprecated 的 slot position 属性。DEBUG 关闭时模板不引用，无副作用。 */
const { node } = useNode();

const actionsVisible = ref(false);

/** 节点显示名称：根画布固定为本地化文案，其余画布显示数据库名称 */
const displayName = computed(() =>
  props.data.isRoot ? t("database.canvas.root-canvas") : props.data.name,
);

/** 解析当前主题下的自定义颜色属性（键缺失即默认值，外观交还组件 CSS 兜底） */
const colors = computed(() => {
  const scheme = deserializeCanvasColor(props.data.color);
  return currentThemeIsDark.value ? scheme.dark : scheme.light;
});

function onDblClick() {
  router.push({ name: "canvas", params: { canvasId: props.data.canvasId } });
}
</script>

<template>
  <div
    class="canvas-node-card"
    :class="{ 'canvas-node-card--selected': selected }"
    :draggable="false"
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
      <div v-if="actionsVisible" class="canvas-node-actions frosted-glass">
        <VBtn
          v-if="!data.isRoot"
          icon="mdi-pencil-outline"
          size="x-small"
          variant="text"
          density="comfortable"
          :title="t('database.canvas-universe.rename-canvas')"
          :style="{ color: colors.action }"
          @click.stop="emit('rename', props.id)"
          @pointerdown.stop
          @mousedown.stop
          @dblclick.stop
        />
        <VBtn
          icon="mdi-palette-outline"
          size="x-small"
          variant="text"
          density="comfortable"
          :title="t('database.canvas-universe.customize-color')"
          :style="{ color: colors.action }"
          @click.stop="emit('color', props.id)"
          @pointerdown.stop
          @mousedown.stop
          @dblclick.stop
        />
        <VBtn
          v-if="!data.isRoot"
          icon="mdi-delete-outline"
          size="x-small"
          variant="text"
          density="comfortable"
          :title="t('database.canvas-universe.delete-canvas')"
          :style="{ color: colors.action }"
          @click.stop="emit('delete', props.id)"
          @pointerdown.stop
          @mousedown.stop
          @dblclick.stop
        />
      </div>
    </Transition>
    <Handle type="target" :position="Position.Left" id="target-left" :connectable="false" style="opacity: 0" />
    <Handle type="source" :position="Position.Right" id="source-right" :connectable="false" style="opacity: 0" />
    <!-- #if [DEBUG] -->
    <NodeDebugOverlay v-if="actionsVisible" :x="node.position.x" :y="node.position.y" />
    <!-- #endif -->
    <VIcon icon="mdi-vector-square" size="18" class="canvas-node-icon" :style="{ color: colors.icon }" />
    <span class="canvas-node-name">{{ displayName }}</span>
  </div>
</template>

<style lang="scss" scoped>
.canvas-node-card {
  position: relative;
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.625rem 1rem;
  border-radius: 0.75rem;
  background-color: rgb(var(--v-theme-surface));
  color: rgb(var(--v-theme-on-surface));
  border: 0.125rem solid transparent;
  box-shadow: 0 0.125rem 0.5rem rgba(0, 0, 0, 0.12);
  cursor: grab;
  user-select: none;
  transition: border-color 0.2s, box-shadow 0.2s;
  min-width: 7.5rem;
  white-space: nowrap;

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

.canvas-node-icon {
  opacity: 0.6;
  flex-shrink: 0;
  color: rgb(var(--v-theme-on-surface));
}

.canvas-node-name {
  font-size: 0.875rem;
  font-weight: 500;
  overflow: hidden;
  text-overflow: ellipsis;
}

.canvas-node-actions {
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
.vue-flow__node:has(.canvas-node-card:hover) {
  z-index: 10 !important;
}
</style>
