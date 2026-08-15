<script setup lang="ts">
import { ref, watch, nextTick, computed } from "vue";
import { t } from "@/i18n";
import {
  userDatabaseTemplateList,
  userDatabaseTemplateCreate,
  userDatabaseTemplateRename,
  userDatabaseTemplateDelete,
  userDatabaseTemplateGetFields,
  userDatabaseTemplateSetFields,
  userDatabaseTemplateExport,
  userDatabaseTemplateImport,
} from "@/api";
import type { Template } from "@/api-types";
import { snackbarErrorCode, snackbarText } from "@/composables/use-snackbar";
import { useFieldList } from "@/composables/use-field-list";
import FieldDefinitionRow from "@/components/FieldDefinitionRow.vue";
import NameInputDialog from "@/components/NameInputDialog.vue";
import ConfirmDialog from "@/components/ConfirmDialog.vue";

const dialog = ref(false);
const templates = ref<Template[]>([]);
const selectedId = ref<string | null>(null);
const loadingFields = ref(false);
const saving = ref(false);
const loadingData = ref(false);
const draggingUid = ref<number | null>(null);
const confirmingClose = ref(false);

const fieldList = useFieldList({ withValues: false });
const nameInputDialogRef = ref<InstanceType<typeof NameInputDialog>>();
const confirmDialogRef = ref<InstanceType<typeof ConfirmDialog>>();

const selectedTemplate = computed(() =>
  templates.value.find((t) => t.id === selectedId.value),
);

function open() {
  selectedId.value = null;
  dialog.value = true;
  loadData();
}

async function loadData() {
  loadingData.value = true;
  try {
    const tmpls = await userDatabaseTemplateList();
    templates.value = tmpls;
    if (tmpls.length > 0 && !selectedId.value) {
      selectedId.value = tmpls[0].id;
      await loadFields(selectedId.value);
    }
  } catch (e) {
    snackbarErrorCode(e);
  } finally {
    loadingData.value = false;
  }
}

async function loadFields(id: string) {
  loadingFields.value = true;
  try {
    const fields = await userDatabaseTemplateGetFields(id);
    fieldList.loadFromTemplateFields(fields);
  } catch (e) {
    snackbarErrorCode(e);
  } finally {
    loadingFields.value = false;
  }
}

async function selectTemplate(id: string) {
  if (selectedId.value === id) return;
  if (fieldList.isDirty()) {
    const confirmed = await confirmDialogRef.value?.open({
      title: t("database.canvas.unsaved-changes-title"),
      text: t("database.canvas.unsaved-changes-text"),
      confirmColor: "error",
    });
    if (!confirmed) return;
  }
  selectedId.value = id;
  await loadFields(id);
}

async function onNewTemplate() {
  const name = await nameInputDialogRef.value?.open({
    title: t("database.canvas.new-template"),
    label: t("database.canvas.template-name-label"),
  });
  if (!name) return;
  try {
    const created = await userDatabaseTemplateCreate(name);
    await refreshTemplates();
    selectedId.value = created.id;
    await loadFields(created.id);
  } catch (e) {
    snackbarErrorCode(e);
  }
}

async function onRename() {
  const current = templates.value.find((t) => t.id === selectedId.value);
  if (!current) return;
  const newName = await nameInputDialogRef.value?.open({
    title: t("database.canvas.rename-template"),
    label: t("database.canvas.template-name-label"),
    initialValue: current.name,
  });
  if (!newName) return;
  try {
    await userDatabaseTemplateRename(current.id, newName);
    await refreshTemplates();
  } catch (e) {
    snackbarErrorCode(e);
  }
}

async function onDelete() {
  const current = templates.value.find((t) => t.id === selectedId.value);
  if (!current) return;
  const confirmed = await confirmDialogRef.value?.open({
    title: t("database.canvas.delete-template"),
    text: t("database.canvas.delete-template-text", { name: current.name }),
    confirmColor: "error",
  });
  if (!confirmed) return;
  try {
    await userDatabaseTemplateDelete(current.id);
    await refreshTemplates();
    selectedId.value = null;
  } catch (e) {
    snackbarErrorCode(e);
  }
}

