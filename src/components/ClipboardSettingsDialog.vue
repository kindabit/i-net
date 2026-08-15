<!--
  剪贴板设置对话框。

  提供设置自动清空剪贴板等待时间的功能，直接复用全局共享的配置状态，点击保存后自动持久化存储。
-->
<script setup lang="ts">
import { ref } from "vue";
import { useClipboardClear } from "@/composables/use-clipboard-clear";

/** 对话框显示状态 */
const dialog = ref(false);
const { timeoutSeconds, saveTimeoutConfig } = useClipboardClear();

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
      <VCardTitle>剪贴板设置</VCardTitle>
      <VCardText>
        <div class="text-body-1 mb-4">自动清空剪贴板等待时间（秒）</div>
        <VSlider
          v-model="timeoutSeconds"
          min="1"
          max="60"
          step="1"
          thumb-label="always"
          show-ticks="always"
          :ticks="{ 1: '1秒', 10: '10秒', 30: '30秒', 60: '60秒' }"
        >
          <template #thumb-label="{ modelValue }">
            {{ modelValue }}秒
          </template>
        </VSlider>
      </VCardText>
      <VCardActions>
        <VSpacer />
        <VBtn variant="text" @click="close">关闭</VBtn>
        <VBtn color="primary" variant="flat" @click="saveAndClose">保存</VBtn>
      </VCardActions>
    </VCard>
  </VDialog>
</template>
