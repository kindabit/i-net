<!--
  节点附件管理对话框。

  管理单个节点的附件：导入、预览、导出、逻辑删除；回收站分区提供恢复与物理删除；
  无主附件文件（有文件无元数据）以警示区上报并由用户显式清理。
  所有操作即时生效并局部刷新，不单独触发保存，随数据库"保存并退出"统一持久化。
  通过 defineExpose 的 open() 打开。
-->
<script setup lang="ts">
import { ref } from "vue";
import { t, d } from "@/i18n";
import {
  userDatabaseAttachmentImport,
  userDatabaseAttachmentList,
  userDatabaseAttachmentExport,
  userDatabaseAttachmentLogicalDelete,
  userDatabaseAttachmentRestore,
  userDatabaseAttachmentPhysicalDelete,
  userDatabaseAttachmentListOrphanFiles,
  userDatabaseAttachmentRemoveOrphanFile,
  userDatabaseAttachmentSwapSortOrder,
} from "@/api";
import type { AttachmentVO } from "@/api-types";
import { snackbarErrorCode, snackbarText } from "@/composables/use-snackbar";
import ConfirmDialog from "@/components/ConfirmDialog.vue";
import AttachmentPreviewDialog from "./AttachmentPreviewDialog.vue";
import { formatSize } from "./attachment-types";

/** 对话框显示状态 */
const dialog = ref(false);
/** 当前节点 id */
const nodeId = ref("");
/** 当前节点标题 */
const nodeTitle = ref("");
/** 数据加载中 */
const loading = ref(false);
/** 导入进行中 */
const importing = ref(false);
/** 正常附件列表 */
const attachments = ref<AttachmentVO[]>([]);
/** 回收站（已逻辑删除）附件列表 */
const deletedAttachments = ref<AttachmentVO[]>([]);
/** 无主附件文件 id 列表 */
const orphanFiles = ref<string[]>([]);
/** 当前拖拽的源附件 id */
const draggingId = ref<string | null>(null);
const confirmDialogRef = ref<InstanceType<typeof ConfirmDialog>>();
const previewDialogRef = ref<InstanceType<typeof AttachmentPreviewDialog>>();

/**
 * 打开对话框并加载指定节点的附件数据。
 * @param id 节点 id
 * @param title 节点标题
 */
function open(id: string, title: string): void {
  nodeId.value = id;
  nodeTitle.value = title;
  dialog.value = true;
  void loadData();
}

/**
 * 加载正常附件、回收站附件与无主文件列表；打开对话框时及各操作成功后统一调用本函数刷新。
 * 无输入参数，无返回值；失败提示并关闭对话框。
 */
async function loadData(): Promise<void> {
  loading.value = true;
  try {
    const [normal, deleted, orphans] = await Promise.all([
      userDatabaseAttachmentList(nodeId.value, false),
      userDatabaseAttachmentList(nodeId.value, true),
      userDatabaseAttachmentListOrphanFiles(),
    ]);
    attachments.value = normal;
    deletedAttachments.value = deleted;
    orphanFiles.value = orphans;
  } catch (e) {
    snackbarErrorCode(e);
    dialog.value = false;
  } finally {
    loading.value = false;
  }
}

/**
 * 导入附件：由后端弹出系统文件选择对话框，导入成功后局部刷新；取消选择静默返回。
 * 无输入参数，无返回值。
 */
async function importAttachment(): Promise<void> {
  importing.value = true;
  try {
    const imported = await userDatabaseAttachmentImport(nodeId.value);
    if (imported === null) return;
    snackbarText(t("database.canvas.attachment.imported"), "success");
    await loadData();
  } catch (e) {
    snackbarErrorCode(e);
  } finally {
    importing.value = false;
  }
}

/**
 * 导出附件：由后端弹出系统保存对话框，导出成功后提示；取消选择静默返回。
 * @param attachment 目标附件
 */
async function exportAttachment(attachment: AttachmentVO): Promise<void> {
  try {
    const exported = await userDatabaseAttachmentExport(attachment.id);
    if (!exported) return;
    snackbarText(t("database.canvas.attachment.exported"), "success");
  } catch (e) {
    snackbarErrorCode(e);
  }
}

/**
 * 逻辑删除附件（移入回收站，附件文件保留），需用户确认。
 * @param attachment 目标附件
 */
async function removeAttachment(attachment: AttachmentVO): Promise<void> {
  const confirmed = await confirmDialogRef.value?.open({
    title: t("database.canvas.attachment.remove-confirm-title"),
    text: t("database.canvas.attachment.remove-confirm-text", {
      name: attachment.file_name,
    }),
    confirmColor: "error",
  });
  if (!confirmed) return;
  try {
    await userDatabaseAttachmentLogicalDelete(attachment.id);
    snackbarText(t("database.canvas.attachment.removed"), "success");
    await loadData();
  } catch (e) {
    snackbarErrorCode(e);
  }
}

/**
 * 恢复回收站中的附件。
 * @param attachment 目标附件
 */