async function onSave() {
  if (!fieldList.validate()) return;
  if (!selectedId.value) return;
  saving.value = true;
  try {
    const fields = fieldList.toTemplateFieldVOs();
    await userDatabaseTemplateSetFields(selectedId.value, fields);
    fieldList.loadFromTemplateFields(fields);
    snackbarText(t("database.canvas.template-saved"), "success");
  } catch (e) {
    snackbarErrorCode(e);
  } finally {
    saving.value = false;
  }
}

async function refreshTemplates() {
  const currentId = selectedId.value;
  try {
    templates.value = await userDatabaseTemplateList();
    if (
      currentId &&
      !templates.value.some((t) => t.id === currentId)
    ) {
      selectedId.value = null;
    }
  } catch (e) {
    snackbarErrorCode(e);
  }
}

/**
 * 导出模板数据：由后端弹出系统保存对话框，导出成功后提示；取消静默返回。
 * 无输入参数，无返回值。
 */
async function onExport(): Promise<void> {
  try {
    const exported = await userDatabaseTemplateExport();
    if (!exported) return;
    snackbarText(t("database.canvas.templates-exported"), "success");
  } catch (e) {
    snackbarErrorCode(e);
  }
}

/**
 * 导入模板数据：需用户确认后，由后端弹出系统文件选择对话框，导入成功后刷新并提示；取消静默返回。
 * 无输入参数，无返回值。
 */
async function onImport(): Promise<void> {
  const confirmed = await confirmDialogRef.value?.open({
    title: t("database.canvas.import-templates"),
    text: t("database.canvas.import-templates-confirm"),
    confirmColor: "warning",
  });
  if (!confirmed) return;
  try {
    const imported = await userDatabaseTemplateImport();
    if (!imported) return;
    snackbarText(t("database.canvas.templates-imported"), "success");
    selectedId.value = null;
    await loadData();
  } catch (e) {
    snackbarErrorCode(e);
  }
}

function onDragStart(uid: number) {
  draggingUid.value = uid;
}

function onDropOn(uid: number) {
  if (draggingUid.value !== null) {
    fieldList.moveRow(draggingUid.value, uid);
    draggingUid.value = null;
  }
}

watch(dialog, (v) => {
  if (!v && fieldList.isDirty() && !confirmingClose.value) {
    confirmingClose.value = true;
    dialog.value = true;
    confirmDialogRef.value
      ?.open({
        title: t("database.canvas.unsaved-changes-title"),
        text: t("database.canvas.unsaved-changes-text"),
        confirmColor: "error",
      })
      .then((confirmed) => {
        if (confirmed) {
          dialog.value = false;
          nextTick(() => {
            confirmingClose.value = false;
          });
        } else {
          confirmingClose.value = false;
        }
      });
  }
});

defineExpose({ open });
</script>

