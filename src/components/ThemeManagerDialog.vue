<!--
  主题管理对话框。

  展示全部主题（内置/自定义分组标识与色板预览），
  提供新建、导入、编辑、导出、删除入口；
  编辑与导入分别由 ThemeEditDialog 与 ThemeImportDialog 承担。
-->
<script setup lang="ts">
import { ref } from "vue";
import { t } from "@/i18n";
import { vuetify } from "@/vuetify";
import { exportTheme, removeCustomTheme, themeList } from "@/themes";
import { snackbarText } from "@/composables/use-snackbar";
import ThemeEditDialog from "@/components/ThemeEditDialog.vue";
import ThemeImportDialog from "@/components/ThemeImportDialog.vue";
import type { AppThemeDefinition } from "@/themes/types";

/** 对话框显示状态 */
const dialog = ref(false);
/** 待删除主题（删除确认对话框的目标，null 表示未在删除流程中） */
const deleting = ref<{ name: string; displayName: string } | null>(null);

const editDialogRef = ref<InstanceType<typeof ThemeEditDialog>>();
const importDialogRef = ref<InstanceType<typeof ThemeImportDialog>>();

/** 打开对话框 */
function open() {
  dialog.value = true;
}

/** 关闭对话框 */
function close() {
  dialog.value = false;
}

/**
 * 读取主题的代表色（用于列表色板预览）。
 * @param name 主题名
 * @returns 颜色值数组
 */
function swatchesOf(name: string): string[] {
  const def = vuetify.theme.themes.value[name] as unknown as
    | AppThemeDefinition
    | undefined;
  if (!def?.colors) return [];
  return [
    def.colors.primary,
    def.colors.secondary,
    def.colors.surface,
    def.colors.background,
  ].filter((color): color is string => typeof color === "string");
}

/** 打开新建主题编辑器 */
function onCreate() {
  editDialogRef.value?.open();
}

/**
 * 打开编辑主题编辑器。
 * @param name 主题名
 */
function onEdit(name: string) {
  editDialogRef.value?.open(name);
}

/**
 * 导出主题 JSON 到剪贴板（用于分享）。
 * @param name 主题名
 * @param displayName 主题显示名称
 * @returns 无返回值
 */
async function onExport(name: string, displayName: string) {
  const json = exportTheme(name);
  if (json === null) return;
  try {
    await navigator.clipboard.writeText(json);
    snackbarText(t("themes.export-success", { name: displayName }), "success");
  } catch (error) {
    console.error(error);
    snackbarText(t("themes.export-failed"), "error");
  }
}

/**
 * 点击删除按钮，进入删除确认流程。
 * @param name 主题名
 * @param displayName 主题显示名称
 */
function onDeleteClick(name: string, displayName: string) {
  deleting.value = { name, displayName };
}

/** 确认删除自定义主题 */
function confirmDelete() {
  if (!deleting.value) return;
  if (removeCustomTheme(deleting.value.name)) {
    snackbarText(
      t("themes.delete-success", { name: deleting.value.displayName }),
      "success",
    );
  }
  deleting.value = null;
}

defineExpose({ open, close });
</script>

<template>
  <VDialog v-model="dialog" max-width="640">
    <VCard>
      <VCardTitle>{{ t("themes.manager-title") }}</VCardTitle>
      <VCardText>
        <div class="actions-row">
          <VBtn prepend-icon="mdi-plus" color="primary" variant="tonal" @click="onCreate">
            {{ t("themes.create") }}
          </VBtn>
          <VBtn
            prepend-icon="mdi-import"
            variant="tonal"
            @click="importDialogRef?.open()"
          >
            {{ t("themes.import") }}
          </VBtn>
        </div>
        <VList lines="two" class="theme-list">
          <VListItem v-for="item in themeList" :key="item.name">
            <template #prepend>
              <div class="swatches">
                <span
                  v-for="color in swatchesOf(item.name)"
                  :key="color"
                  class="swatch"
                  :style="{ backgroundColor: color }"
                />
              </div>
            </template>
            <VListItemTitle>{{ item.displayName }}</VListItemTitle>
            <VListItemSubtitle>
              {{
                item.builtin
                  ? t("themes.builtin-tag")
                  : `${t("themes.custom-tag")} · ${item.name}`
              }}
            </VListItemSubtitle>
            <template #append>
              <VIconBtn
                icon="mdi-export-variant"
                variant="text"
                size="small"
                :aria-label="t('themes.export')"
                @click="onExport(item.name, item.displayName)"
              />
              <VIconBtn
                v-if="!item.builtin"
                icon="mdi-pencil-outline"
                variant="text"
                size="small"
                :aria-label="t('themes.edit')"
                @click="onEdit(item.name)"
              />
              <VIconBtn
                v-if="!item.builtin"
                icon="mdi-delete-outline"
                variant="text"
                size="small"
                color="error"
                :aria-label="t('themes.delete')"
                @click="onDeleteClick(item.name, item.displayName)"
              />
            </template>
          </VListItem>
        </VList>
      </VCardText>
      <VCardActions>
        <VSpacer />
        <VBtn variant="text" @click="close">{{ t("themes.close") }}</VBtn>
      </VCardActions>
    </VCard>

    <ThemeEditDialog ref="editDialogRef" />
    <ThemeImportDialog ref="importDialogRef" />

    <VDialog
      :model-value="deleting !== null"
      max-width="420"
      persistent
      @update:model-value="deleting = null"
    >
      <VCard>
        <VCardTitle>{{ t("themes.delete-confirm-title") }}</VCardTitle>
        <VCardText>
          {{ t("themes.delete-confirm-body", { name: deleting?.displayName }) }}
        </VCardText>
        <VCardActions>
          <VSpacer />
          <VBtn variant="text" @click="deleting = null">
            {{ t("common.cancel") }}
          </VBtn>
          <VBtn color="error" variant="flat" @click="confirmDelete">
            {{ t("themes.delete") }}
          </VBtn>
        </VCardActions>
      </VCard>
    </VDialog>
  </VDialog>
</template>

<style lang="scss" scoped>
.actions-row {
  display: flex;
  gap: 0.5rem;
  margin-bottom: 0.75rem;
}

.theme-list {
  max-height: 24rem;
  overflow-y: auto;
}

.swatches {
  display: flex;
  gap: 0.25rem;
  margin-right: 0.75rem;

  .swatch {
    width: 0.875rem;
    height: 0.875rem;
    border-radius: 50%;
    border: 1px solid rgba(var(--v-border-color), var(--v-border-opacity));
  }
}
</style>
