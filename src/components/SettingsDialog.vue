<!--
  应用设置对话框。

  集中承载应用的各项配置（当前包含剪贴板自动清空等待时间），
  直接复用全局共享的配置状态，点击保存后自动持久化存储。
-->
<script setup lang="ts">
import { ref } from "vue";
import { useClipboardClear } from "@/composables/use-clipboard-clear";
import { t } from "@/i18n";

/** 对话框显示状态 */
const dialog = ref(false);
const { timeoutSeconds, saveTimeoutConfig } = useClipboardClear();

/**
 * 生成秒数文案（滑块刻度与拇指标签共用）。
 * @param value 秒数
 * @returns 本地化秒数文案
 */
function formatSeconds(value: number): string {
  return t("app.settings-seconds", { n: value });
}

/** 打开对话框 */
function open() {
  dialog.value = true;
}

/** 关闭对话框 */
function close() {
  dialog.value = false;
}

/** 保存配置并关闭对话框 */
async function saveAndClose() {
  await saveTimeoutConfig();
  dialog.value = false;
}

defineExpose({ open, close });
</script>

<template>
  <VDialog v-model="dialog" max-width="480">
    <VCard>
      <VCardTitle>{{ t("app.settings") }}</VCardTitle>
      <VCardText>
        <div class="text-body-1 mb-4">
          {{ t("app.settings-clipboard-clear-timeout") }}
        </div>
        <VSlider
          v-model="timeoutSeconds"
          min="1"
          max="60"
          step="1"
          thumb-label="hover"
          show-ticks="always"
          :ticks="{
            1: formatSeconds(1),
            10: formatSeconds(10),
            30: formatSeconds(30),
            60: formatSeconds(60),
          }"
        >
          <template #thumb-label="{ modelValue }">
            <div class="text-no-wrap">{{ formatSeconds(modelValue) }}</div>
          </template>
        </VSlider>
      </VCardText>
      <VCardActions>
        <VSpacer />
        <VBtn variant="text" @click="close">{{ t("common.close") }}</VBtn>
        <VBtn color="primary" variant="flat" @click="saveAndClose">
          {{ t("common.save") }}
        </VBtn>
      </VCardActions>
    </VCard>
  </VDialog>
</template>
