<!--
  通用名称输入对话框。

  通过 defineExpose 的 open() 以 Promise 形式获取用户输入：
  确认返回 trim 后的名称，取消或关闭（遮罩 / ESC）返回 null。
-->
<script lang="ts">
/** 打开名称输入对话框的选项 */
export interface NameInputDialogOptions {
  /** 标题 */
  title: string;
  /** 输入框标签 */
  label: string;
  /** 初始值（可选） */
  initialValue?: string;
  /** 确认按钮文案（默认 common.confirm） */
  confirmText?: string;
}
</script>

<script setup lang="ts">
import { ref, watch } from "vue";
import { t } from "@/i18n";

/** 对话框显示状态 */
const dialog = ref(false);
/** 当前展示的选项 */
const options = ref<NameInputDialogOptions>({ title: "", label: "" });
/** 输入框文本 */
const name = ref("");
/** 等待用户输入的 Promise resolve */
let resolveOpen: ((value: string | null) => void) | null = null;

/**
 * 打开对话框并等待用户输入。
 * @param opts 对话框选项
 * @returns 确认返回 trim 后的名称，取消或关闭返回 null
 */
function open(opts: NameInputDialogOptions): Promise<string | null> {
  settle(null);
  options.value = opts;
  name.value = opts.initialValue ?? "";
  dialog.value = true;
  return new Promise((resolve) => {
    resolveOpen = resolve;
  });
}

/**
 * 结算等待中的 Promise。
 * @param value 用户输入或 null
 */
function settle(value: string | null) {
  resolveOpen?.(value);
  resolveOpen = null;
}

/** 确认并关闭 */
function onConfirm() {
  const trimmed = name.value.trim();
  if (trimmed === "") return;
  settle(trimmed);
  dialog.value = false;
}

// 任何途径的关闭（取消按钮、遮罩、ESC）都按取消结算
watch(dialog, (value) => {
  if (!value) settle(null);
});

defineExpose({ open });
</script>

<template>
  <VDialog v-model="dialog" max-width="400">
    <VCard>
      <VCardTitle>{{ options.title }}</VCardTitle>
      <VCardText>
        <VTextField
          v-model="name"
          :label="options.label"
          autofocus
          variant="outlined"
          density="comfortable"
          @keydown.enter="onConfirm"
        />
      </VCardText>
      <VCardActions class="justify-end ga-2">
        <VBtn variant="text" @click="dialog = false">
          {{ t("common.cancel") }}
        </VBtn>
        <VBtn
          color="primary"
          variant="flat"
          :disabled="name.trim() === ''"
          @click="onConfirm"
        >
          {{ options.confirmText ?? t("common.confirm") }}
        </VBtn>
      </VCardActions>
    </VCard>
  </VDialog>
</template>