<template>
  <VDialog v-model="dialog" max-width="720">
    <VCard>
      <VCardTitle>{{ t("database.canvas.template-manager-title") }}</VCardTitle>
      <VCardText class="template-manager-card-text">
        <div v-if="loadingData" class="d-flex justify-center py-8">
          <VProgressCircular indeterminate color="primary" />
        </div>
        <div v-else class="template-manager-body">
          <div class="template-manager-left">
            <VBtn
              block
              variant="tonal"
              prepend-icon="mdi-plus"
              class="template-new-btn"
              @click="onNewTemplate"
            >
              {{ t("database.canvas.new-template") }}
            </VBtn>
            <VList density="compact" class="template-list mt-2">
              <VListItem
                v-for="tpl in templates"
                :key="tpl.id"
                :title="tpl.name"
                :active="selectedId === tpl.id"
                @click="selectTemplate(tpl.id)"
              />
            </VList>
          </div>
          <div class="template-manager-right">
            <div
              v-if="!selectedId"
              class="template-manager-right-empty"
            >
              {{ t("database.canvas.no-template-selected") }}
            </div>
            <template v-else>
              <div class="template-toolbar">
                <span class="template-name-title">{{ selectedTemplate?.name }}</span>
                <VSpacer />
                <VBtn
                  icon="mdi-pencil-outline"
                  variant="text"
                  density="compact"
                  :title="t('database.canvas.rename-template')"
                  @click="onRename"
                />
                <VBtn
                  icon="mdi-delete-outline"
                  variant="text"
                  density="compact"
                  color="error"
                  :title="t('database.canvas.delete-template')"
                  @click="onDelete"
                />
              </div>
              <div class="template-fields">
                <div v-if="loadingFields" class="d-flex justify-center py-8">
                  <VProgressCircular indeterminate color="primary" />
                </div>
                <template v-else>
                  <div
                    v-for="row in fieldList.rows.value"
                    :key="row.uid"
                    class="mb-3"
                  >
                    <FieldDefinitionRow
                      :row="row"
                      :with-values="false"
                      :name-error="fieldList.errors.value.get(row.uid)?.name"
                      @remove="fieldList.removeRow"
                      @drag-start="onDragStart"
                      @drop-on="onDropOn"
                    />
                  </div>
                </template>
              </div>
              <VBtn
                variant="text"
                prepend-icon="mdi-plus"
                class="template-add-field-btn"
                @click="fieldList.addRow()"
              >
                {{ t("database.field.add-field") }}
              </VBtn>
            </template>
          </div>
        </div>
      </VCardText>
      <VCardActions>
        <VBtn variant="text" prepend-icon="mdi-database-import" @click="onImport">
          {{ t("database.canvas.import-templates") }}
        </VBtn>
        <VBtn variant="text" prepend-icon="mdi-database-export" @click="onExport">
          {{ t("database.canvas.export-templates") }}
        </VBtn>
        <VSpacer />
        <VBtn variant="text" @click="dialog = false">
          {{ t("common.cancel") }}
        </VBtn>
        <VBtn color="primary" :disabled="!fieldList.isDirty()" :loading="saving" @click="onSave">
          {{ t("common.save") }}
        </VBtn>
      </VCardActions>
    </VCard>
    <NameInputDialog ref="nameInputDialogRef" />
    <ConfirmDialog ref="confirmDialogRef" />
  </VDialog>
</template>

<style lang="scss" scoped>
.template-manager-card-text {
  max-height: 60vh;
  overflow: hidden;
}

.template-manager-body {
  display: flex;
  gap: 1rem;
  min-height: 20rem;
  /* 与 VCardText 默认上下 padding（各 1rem）配套，保证 body 封顶后由左右栏内部滚动 */
  max-height: calc(60vh - 2rem);
  overflow: hidden;
}

.template-manager-left {
  width: 13rem;
  flex: none;
  display: flex;
  flex-direction: column;
  border-right: 1px solid rgba(var(--v-theme-on-surface), 0.12);
  padding-right: 1rem;

  /* Vuetify 的 .v-btn--block 自带 flex: 1 0 auto，在 flex 列容器中会被拉长，此处禁用伸展 */
  .template-new-btn {
    flex: none;
  }
}

.template-list {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
}

.template-manager-right {
  flex: 1;
  display: flex;
  flex-direction: column;
}

.template-manager-right-empty {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  opacity: 0.6;
  color: rgb(var(--v-theme-on-surface));
}

.template-toolbar {
  display: flex;
  align-items: center;
  gap: 0.25rem;
  margin-bottom: 1rem;
  flex-shrink: 0;
}

.template-fields {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  /* 为首行 outlined 输入框向上突出的 label 留出空间，避免被滚动区上边界裁剪 */
  padding-top: 0.5rem;
}

.template-add-field-btn {
  flex-shrink: 0;
  align-self: flex-start;
}

.template-name-title {
  font-weight: 600;
}

.mb-3 {
  margin-bottom: 0.75rem;
}

.mt-2 {
  margin-top: 0.5rem;
}

.py-8 {
  padding-top: 2rem;
  padding-bottom: 2rem;
}
</style>