async function restoreAttachment(attachment: AttachmentVO): Promise<void> {
  try {
    await userDatabaseAttachmentRestore(attachment.id);
    snackbarText(t("database.canvas.attachment.restored"), "success");
    await loadData();
  } catch (e) {
    snackbarErrorCode(e);
  }
}

/**
 * 物理删除回收站中的附件（附件文件一并删除，不可恢复），需用户确认。
 * @param attachment 目标附件
 */
async function physicalDeleteAttachment(attachment: AttachmentVO): Promise<void> {
  const confirmed = await confirmDialogRef.value?.open({
    title: t("database.canvas.attachment.physical-delete-confirm-title"),
    text: t("database.canvas.attachment.physical-delete-confirm-text", {
      name: attachment.file_name,
    }),
    confirmColor: "error",
  });
  if (!confirmed) return;
  try {
    await userDatabaseAttachmentPhysicalDelete(attachment.id);
    snackbarText(t("database.canvas.attachment.removed"), "success");
    await loadData();
  } catch (e) {
    snackbarErrorCode(e);
  }
}

/**
 * 删除无主附件文件（永久删除，不可恢复），需用户确认。
 * @param id 无主文件 id
 */
async function removeOrphanFile(id: string): Promise<void> {
  const confirmed = await confirmDialogRef.value?.open({
    title: t("database.canvas.attachment.orphan-remove-confirm-title"),
    text: t("database.canvas.attachment.orphan-remove-confirm-text", {
      name: id,
    }),
    confirmColor: "error",
  });
  if (!confirmed) return;
  try {
    await userDatabaseAttachmentRemoveOrphanFile(id);
    snackbarText(t("database.canvas.attachment.orphan-removed"), "success");
    await loadData();
  } catch (e) {
    snackbarErrorCode(e);
  }
}

/**
 * 预览附件。
 * @param attachment 目标附件
 */
function previewAttachment(attachment: AttachmentVO): void {
  previewDialogRef.value?.open(attachment);
}

/**
 * 拖拽开始：记录源附件 id，并设置自定义 ghost image 为整行元素。
 * @param event 拖拽事件
 * @param id 源附件 id
 */
function onDragStart(event: DragEvent, id: string): void {
  draggingId.value = id;
  if (event.dataTransfer) {
    event.dataTransfer.effectAllowed = "move";
    event.dataTransfer.setData("text/plain", id);
    const target = event.target as HTMLElement;
    const listItem = target.closest(".v-list-item");
    if (listItem) {
      event.dataTransfer.setDragImage(listItem, 0, 0);
    }
  }
}

/**
 * 拖拽经过：阻止默认行为以允许放置。
 * @param event 拖拽事件
 */
function onDragOver(event: DragEvent): void {
  event.preventDefault();
  if (event.dataTransfer) {
    event.dataTransfer.dropEffect = "move";
  }
}

/**
 * 放置：交换两个附件在数组中的位置，并调用后端 API 持久化。
 * @param event 拖拽事件
 * @param targetId 目标附件 id
 */
function onDrop(event: DragEvent, targetId: string): void {
  event.preventDefault();
  const sourceId = draggingId.value;
  if (!sourceId || sourceId === targetId) {
    draggingId.value = null;
    return;
  }
  const fromIdx = attachments.value.findIndex((a) => a.id === sourceId);
  const toIdx = attachments.value.findIndex((a) => a.id === targetId);
  if (fromIdx === -1 || toIdx === -1) {
    draggingId.value = null;
    return;
  }
  const temp = attachments.value[fromIdx];
  attachments.value[fromIdx] = attachments.value[toIdx];
  attachments.value[toIdx] = temp;
  void userDatabaseAttachmentSwapSortOrder(sourceId, targetId);
  draggingId.value = null;
}

defineExpose({ open });
</script>

