<!--
  数据损坏受控崩溃对话框。

  由 App.vue 全局挂载，消费 use-fatal-error 的 fatalError 状态。
  阻塞式展示（persistent），点遮罩与 Esc 均不关闭，唯一出路是退出应用：
  调用 fatalExit 通知后端记录日志后退出进程（不保存任何数据）。
-->
<script setup lang="ts">
import { computed } from "vue";
import { t } from "@/i18n";
import { fatalError } from "@/composables/use-fatal-error";
import { fatalExit } from "@/api";

/** 当前是否有待展示的致命错误（供 VDialog 的 model-value 使用） */
const isOpen = computed(() => fatalError.value !== null);

/**
 * 点击退出按钮：序列化当前致命错误并通知后端执行受控崩溃。
 * 后端会在记录日志后直接退出进程，前端无需额外处理。
 */
function onExit() {
  void fatalExit(JSON.stringify(fatalError.value));
}
</script>

<template>
  <VDialog
    :model-value="isOpen"
    persistent
    max-width="448"
  >
    <VCard>
      <VCardItem>
        <template #prepend>
          <VIcon
            icon="mdi-alert-octagon-outline"
            color="error"
            size="2rem"
          />
        </template>
        <VCardTitle>{{ t("app.fatal-error-title") }}</VCardTitle>
      </VCardItem>
      <VCardText v-if="fatalError">
        <div class="fatal-error-variant-title">
          {{ t(`error-code.${fatalError.variant}.title`) }}
        </div>
        <div class="fatal-error-variant-text">
          {{ t(`error-code.${fatalError.variant}.text`, fatalError.data ?? {}) }}
        </div>
        <div class="fatal-error-notice">
          {{ t("app.fatal-error-notice") }}
        </div>
      </VCardText>
      <VCardActions class="justify-end">
        <VBtn
          color="error"
          variant="flat"
          @click="onExit"
        >
          {{ t("app.fatal-error-exit") }}
        </VBtn>
      </VCardActions>
    </VCard>
  </VDialog>
</template>

<style lang="scss" scoped>
.fatal-error-variant-title {
  font-weight: 600;
  margin-bottom: 0.5rem;
}

.fatal-error-variant-text {
  margin-bottom: 0.75rem;
}

.fatal-error-notice {
  color: rgb(var(--v-theme-on-surface));
  opacity: 0.85;
}
</style>
