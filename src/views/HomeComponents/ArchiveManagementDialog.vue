<!--
  用户数据库归档管理对话框。

  展示全部用户数据库及其归档状态，支持归档、还原与永久删除；
  永久删除需先在 DeleteDatabaseDialog 中确认名称与密码。
-->
<script setup lang="ts">
import { ref, useTemplateRef } from "vue";
import { t, d } from "@/i18n";
import {
  metadataArchive,
  metadataList,
  metadataPhysicalDelete,
  metadataSave,
} from "@/api";
import { isErrorCode } from "@/error-code";
import type { Metadata } from "@/api-types";
import { snackbarErrorCode, snackbarText } from "@/composables/use-snackbar";
import DeleteDatabaseDialog from "@/views/HomeComponents/DeleteDatabaseDialog.vue";

const emit = defineEmits<{
  /** 关闭对话框后通知父组件刷新数据库列表 */
  update: [];
}>();

/** 对话框显示状态 */
const dialog = ref(false);
/** 全部数据库元数据（含已归档） */
const metadatas = ref<Metadata[]>([]);
/** 列表加载状态 */
const listLoading = ref(false);
/** 操作按钮加载状态 */
const opLoading = ref(false);

const deleteDialogRef = useTemplateRef<InstanceType<typeof DeleteDatabaseDialog>>(
  "deleteDialogRef",
);

/**
 * 刷新数据库列表（合并归档与未归档，按最后打开时间降序）。
 * @returns 无返回值
 */
async function refresh() {
  if (metadatas.value.length === 0) {
    listLoading.value = true;
  }
  try {
    const [archived, unarchived] = await Promise.all([
      metadataList(true),
      metadataList(false),
    ]);
    metadatas.value = [...archived, ...unarchived].sort(
      (a, b) => b.last_open_time - a.last_open_time,
    );
  } catch (error) {
    snackbarErrorCode(error);
  } finally {
    listLoading.value = false;
  }
}

/**
 * 打开对话框并刷新列表。
 * @returns 无返回值
 */
async function open() {
  dialog.value = true;
  await refresh();
}

/** 关闭对话框 */
function close() {
  dialog.value = false;
  emit("update");
}

/**
 * 设置数据库的归档状态并刷新列表。
 * @param metadata 目标数据库元数据
 * @param archived 归档状态，true 归档，false 解除归档
 * @returns 无返回值
 */
async function setArchived(metadata: Metadata, archived: boolean) {
  opLoading.value = true;
  try {
    await metadataArchive(metadata.id, archived);
    void metadataSave().catch(snackbarErrorCode);
    const messageKey = archived
      ? "home.archive-management.archive-success"
      : "home.archive-management.unarchive-success";
    snackbarText(t(messageKey, { name: metadata.name }), "success");
    await refresh();
  } catch (error) {
    snackbarErrorCode(error);
  } finally {
    opLoading.value = false;
  }
}

/**
 * 点击删除按钮，打开删除确认对话框。
 * @param metadata 目标数据库元数据
 */
function onDeleteClick(metadata: Metadata) {
  deleteDialogRef.value?.open(metadata.id, metadata.name);
}

/**
 * 确认删除数据库。
 * @param id 数据库 id
 * @param name 数据库名称
 * @param password 数据库密码
 * @returns 无返回值
 */
async function onDeleteConfirm(id: string, name: string, password: string) {
  opLoading.value = true;
  try {
    await metadataPhysicalDelete(id, password);
    void metadataSave().catch(snackbarErrorCode);
    snackbarText(
      t("home.archive-management.delete-success", { name }),
      "success",
    );
    await refresh();
  } catch (error) {
    if (isErrorCode(error, "FailToDecrypt")) {
      snackbarText(t("home.archive-management.wrong-password"), "error");
    } else {
      snackbarErrorCode(error);
    }
  } finally {
    opLoading.value = false;
  }
}

/**
 * 列表项副标题：归档状态 + 最后打开时间。
 * @param metadata 数据库元数据
 * @returns 副标题文本
 */
function subtitleOf(metadata: Metadata): string {
  const status = metadata.archived
    ? t("home.archive-management.status-archived")
    : t("home.archive-management.status-not-archived");
  return `${status} · ${d(new Date(metadata.last_open_time), "short")}`;
}

defineExpose({ open, close });
</script>

<template>
  <VDialog v-model="dialog" max-width="600" persistent>
    <VCard>
      <VCardTitle>{{ t("home.archive-management.title") }}</VCardTitle>
      <VCardText class="list-text">
        <div v-if="listLoading" class="placeholder-text">
          <VProgressCircular indeterminate size="32" />
        </div>
        <VList v-else-if="metadatas.length > 0" lines="two">
          <VListItem
            v-for="metadata in metadatas"
            :key="metadata.id"
            :title="metadata.name"
            :subtitle="subtitleOf(metadata)"
          >
            <template #append>
              <VBtn
                v-if="!metadata.archived"
                :loading="opLoading"
                :disabled="opLoading"
                variant="text"
                color="primary"
                @click="setArchived(metadata, true)"
              >
                {{ t("home.archive-management.archive") }}
              </VBtn>
              <VBtn
                v-if="metadata.archived"
                :loading="opLoading"
                :disabled="opLoading"
                variant="text"
                color="primary"
                @click="setArchived(metadata, false)"
              >
                {{ t("home.archive-management.unarchive") }}
              </VBtn>
              <VBtn
                v-if="metadata.archived"
                :loading="opLoading"
                :disabled="opLoading"
                variant="text"
                color="error"
                @click="onDeleteClick(metadata)"
              >
                {{ t("home.archive-management.delete") }}
              </VBtn>
            </template>
          </VListItem>
        </VList>
        <div v-else class="placeholder-text">
          {{ t("home.archive-management.empty") }}
        </div>
      </VCardText>
      <VCardActions>
        <VSpacer />
        <VBtn variant="text" @click="close">
          {{ t("home.archive-management.close") }}
        </VBtn>
      </VCardActions>
    </VCard>
  </VDialog>
  <DeleteDatabaseDialog ref="deleteDialogRef" @confirm="onDeleteConfirm" />
</template>

<style lang="scss" scoped>
.list-text {
  padding-top: 0.5rem;
}

.placeholder-text {
  text-align: center;
  padding: 2rem 0;
  color: rgba(var(--v-theme-on-surface), 0.6);
}
</style>
