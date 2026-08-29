<!--
  画布回收站面板组件。

  在画布宇宙左上角悬浮菜单右侧显示已逻辑删除的画布列表。
  通过 props 接收画布列表和路径映射，通过 emit 事件向父组件通知用户操作。
  通过 defineExpose 暴露 open / close / toggle / visible 方法。
  面板内部集成 ConfirmDialog 进行物理删除和清空操作的二次确认。
-->
<script setup lang="ts">
import { ref, computed } from "vue";
import { t } from "@/i18n";
import type { Canvas } from "@/api-types";
import ConfirmDialog from "@/components/ConfirmDialog.vue";

const props = defineProps<{
  canvases: Canvas[];
  /** 画布 id → 祖先路径文本（如 "父画布A / 父画布B"），无路径的画布无对应键 */
  paths: Record<string, string>;
}>();

const emit = defineEmits<{
  restore: [canvas: Canvas];
  physicalDelete: [canvas: Canvas];
  empty: [];
}>();

const visible = ref(false);
const confirmDialogRef = ref<InstanceType<typeof ConfirmDialog>>();
const draggingId = ref<string | null>(null);

function open() {
  visible.value = true;
}

function close() {
  visible.value = false;
}

function toggle() {
  visible.value = !visible.value;
}

function onDragStart(event: DragEvent, canvas: Canvas) {
  event.dataTransfer?.setData("application/x-inet-recycle-canvas", canvas.id);
  if (event.dataTransfer) event.dataTransfer.effectAllowed = "move";
  draggingId.value = canvas.id;
}

function onDragEnd() {
  draggingId.value = null;
}

async function onPhysicalDelete(canvas: Canvas) {
  const confirmed = await confirmDialogRef.value?.open({
    title: t("database.canvas-universe.physical-delete-title"),
    text: t("database.canvas-universe.physical-delete-text", {
      name: canvas.name,
    }),
    confirmColor: "error",
  });
  if (confirmed) emit("physicalDelete", canvas);
}

async function onEmpty() {
  const confirmed = await confirmDialogRef.value?.open({
    title: t("database.canvas-universe.empty-recycle-bin-title"),
    text: t("database.canvas-universe.empty-recycle-bin-text", {
      count: props.canvases.length,
    }),
    confirmColor: "error",
  });
  if (confirmed) emit("empty");
}

defineExpose({
  open,
  close,
  toggle,
  visible: computed(() => visible.value),
});
</script>

<template>
  <Transition name="recycle-panel">
    <div
      v-if="visible"
      v-click-outside="close"
      class="recycle-bin-panel frosted-glass"
    >
      <div class="recycle-bin-header">
        <span class="recycle-bin-title">
          {{ t("database.canvas-universe.recycle-bin") }}
        </span>
        <VBtn
          icon="mdi-close"
          size="x-small"
          variant="text"
          density="comfortable"
          @click="close"
        />
      </div>
      <div class="recycle-bin-list">
        <TransitionGroup
          name="recycle-item"
          tag="div"
          class="recycle-bin-list-inner"
        >
          <div
            v-for="canvas in canvases"
            :key="canvas.id"
            class="recycle-bin-item"
            :class="{ 'recycle-bin-item--dragging': draggingId === canvas.id }"
            draggable="true"
            @dragstart="onDragStart($event, canvas)"
            @dragend="onDragEnd"
          >
            <div class="recycle-bin-item-content">
              <VIcon
                icon="mdi-vector-square"
                size="16"
                class="recycle-bin-item-icon"
              />
              <div class="recycle-bin-item-text">
                <div class="recycle-bin-item-title">{{ canvas.name }}</div>
              </div>
            </div>
            <div class="recycle-bin-item-actions">
              <VBtn
                icon="mdi-restore"
                size="x-small"
                variant="text"
                density="comfortable"
                :title="t('database.canvas-universe.restore-canvas')"
                @click="emit('restore', canvas)"
              />
              <VBtn
                icon="mdi-delete-forever-outline"
                size="x-small"
                variant="text"
                density="comfortable"
                :title="t('database.canvas-universe.physical-delete-canvas')"
                @click="onPhysicalDelete(canvas)"
              />
            </div>
            <VTooltip
              v-if="paths[canvas.id]"
              activator="parent"
              location="top"
            >
              {{ paths[canvas.id] }}
            </VTooltip>
          </div>
        </TransitionGroup>
        <Transition name="recycle-empty">
          <div v-if="canvases.length === 0" class="recycle-bin-empty">
            {{ t("database.canvas-universe.recycle-bin-empty") }}
          </div>
        </Transition>
      </div>
      <div class="recycle-bin-footer">
        <VBtn
          prepend-icon="mdi-delete-sweep-outline"
          size="small"
          variant="text"
          block
          :disabled="canvases.length === 0"
          @click="onEmpty"
        >
          {{ t("database.canvas-universe.empty-recycle-bin") }}
        </VBtn>
      </div>
    </div>
  </Transition>
  <ConfirmDialog ref="confirmDialogRef" />
