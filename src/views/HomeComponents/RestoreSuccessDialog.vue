<!--
  还原成功提示对话框。
  只允许通过点击唯一按钮关闭，persistent 防止点击 overlay 或按下 Esc 关闭。
  点击按钮触发 window.location.replace("/") 刷新当前 webview 重新加载 Home，
  由前端重新从最新内存 connection 拉取数据。后端 Rust 进程不退出。
-->
<script setup lang="ts">
import { ref } from "vue";
import { t } from "@/i18n";

const dialog = ref(false);

/**
 * 打开对话框。
 */
function open() {
  dialog.value = true;
}

/**
 * 刷新当前 webview：把 URL 切回 / 并触发完整 reload。
 * 完成后回到 Home，由其 onMounted 重新从内存 connection 拉取数据。
 */
function refresh() {
  window.location.replace("/");
}

defineExpose({ open });
</script>

<template>
  <VDialog v-model="dialog" max-width="24rem" persistent>
    <VCard>
      <VCardTitle>{{ t("home.restore-success.title") }}</VCardTitle>
      <VCardText class="success-text">
        {{ t("home.restore-success.text") }}
      </VCardText>
      <VCardActions>
        <VSpacer />
        <VBtn color="primary" variant="flat" @click="refresh">
          {{ t("home.restore-success.button") }}
        </VBtn>
      </VCardActions>
    </VCard>
  </VDialog>
</template>

<style lang="scss" scoped>
.success-text {
  font-size: 0.875rem;
}
</style>