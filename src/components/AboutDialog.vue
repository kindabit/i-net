<!--
  关于对话框。

  展示软件版本、作者、源代码仓库地址、技术栈版本信息以及软件简介。
  仓库地址可在系统默认浏览器中打开；技术栈中的 Vue 与 Vuetify 版本取自前端编译期常量，
  其余版本通过后端接口获取。
-->
<script setup lang="ts">
import { onMounted, ref } from "vue";
import { version as vueVersion } from "vue";
import { version as vuetifyVersion } from "vuetify";
import { openUrl } from "@tauri-apps/plugin-opener";
import { appInfoGet, type AppInfo } from "@/api";
import { t } from "@/i18n";
import { snackbarText } from "@/composables/use-snackbar";

/** 对话框显示状态 */
const dialog = ref(false);
/** 已加载的应用信息；null 表示尚未加载成功 */
const info = ref<AppInfo | null>(null);
/** 防止 onMounted 与用户点击并发触发重复加载 */
let loading = false;

/**
 * 按需加载应用信息。已成功加载或正在加载时直接返回，避免重复 IPC 调用；
 * 加载失败时弹出错误提示并复位 loading，使下次调用可以重试。
 */
async function ensureLoaded() {
  if (info.value !== null || loading) return;
  loading = true;
  try {
    info.value = await appInfoGet();
  } catch {
    snackbarText(t("app.about-load-failed"), "error");
  } finally {
    loading = false;
  }
}

/**
 * 打开对话框。若预取已成功则无延迟直接展示；预取失败时复用以加载函数进行重试。
 */
async function open() {
  dialog.value = true;
  await ensureLoaded();
}

/** 关闭对话框 */
function close() {
  dialog.value = false;
}

/**
 * 在系统默认浏览器中打开源代码仓库地址。info 未加载时不响应点击。
 */
function openRepository() {
  if (info.value) {
    openUrl(info.value.repository);
  }
}

onMounted(ensureLoaded);

defineExpose({ open, close });
</script>

<template>
  <VDialog v-model="dialog" max-width="480">
    <VCard>
      <VCardTitle>{{ t("app.about-title") }}</VCardTitle>
      <VCardText>
        <VList density="compact">
          <VListItem>
            <div class="about-row">
              <span class="about-label">{{ t("app.about-version") }}</span>
              <span class="about-value">{{ info?.app_version ?? "-" }}</span>
            </div>
          </VListItem>
          <VListItem>
            <div class="about-row">
              <span class="about-label">{{ t("app.about-author") }}</span>
              <span class="about-value">{{ info?.author ?? "-" }}</span>
            </div>
          </VListItem>
          <VListItem>
            <div class="about-row">
              <span class="about-label">{{ t("app.about-repository") }}</span>
              <span
                class="about-value about-link"
                :class="{ 'about-link-disabled': !info }"
                role="button"
                @click="openRepository"
              >
                {{ info?.repository ?? "-" }}
              </span>
            </div>
          </VListItem>
          <VListItem>
            <div class="about-row">
              <span class="about-label">{{ t("app.about-rust-version") }}</span>
              <span class="about-value">{{ info?.rust_version ?? "-" }}</span>
            </div>
          </VListItem>
          <VListItem>
            <div class="about-row">
              <span class="about-label">{{ t("app.about-tauri-version") }}</span>
              <span class="about-value">{{ info?.tauri_version ?? "-" }}</span>
            </div>
          </VListItem>
          <VListItem>
            <div class="about-row">
              <span class="about-label">{{ t("app.about-vue-version") }}</span>
              <span class="about-value">{{ vueVersion }}</span>
            </div>
          </VListItem>
          <VListItem>
            <div class="about-row">
              <span class="about-label">{{ t("app.about-vuetify-version") }}</span>
              <span class="about-value">{{ vuetifyVersion }}</span>
            </div>
          </VListItem>
        </VList>
        <div class="about-description">{{ t("app.about-description") }}</div>
      </VCardText>
      <VCardActions>
        <VSpacer />
        <VBtn variant="text" @click="close">{{ t("common.close") }}</VBtn>
      </VCardActions>
    </VCard>
  </VDialog>
</template>

<style scoped>
.about-row {
  display: flex;
  align-items: baseline;
  gap: 1rem;
  width: 100%;
  min-width: 0;
}

.about-label {
  flex: 0 0 auto;
  color: rgba(var(--v-theme-on-surface), var(--v-medium-emphasis-opacity));
}

.about-value {
  flex: 1 1 auto;
  min-width: 0;
  word-break: break-all;
}

.about-link {
  cursor: pointer;
  color: rgb(var(--v-theme-primary));
  text-decoration: underline;
}

.about-link-disabled {
  cursor: default;
  text-decoration: none;
}

.about-description {
  margin-top: 1rem;
  color: rgba(var(--v-theme-on-surface), var(--v-medium-emphasis-opacity));
  line-height: 1.5;
}
</style>