<template>
  <VDialog v-model="dialog" max-width="40rem">
    <VCard>
      <VCardTitle class="attachment-title">
        {{ t("database.canvas.attachment.title", { title: nodeTitle }) }}
      </VCardTitle>
      <VCardText class="attachment-card-text">
        <div v-if="loading" class="attachment-loading">
          <VProgressCircular indeterminate color="primary" />
        </div>
        <template v-else>
          <VList v-if="attachments.length > 0" density="compact" class="attachment-list">
              <VListItem
                v-for="item in attachments"
                :key="item.id"
                :class="{ 'dragging-over': draggingId !== null && draggingId !== item.id }"
                @dragover="onDragOver"
                @drop="onDrop($event, item.id)"
              >
                <template #prepend>
                  <VBtn
                    icon="mdi-drag-vertical"
                    variant="text"
                    density="compact"
                    size="small"
                    class="attachment-drag-handle"
                    draggable="true"
                    :title="t('database.canvas.attachment.drag-to-reorder')"
                    @dragstart="onDragStart($event, item.id)"
                  />
                </template>
                <VListItemTitle class="attachment-name">
                 {{ item.file_name }}
                 <span v-if="item.missing_file" class="attachment-missing">
                   {{ t("database.canvas.attachment.missing-file") }}
                 </span>
               </VListItemTitle>
               <VListItemSubtitle>
                 {{ formatSize(item.size) }} · {{ d(new Date(item.create_time), "short") }}
               </VListItemSubtitle>
               <template #append>
                  <VBtn
                    v-if="!item.missing_file"
                    icon="mdi-eye-outline"
                    variant="text"
                    density="compact"
                    size="small"
                    :title="t('database.canvas.attachment.preview')"
                    @click="previewAttachment(item)"
                  />
                  <VBtn
                    v-if="!item.missing_file"
                    icon="mdi-export-variant"
                    variant="text"
                    density="compact"
                    size="small"
                    :title="t('database.canvas.attachment.export')"
                    @click="exportAttachment(item)"
                  />
                  <VBtn
                    icon="mdi-delete-outline"
                    variant="text"
                    density="compact"
                    size="small"
                    color="error"
                    :title="t('database.canvas.attachment.remove')"
                    @click="removeAttachment(item)"
                  />
                </template>
             </VListItem>
          </VList>
          <div v-else class="attachment-empty">
            {{ t("database.canvas.attachment.empty") }}
          </div>
          <template v-if="deletedAttachments.length > 0">
            <VDivider class="attachment-divider" />
            <div class="attachment-section-title">
              {{ t("database.canvas.attachment.recycle-bin", { count: deletedAttachments.length }) }}
            </div>
            <VList density="compact" class="attachment-list">
              <VListItem v-for="item in deletedAttachments" :key="item.id">
                <VListItemTitle class="attachment-name">
                  {{ item.file_name }}
                  <span v-if="item.missing_file" class="attachment-missing">
                    {{ t("database.canvas.attachment.missing-file") }}
                  </span>
                </VListItemTitle>
                <VListItemSubtitle>
                  {{ formatSize(item.size) }} · {{ d(new Date(item.create_time), "short") }}
                </VListItemSubtitle>
                <template #append>
                  <VBtn
                    icon="mdi-restore"
                    variant="text"
                    density="compact"
                    size="small"
                    :title="t('database.canvas.attachment.restore')"
                    @click="restoreAttachment(item)"
                  />
                  <VBtn
                    icon="mdi-delete-forever-outline"
                    variant="text"
                    density="compact"
                    size="small"
                    color="error"
                    :title="t('database.canvas.attachment.physical-delete')"
                    @click="physicalDeleteAttachment(item)"
                  />
                </template>
              </VListItem>
            </VList>
          </template>
          <template v-if="orphanFiles.length > 0">
            <VDivider class="attachment-divider" />
            <VAlert
              type="warning"
              variant="tonal"
              density="compact"
              class="attachment-orphan-alert"
              :title="t('database.canvas.attachment.orphan-title', { count: orphanFiles.length })"
              :text="t('database.canvas.attachment.orphan-text')"
            />
            <VList density="compact" class="attachment-list">
              <VListItem v-for="id in orphanFiles" :key="id">
                <VListItemTitle class="attachment-name">{{ id }}</VListItemTitle>
                <template #append>
                  <VBtn
                    icon="mdi-delete-forever-outline"
                    variant="text"
                    density="compact"
                    size="small"
                    color="error"
                    :title="t('database.canvas.attachment.orphan-remove')"
                    @click="removeOrphanFile(id)"
                  />
                </template>
              </VListItem>
            </VList>
          </template>
        </template>
      </VCardText>
      <VCardActions class="justify-end ga-2">
        <VBtn
          color="primary"
          variant="flat"
          prepend-icon="mdi-import"
          class="mr-auto"
          :loading="importing"
          :disabled="loading"
          @click="importAttachment"
        >
          {{ t("database.canvas.attachment.import") }}
        </VBtn>
        <VBtn variant="text" @click="dialog = false">
          {{ t("common.close") }}
        </VBtn>
      </VCardActions>
    </VCard>
    <AttachmentPreviewDialog ref="previewDialogRef" />
    <ConfirmDialog ref="confirmDialogRef" />
  </VDialog>
</template>

<style lang="scss" scoped>
.attachment-title {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.attachment-card-text {
  max-height: 60vh;
  overflow-y: auto;
}

.attachment-loading {
  display: flex;
  justify-content: center;
  padding-top: 2rem;
  padding-bottom: 2rem;
}

.attachment-empty {
  text-align: center;
  opacity: 0.6;
  padding-top: 2rem;
  padding-bottom: 2rem;
}

.attachment-name {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.attachment-missing {
  margin-left: 0.5rem;
  color: rgb(var(--v-theme-error));
  font-size: 0.75rem;
}

.attachment-divider {
  margin-top: 1rem;
  margin-bottom: 1rem;
}

.attachment-section-title {
  font-size: 0.875rem;
  opacity: 0.75;
  margin-bottom: 0.25rem;
}

.attachment-orphan-alert {
  margin-bottom: 0.5rem;
}

.attachment-drag-handle {
  cursor: grab;
  opacity: 0.38;
  color: rgb(var(--v-theme-on-surface));
}

.attachment-drag-handle:active {
  cursor: grabbing;
}

.dragging-over {
  border-top: 2px solid rgb(var(--v-theme-primary));
}

.mr-auto {
  margin-right: auto;
}
</style>
