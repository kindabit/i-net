<!--
  数据库导出对话框。

  提醒用户导出内容为明文，让用户选择字段导出模式后调用后端导出。
-->
<script setup lang="ts">
import { ref, watch } from "vue";
import { t } from "@/i18n";
import type { DatabaseExportMode } from "@/api";

/** 对话框显示状态 */
const dialog = ref(false);
/** 当前选中的导出模式，默认打码值 */
const mode = ref<DatabaseExportMode>("mask-values");
/** 等待用户选择的 Promise resolve（仅生效一次） */
let resolveOpen: ((value: DatabaseExportMode | null) => void) | null = null;

/**
 * 打开对话框并等待用户选择。
 * @returns 确定返回所选模式，取消或关闭返回 null
 */
function open(): Promise<DatabaseExportMode | null> {
  // 重复打开时先按取消结算上一次等待，避免 Promise 泄漏
  settle(null);
  mode.value = "mask-values";
  dialog.value = true;
  return new Promise((resolve) => {
    resolveOpen = resolve;
  });
}

/**
 * 结算等待中的 Promise。
 * @param value 用户选择
 */
function settle(value: DatabaseExportMode | null) {
  resolveOpen?.(value);
  resolveOpen = null;
}

/** 确定并关闭 */
function onConfirm() {
  settle(mode.value);
  dialog.value = false;
}

// 任何途径的关闭（取消按钮、遮罩、ESC）都按取消结算
watch(dialog, (value) => {
  if (!value) settle(null);
});

defineExpose({ open });
</script>

<template>
  <VDialog v-model="dialog" max-width="480">
    <VCard>
      <VCardTitle>{{ t("database.export.dialog-title") }}</VCardTitle>
      <VCardText>
        <VAlert type="warning" variant="tonal" class="mb-4">
          {{ t("database.export.warning") }}
        </VAlert>
        <VRadioGroup v-model="mode" :label="t('database.export.mode-label')">
          <VRadio value="exclude-fields" :label="t('database.export.mode-exclude-fields')" />
          <div class="text-medium-emphasis pl-8 pb-2" style="font-size: 0.75rem">
            {{ t("database.export.mode-exclude-fields-hint") }}
          </div>
          <VRadio value="mask-values" :label="t('database.export.mode-mask-values')" />
          <div class="text-medium-emphasis pl-8 pb-2" style="font-size: 0.75rem">
            {{ t("database.export.mode-mask-values-hint") }}
          </div>
          <VRadio value="include-values" :label="t('database.export.mode-include-values')" />
          <div class="text-medium-emphasis pl-8 pb-2" style="font-size: 0.75rem">
            {{ t("database.export.mode-include-values-hint") }}
          </div>
        </VRadioGroup>
      </VCardText>
      <VCardActions>
        <VSpacer />
        <VBtn variant="text" @click="dialog = false">
          {{ t("common.cancel") }}
        </VBtn>
        <VBtn color="primary" variant="flat" @click="onConfirm">
          {{ t("common.confirm") }}
        </VBtn>
      </VCardActions>
    </VCard>
  </VDialog>
</template>
