<!--
  主题编辑对话框。

  用于新建或编辑自定义主题：设置主题标识、显示名称、明暗基调与核心颜色。
  新建时以当前主题为底色起点；编辑时主题标识不可修改。
-->
<script setup lang="ts">
import { computed, reactive, ref } from "vue";
import { t } from "@/i18n";
import { builtinThemeNames } from "@/themes/builtin";
import {
  currentThemeName,
  getThemeDefinition,
  hasTheme,
  saveCustomTheme,
} from "@/themes";
import { snackbarText } from "@/composables/use-snackbar";
import type { AppThemeDefinition } from "@/themes/types";

/** 参与编辑的核心颜色键（on-* 等派生颜色由 Vuetify 自动生成） */
const COLOR_KEYS = [
  "background",
  "surface",
  "primary",
  "secondary",
  "success",
  "warning",
  "error",
  "info",
] as const;

/** 对话框显示状态 */
const dialog = ref(false);
/** 编辑模式（编辑已有主题时为 true，标识不可修改） */
const editing = ref(false);
/** 编辑草稿 */
const draft = reactive({
  name: "",
  displayName: "",
  dark: false,
  colors: {} as Record<string, string>,
});
/** 主题标识错误提示 */
const nameError = ref("");
/** 显示名称错误提示 */
const displayNameError = ref("");

/** 对话框标题 */
const title = computed(() =>
  editing.value ? t("themes.editor-title-edit") : t("themes.editor-title-create"),
);

/**
 * 打开编辑器。传入主题名时进入编辑模式，否则以当前主题为底色起点新建。
 * @param name 待编辑的主题名，不传则为新建
 */
function open(name?: string) {
  nameError.value = "";
  displayNameError.value = "";
  const initial = name !== undefined ? getThemeDefinition(name) : null;
  editing.value = initial !== null;
  const base = initial ?? getThemeDefinition(currentThemeName.value);
  draft.name = initial
    ? initial.name
    : `custom-${Date.now().toString(36)}`;
  draft.displayName = initial
    ? initial.displayName
    : t("themes.editor-default-display-name");
  draft.dark = base?.dark ?? false;
  draft.colors = pickColors(base?.colors ?? {});
  dialog.value = true;
}

/**
 * 从主题颜色表中拣选核心颜色键，缺失时使用占位色。
 * @param colors 主题颜色表
 * @returns 仅含核心颜色键的颜色表
 */
function pickColors(colors: Record<string, string>): Record<string, string> {
  return Object.fromEntries(
    COLOR_KEYS.map((key) => [key, colors[key] ?? "#9E9E9E"]),
  );
}

/** 关闭对话框 */
function close() {
  dialog.value = false;
}

/** 校验并保存主题 */
function onSave() {
  nameError.value = "";
  displayNameError.value = "";
  const name = draft.name.trim();
  const displayName = draft.displayName.trim();
  if (name === "") {
    nameError.value = t("themes.editor-name-empty");
    return;
  }
  if (!editing.value && builtinThemeNames.has(name)) {
    nameError.value = t("themes.editor-name-reserved", { name });
    return;
  }
  if (!editing.value && hasTheme(name)) {
    nameError.value = t("themes.editor-name-exists");
    return;
  }
  if (displayName === "") {
    displayNameError.value = t("themes.editor-display-name-empty");
    return;
  }
  const def: AppThemeDefinition = {
    name,
    displayName,
    dark: draft.dark,
    colors: { ...draft.colors },
  };
  if (saveCustomTheme(def)) {
    snackbarText(t("themes.editor-save-success", { name: displayName }), "success");
    close();
  }
}

defineExpose({ open, close });
</script>

<template>
  <VDialog v-model="dialog" max-width="560" persistent>
    <VCard>
      <VCardTitle>{{ title }}</VCardTitle>
      <VCardText class="editor-body">
        <VTextField
          v-model="draft.name"
          :label="t('themes.editor-name')"
          :hint="t('themes.editor-name-hint')"
          persistent-hint
          :disabled="editing"
          :error-messages="nameError"
          variant="outlined"
          density="comfortable"
          class="mb-2"
        />
        <VTextField
          v-model="draft.displayName"
          :label="t('themes.editor-display-name')"
          :error-messages="displayNameError"
          variant="outlined"
          density="comfortable"
          class="mb-2"
        />
        <VSwitch
          v-model="draft.dark"
          :label="t('themes.editor-dark')"
          color="primary"
          density="comfortable"
          hide-details
          class="mb-2"
        />
        <div class="color-grid">
          <div v-for="key in COLOR_KEYS" :key="key" class="color-row">
            <VMenu :close-on-content-click="false" offset="8">
              <template #activator="{ props }">
                <button
                  v-bind="props"
                  type="button"
                  class="swatch-btn"
                  :style="{ backgroundColor: draft.colors[key] }"
                  :aria-label="t(`themes.colors.${key}`)"
                />
              </template>
              <VColorPicker v-model="draft.colors[key]" mode="hex" :modes="['hex']" />
            </VMenu>
            <VTextField
              v-model="draft.colors[key]"
              :label="t(`themes.colors.${key}`)"
              variant="outlined"
              density="compact"
              hide-details
            />
          </div>
        </div>
      </VCardText>
      <VCardActions>
        <VSpacer />
        <VBtn variant="text" @click="close">{{ t("common.cancel") }}</VBtn>
        <VBtn color="primary" variant="flat" @click="onSave">
          {{ t("themes.editor-save") }}
        </VBtn>
      </VCardActions>
    </VCard>
  </VDialog>
</template>

<style lang="scss" scoped>
.editor-body {
  max-height: 60vh;
  overflow-y: auto;
}

.color-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 0.5rem 0.75rem;
}

.color-row {
  display: flex;
  align-items: center;
  gap: 0.5rem;

  .swatch-btn {
    width: 2rem;
    height: 2rem;
    flex-shrink: 0;
    border-radius: 0.25rem;
    border: 1px solid rgba(var(--v-border-color), var(--v-border-opacity));
    cursor: pointer;
  }
}
</style>
