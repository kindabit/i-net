<!--
  编辑边的对话框组件。

  通过 defineExpose 的 open() 以 Promise 形式获取用户编辑结果：
  确认返回 trim 后的 { title, description }，取消或关闭（遮罩 / ESC）返回 null。
-->
<script lang="ts">
/** 编辑边的对话框输入选项 */
export interface EditEdgeDialogOptions {
  /** 初始标题 */
  title: string;
  /** 初始详情 */
  description: string;
}
</script>

<script setup lang="ts">
import { ref, watch } from "vue";
import { t } from "@/i18n";

/** 对话框显示状态 */
const dialog = ref(false);
/** 边标题 */
const title = ref("");
/** 边详情 */
const description = ref("");
/** 提交中状态 */
const submitting = ref(false);
/** 等待用户结果的 Promise resolve */
let resolveOpen: ((value: { title: string; description: string } | null) => void) | null =
  null;

/**
 * 打开对话框并等待用户编辑。
 * @param opts 初始标题和详情
 * @returns 确认返回 trim 后的 { title, description }，取消或关闭返回 null
 */
function open(
  opts: EditEdgeDialogOptions
): Promise<{ title: string; description: string } | null> {
  settle(null);
  title.value = opts.title;
  description.value = opts.description;
  dialog.value = true;
  return new Promise((resolve) => {
    resolveOpen = resolve;
  });
}

/**
 * 结算等待中的 Promise。
 * @param value 编辑结果或 null
 */
function settle(value: { title: string; description: string } | null) {
  resolveOpen?.(value);
  resolveOpen = null;
}

/** 确认并关闭 */
function onConfirm() {
  submitting.value = true;
  settle({ title: title.value.trim(), description: description.value.trim() });
  dialog.value = false;
  submitting.value = false;
}

// 任何途径的关闭（取消按钮、遮罩、ESC）都按取消结算
watch(dialog, (value) => {
  if (!value) settle(null);
});

defineExpose({ open });
</script>

<template>
  <VDialog v-model="dialog" max-width="48rem" :persistent="submitting">
    <VCard>
      <VCardTitle>{{ t("database.canvas.edit-edge") }}</VCardTitle>
      <VCardText>
        <VTextField
          v-model="title"
          :label="t('database.canvas.edit-edge-title-label')"
          variant="outlined"
          density="comfortable"
          autofocus
        />
        <VTextarea
          v-model="description"
          :label="t('database.canvas.edit-edge-description-label')"
          variant="outlined"
          density="comfortable"
          rows="3"
          class="mt-4"
        />
      </VCardText>
      <VCardActions class="justify-end ga-2">
        <VBtn variant="text" @click="dialog = false">
          {{ t("common.cancel") }}
        </VBtn>
        <VBtn
          color="primary"
          variant="flat"
          :loading="submitting"
          @click="onConfirm"
        >
          {{ t("common.confirm") }}
        </VBtn>
      </VCardActions>
    </VCard>
  </VDialog>
</template>