</template>

<style lang="scss" scoped>
.recycle-bin-panel {
  position: absolute;
  top: 0.75rem;
  left: 4.25rem;
  z-index: 10;
  width: 16.25rem;
  max-height: 60vh;
  display: flex;
  flex-direction: column;
  border-radius: 0.5rem;
  overflow: hidden;
}

.recycle-bin-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0.5rem 0.75rem;
  flex-shrink: 0;
}

.recycle-bin-title {
  font-size: 0.875rem;
  font-weight: 500;
  white-space: nowrap;
}

.recycle-bin-list {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  position: relative;
}

.recycle-bin-list-inner {
  position: relative;
}

.recycle-bin-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0.5rem 0.75rem;
  gap: 0.5rem;
}

.recycle-bin-item--dragging {
  opacity: 0.4;
}

.recycle-bin-item-content {
  display: flex;
  align-items: center;
  gap: 0.375rem;
  overflow: hidden;
  flex: 1;
  min-width: 0;
}

.recycle-bin-item-icon {
  opacity: 0.5;
  flex-shrink: 0;
}

.recycle-bin-item-text {
  overflow: hidden;
  flex: 1;
  min-width: 0;
}

.recycle-bin-item-title {
  font-size: 0.8rem;
  font-weight: 500;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.recycle-bin-item-subtitle {
  font-size: 0.7rem;
  opacity: 0.5;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.recycle-bin-item-actions {
  display: flex;
  gap: 0.125rem;
  flex-shrink: 0;
}

.recycle-bin-empty {
  text-align: center;
  padding: 1.5rem 0.75rem;
  opacity: 0.5;
  font-size: 0.8rem;
}

.recycle-bin-footer {
  flex-shrink: 0;
  padding: 0.375rem 0.5rem;
}

/* 面板展开/收起动画 */
.recycle-panel-enter-active,
.recycle-panel-leave-active {
  transition: opacity 0.25s ease, transform 0.25s ease, max-width 0.25s ease;
  overflow: hidden;
}

.recycle-panel-enter-from,
.recycle-panel-leave-to {
  opacity: 0;
  transform: translateX(-0.5rem);
  max-width: 0;
}

.recycle-panel-enter-to,
.recycle-panel-leave-from {
  max-width: 16.25rem;
}

/* 列表项动画 */
.recycle-item-enter-active,
.recycle-item-leave-active {
  transition: opacity 0.25s ease, max-height 0.25s ease;
  overflow: hidden;
}

.recycle-item-enter-from,
.recycle-item-leave-to {
  opacity: 0;
  max-height: 0;
}

.recycle-item-enter-to,
.recycle-item-leave-from {
  max-height: 4rem;
}

.recycle-item-leave-active {
  position: absolute;
  width: 100%;
}

.recycle-item-move {
  transition: transform 0.25s ease;
}

/* 空态文案淡入淡出 */
.recycle-empty-enter-active,
.recycle-empty-leave-active {
  transition: opacity 0.2s ease;
}

.recycle-empty-enter-from,
.recycle-empty-leave-to {
  opacity: 0;
}
</style>
