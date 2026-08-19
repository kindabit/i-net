<!--
  主题导入对话框。

  粘贴主题 JSON 文本完成导入（主题分享入口）；
  数据校验与错误提示由主题基座（importTheme）承担。
-->
<script setup lang="ts">
import { computed, ref } from "vue";
import { isString } from "lodash";
import { t } from "@/i18n";
import { importTheme } from "@/themes";
import { snackbarText } from "@/composables/use-snackbar";

/** 对话框显示状态 */
const dialog = ref(false);
/** 待导入的主题 JSON 文本 */
const json = ref("");

/** 是否可以执行导入 */
const canImport = computed(() => json.value.trim().length > 0);

/** 打开对话框 */
function open() {
  json.value = "";
  dialog.value = true;
}

/** 关闭对话框 */
function close() {
  dialog.value = false;
}

/** 执行导入，成功后提示并关闭对话框 */
function onImport() {
  if (!canImport.value) return;
  if (importTheme(json.value)) {
    snackbarText(
      t("themes.import-success", { name: readDisplayName(json.value) }),
      "success",
    );
    close();
  }
}

/**
 * 从已通过校验的 JSON 文本中读取主题显示名称（读取失败时返回空字符串）。
 * @param text 主题 JSON 文本
 * @returns 主题显示名称
 */
function readDisplayName(text: string): string {
  try {
    const parsed = JSON.parse(text) as { displayName?: unknown };
    return isString(parsed.displayName) ? parsed.displayName : "";
  } catch {
    return "";
  }
}

defineExpose({ open, close });
</script>

<template>
  <VDialog v-model="dialog" max-width="560" persistent>
    <VCard>
      <VCardTitle>{{ t("themes.import-title") }}</VCardTitle>
      <VCardText>
        <VTextarea
          v-model="json"
          :label="t('themes.import-label')"
          :placeholder="t('themes.import-hint')"
          variant="outlined"
          rows="8"
          auto-grow
          hide-details
        />
      </VCardText>
      <VCardActions>
        <VSpacer />
        <VBtn variant="text" @click="close">{{ t("common.cancel") }}</VBtn>
        <VBtn
          color="primary"
          variant="flat"
          :disabled="!canImport"
          @click="onImport"
        >
          {{ t("themes.import") }}
        </VBtn>
      </VCardActions>
    </VCard>
  </VDialog>
</template>
