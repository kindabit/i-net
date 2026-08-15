<!--
  模板面板组件。

  悬浮在画布左上角悬浮菜单右侧，展示空白节点与可复用的节点模板列表。
  支持将空白节点或模板拖拽到画布上创建节点，并提供"管理模板"入口。
  通过 defineExpose 暴露 open / close / toggle / visible 方法。
-->
<script setup lang="ts">
import { ref } from "vue";
import { t } from "@/i18n";
import { userDatabaseTemplateList } from "@/api";
import type { Template } from "@/api-types";
import { snackbarErrorCode } from "@/composables/use-snackbar";

const emit = defineEmits<{
  openTemplateManager: [];
}>();

const visible = ref(false);
const templates = ref<Template[]>([]);

/**
 * 从后端加载当前数据库的节点模板列表并更新面板数据。
 * 无输入参数，无返回值；加载失败时通过全局 snackbar 展示错误。
 */
async function loadTemplates() {
  try {
    templates.value = await userDatabaseTemplateList();
  } catch (e) {
    snackbarErrorCode(e);
  }
}

/**
 * 打开模板面板并加载模板列表。
 * 无输入参数，无返回值。
 */
function open() {
  visible.value = true;
  loadTemplates();
}

/**
 * 关闭模板面板。
 * 无输入参数，无返回值。
 */
function close() {
  visible.value = false;
}

/**
 * 切换模板面板的显示状态；展开时重新加载模板列表。
 * 无输入参数，无返回值。
 */
function toggle() {
  visible.value = !visible.value;
  if (visible.value) {
    loadTemplates();
  }
}

/**
 * 拖拽开始时向 DataTransfer 写入模板信息，并设置拖拽预览图。
 * @param event 拖拽事件对象
 * @param templateId 模板 ID（"blank" 表示空白节点）
 * @param name 模板名称，作为新节点的副标题
 * @param createCanvas 是否创建画布节点
 */
function onDragStart(event: DragEvent, templateId: string, name: string, createCanvas: boolean) {
  event.dataTransfer!.setData("application/x-i-net-template", templateId);
  event.dataTransfer!.setData("application/x-i-net-template-name", name);
  if (createCanvas) {
    event.dataTransfer!.setData("application/x-i-net-create-canvas", "true");
  }
  event.dataTransfer!.effectAllowed = "copy";
  setDragImage(event);
}

/**
 * 将整行面板项设为拖拽预览图，并按鼠标在行内的偏移定位预览。
 * @param event 拖拽事件对象
 */
function setDragImage(event: DragEvent) {
  const rowEl = (event.currentTarget as HTMLElement).closest(".panel-item") as HTMLElement | null;
  if (!rowEl) return;
  const rect = rowEl.getBoundingClientRect();
  const offsetX = event.clientX - rect.left;
  const offsetY = event.clientY - rect.top;
  event.dataTransfer!.setDragImage(rowEl, offsetX, offsetY);
}

/**
 * 通知父组件打开模板管理对话框，并关闭当前面板。
 * 无输入参数，无返回值。
 */
function onManageTemplates() {
  emit("openTemplateManager");
  close();
}

defineExpose({
  open,
  close,
  toggle,
  visible,
});
</script>

<template>
  <Transition name="template-panel">
    <div
      v-if="visible"
      v-click-outside="close"
      class="node-template-panel frosted-glass"
    >
      <div class="node-template-header">
        <span class="node-template-title">
          {{ t("database.canvas.new-node") }}
        </span>
        <div class="node-template-actions">
          <VBtn
            icon="mdi-cog-outline"
            size="x-small"
            variant="text"
            density="comfortable"
            :title="t('database.canvas.manage-templates')"
            @click="onManageTemplates"
          />
          <VBtn
            icon="mdi-close"
            size="x-small"
            variant="text"
            density="comfortable"
            @click="close"
          />
        </div>
      </div>
      <div class="node-template-list">
        <div class="panel-item">
          <VIcon icon="mdi-plus-circle-outline" />
          <span>{{ t("database.canvas.blank-node") }}</span>
          <div class="drag-handles">
            <VIcon
              icon="mdi-file-outline"
              class="drag-handle"
              :title="t('database.canvas.drag-create-node')"
              draggable="true"
              @dragstart="($event: DragEvent) => onDragStart($event, 'blank', t('database.canvas.blank-node'), false)"
            />
            <VIcon
              icon="mdi-vector-square"
              class="drag-handle"
              :title="t('database.canvas.drag-create-canvas-node')"
              draggable="true"
              @dragstart="($event: DragEvent) => onDragStart($event, 'blank', t('database.canvas.blank-node'), true)"
            />
          </div>
        </div>
        <div
          v-for="tpl in templates"
          :key="tpl.id"
          class="panel-item"
        >
          <VIcon icon="mdi-clipboard-text-outline" />
          <span>{{ tpl.name }}</span>
          <div class="drag-handles">
            <VIcon
              icon="mdi-file-outline"
              class="drag-handle"
              :title="t('database.canvas.drag-create-node')"
              draggable="true"
              @dragstart="($event: DragEvent) => onDragStart($event, tpl.id, tpl.name, false)"
            />
            <VIcon
              icon="mdi-vector-square"
              class="drag-handle"
              :title="t('database.canvas.drag-create-canvas-node')"
              draggable="true"
              @dragstart="($event: DragEvent) => onDragStart($event, tpl.id, tpl.name, true)"
            />
          </div>
        </div>
      </div>
    </div>
  </Transition>
</template>

<style lang="scss" scoped>
.node-template-panel {
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

.node-template-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0.5rem 0.75rem;
  flex-shrink: 0;
}

.node-template-title {
  font-size: 0.875rem;
  font-weight: 500;
  white-space: nowrap;
}

.node-template-actions {
  display: flex;
  align-items: center;
}

.node-template-list {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 0.25rem;
}

.panel-item {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.375rem 0.75rem;
  border-radius: 0.25rem;

  &:hover {
    background: rgba(var(--v-theme-on-surface), 0.06);
  }
}

.drag-handles {
  display: flex;
  gap: 0.25rem;
  margin-left: auto;
}

.drag-handle {
  cursor: grab;
  opacity: 0.55;

  &:hover {
    opacity: 1;
  }
}

/* 面板展开/收起动画 */
.template-panel-enter-active,
.template-panel-leave-active {
  transition: opacity 0.2s ease, transform 0.2s ease;
  overflow: hidden;
}

.template-panel-enter-from,
.template-panel-leave-to {
  opacity: 0;
  transform: translateX(-0.5rem);
}
</style>
