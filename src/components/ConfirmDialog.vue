<!--
  通用确认对话框。

  通过 defineExpose 的 open() 以 Promise 形式获取用户选择：
  确认返回 true，取消或关闭（遮罩 / ESC）返回 false。
  按钮文案默认取 common 模块的 confirm / cancel。
-->
<script lang="ts">
/** 打开确认对话框的选项 */
export interface ConfirmDialogOptions {
  /** 标题 */
  title: string;
  /** 正文（可选） */
  text?: string;
  /** 确认按钮文案（默认 common.confirm） */
  confirmText?: string;
  /** 确认按钮颜色（默认 primary） */
  confirmColor?: string;
}
</script>

<script setup lang="ts">
import { ref, watch } from "vue";
import { t } from "@/i18n";

/** 对话框显示状态 */
const dialog = ref(false);
/** 当前展示的选项 */
const options = ref<ConfirmDialogOptions>({ title: "" });
/** 等待用户选择的 Promise resolve（仅生效一次） */
let resolveOpen: ((value: boolean) => void) | null = null;

/**
 * 打开对话框并等待用户选择。
 * @param opts 对话框选项
 * @returns 确认返回 true，取消或关闭返回 false
 */
function open(opts: ConfirmDialogOptions): Promise<boolean> {
  // 重复打开时先按取消结算上一次等待，避免 Promise 泄漏
  settle(false);
  options.value = opts;
  dialog.value = true;
  return new Promise((resolve) => {
    resolveOpen = resolve;
  });
}

/**
 * 结算等待中的 Promise。
 * @param value 用户选择
 */
function settle(value: boolean) {
  resolveOpen?.(value);
  resolveOpen = null;
}

/** 确认并关闭 */
function onConfirm() {
  settle(true);
  dialog.value = false;
}

// 任何途径的关闭（取消按钮、遮罩、ESC）都按取消结算
watch(dialog, (value) => {
  if (!value) settle(false);
});

defineExpose({ open });
</script>

<template>
  <VDialog v-model="dialog" max-width="400">
    <VCard>
      <VCardTitle>{{ options.title }}</VCardTitle>
      <VCardText v-if="options.text">{{ options.text }}</VCardText>
      <VCardActions class="justify-end ga-2">
        <VBtn variant="text" @click="dialog = false">
          {{ t("common.cancel") }}
        </VBtn>
        <VBtn
          :color="options.confirmColor ?? 'primary'"
          variant="flat"
          @click="onConfirm"
        >
          {{ options.confirmText ?? t("common.confirm") }}
        </VBtn>
      </VCardActions>
    </VCard>
  </VDialog>
</template>
