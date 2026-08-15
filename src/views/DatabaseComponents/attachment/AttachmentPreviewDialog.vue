<!--
  附件预览对话框。

  打开后从后端加载附件明文（内存中解密，不落盘），按扩展名路由到
  对应查看器（file-viewer 通用预览 / 文本）预览；查看器组件按需异步加载，不进入主包。
  文本查看器支持编辑与保存，关闭前若存在未保存修改需二次确认。
  不支持的类型提示导出后用系统程序打开；加载失败提示并关闭。
  通过 defineExpose 的 open() 打开。
-->
<script setup lang="ts">
import { computed, defineAsyncComponent, ref, type Component } from "vue";
import { t } from "@/i18n";
import { userDatabaseAttachmentExport, userDatabaseAttachmentLoad } from "@/api";
import type { AttachmentVO } from "@/api-types";
import { snackbarErrorCode, snackbarText } from "@/composables/use-snackbar";
import { viewerTypeOf, type AttachmentViewerType } from "./attachment-types";
import ConfirmDialog from "@/components/ConfirmDialog.vue";

/** 各查看器类型对应的异步组件加载器（动态 import，按需分包） */
const VIEWER_LOADERS: Record<AttachmentViewerType, () => Promise<Component>> = {
  omni: () => import("./AttachmentViewerOmni.vue"),
  text: () => import("./AttachmentViewerText.vue"),
};

/** 对话框显示状态 */
const dialog = ref(false);
/** 内容加载中 */
const loading = ref(false);
/** 导出进行中 */
const exporting = ref(false);
/** 当前预览的附件 */
const attachment = ref<AttachmentVO | null>(null);
/** 附件明文内容 */
const bytes = ref<Uint8Array | null>(null);
/** 路由出的查看器类型；null 表示不支持预览 */
const viewerType = ref<AttachmentViewerType | null>(null);

/** 当前应渲染的查看器组件（按类型异步加载） */
const viewerComponent = computed(() =>
  viewerType.value === null
    ? null
    : defineAsyncComponent(VIEWER_LOADERS[viewerType.value]),
);

/** 通用确认对话框引用（文本查看器未保存修改的关闭确认） */
const confirmDialogRef = ref<InstanceType<typeof ConfirmDialog>>();
/** 当前查看器组件实例；仅文本查看器暴露 hasUnsavedChanges()，供关闭前查询未保存状态 */
const viewerRef = ref<{ hasUnsavedChanges?: () => boolean } | null>(null);

/**
 * 打开对话框预览指定附件。
 * @param target 目标附件
 */
function open(target: AttachmentVO): void {
  attachment.value = target;
  bytes.value = null;
  viewerType.value = viewerTypeOf(target.file_name);
  dialog.value = true;
  // 不支持的类型无需加载内容，直接展示提示与导出入口
  if (viewerType.value !== null) {
    void loadContent(target.id);
  }
}

/**
 * 加载附件明文内容；失败提示并关闭对话框。
 * @param id 附件 id
 */
async function loadContent(id: string): Promise<void> {
  loading.value = true;
  try {
    const data = await userDatabaseAttachmentLoad(id);
    bytes.value = new Uint8Array(data);
  } catch (e) {
    snackbarErrorCode(e);
    dialog.value = false;
  } finally {
    loading.value = false;
  }
}

/**
 * 导出当前附件：由后端弹出系统保存对话框，导出成功后提示；取消选择静默返回。
 * 无输入参数，无返回值。
 */
async function exportCurrent(): Promise<void> {
  const current = attachment.value;
  if (!current) return;
  exporting.value = true;
  try {
    const exported = await userDatabaseAttachmentExport(current.id);
    if (!exported) return;
    snackbarText(t("database.canvas.attachment.exported"), "success");
  } catch (e) {
    snackbarErrorCode(e);
  } finally {
    exporting.value = false;
  }
}

/**
 * 请求关闭对话框：若当前为文本查看器且存在未保存修改，先弹确认框，确认后才真正关闭；
 * 无未保存修改时直接关闭。无输入参数，无返回值。
 */
async function requestClose(): Promise<void> {
  if (viewerType.value === "text" && viewerRef.value?.hasUnsavedChanges?.()) {
    const confirmed = await confirmDialogRef.value?.open({
      title: t("database.canvas.attachment.text-unsaved-confirm-title"),
      text: t("database.canvas.attachment.text-unsaved-confirm-text"),
    });
    if (!confirmed) return;
  }
  dialog.value = false;
}

/**
 * 处理 VDialog 的 modelValue 更新：打开直接生效；任何途径的关闭（按钮 / 遮罩 / ESC）
 * 都先经未保存修改检查，确认通过后再真正关闭。
 * @param value 对话框下一次的显示状态
 */
function onDialogUpdate(value: boolean): void {
  if (value) {
    dialog.value = true;
    return;
  }
  // 已处于关闭态时忽略重复的关闭事件（确认关闭触发更新时防递归）
  if (!dialog.value) return;
  void requestClose();
}

defineExpose({ open });
</script>

<template>
  <VDialog
    :model-value="dialog"
    max-width="56rem"
    @update:model-value="onDialogUpdate"
  >
    <VCard>
      <VCardTitle class="preview-title">{{ attachment?.file_name }}</VCardTitle>
      <VCardText class="preview-content">
        <div v-if="loading" class="preview-loading">
          <VProgressCircular indeterminate color="primary" />
        </div>
        <div v-else-if="viewerComponent && bytes" class="preview-viewer-wrap">
          <component
            :is="viewerComponent"
            ref="viewerRef"
            :bytes="bytes"
            :file-name="attachment?.file_name ?? ''"
            :attachment-id="attachment?.id ?? ''"
          />
        </div>
        <div v-else class="preview-unsupported">
          {{ t("database.canvas.attachment.preview-unsupported") }}
        </div>
      </VCardText>
      <VCardActions class="justify-end ga-2">
        <VBtn
          v-if="viewerType === null"
          color="primary"
          variant="flat"
          :loading="exporting"
          @click="exportCurrent"
        >
          {{ t("database.canvas.attachment.export") }}
        </VBtn>
        <VBtn variant="text" @click="requestClose">
          {{ t("common.close") }}
        </VBtn>
      </VCardActions>
    </VCard>
  </VDialog>
  <ConfirmDialog ref="confirmDialogRef" />
</template>

<style lang="scss" scoped>
.preview-title {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

// 内容区固定高度，对话框尺寸不随预览内容变化；超出部分由查看器内部滚动承载
.preview-content {
  height: 65vh;
  display: flex;
  flex-direction: column;
  overflow: hidden;

  > * {
    height: 100%;
    min-height: 0;
  }
}

.preview-viewer-wrap {
  display: flex;
  flex-direction: column;
}

.preview-loading {
  display: flex;
  justify-content: center;
  align-items: center;
}

.preview-unsupported {
  display: flex;
  justify-content: center;
  align-items: center;
  text-align: center;
  opacity: 0.75;
}
</style>
