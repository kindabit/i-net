<!--
  日志对话框。

  分页查看当前用户数据库的操作日志，每页 20 条，按时间倒序。
  defineExpose 暴露 open() / close() 供父组件控制显隐。
-->
<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { d, t, te } from "@/i18n";
import { userDatabaseLogList } from "@/api";
import { snackbarErrorCode } from "@/composables/use-snackbar";
import type { LogListResponse, NodeFieldChange, FieldValue } from "@/api-types";
import { getFieldTypeDef, formatValueForDisplay } from "@/field-types";

const PAGE_SIZE = 20;

const dialog = ref(false);
const page = ref(1);
const total = ref(0);
const items = ref<LogListResponse[]>([]);
const loading = ref(false);
const showSensitive = ref(false);

const totalPages = computed(() => Math.ceil(total.value / PAGE_SIZE));

function detailParams(entry: LogListResponse): Record<string, string> {
  return Object.fromEntries(
    Object.entries(entry.action.data).map(([k, v]) => [k, String(v)]),
  );
}

function isNodeFieldsModify(entry: LogListResponse): boolean {
  return entry.action.variant === "NodeFieldsModify";
}

function getNodeFieldsModifyData(entry: LogListResponse) {
  return entry.action.data as unknown as {
    node_title: string;
    changes: NodeFieldChange[];
  };
}

function formatChangeValue(value: unknown, masked: boolean): string {
  const fv = value as FieldValue;
  if (!fv || fv.data === null) return t("log.empty-value");
  if (masked && !showSensitive.value) return "\u2022\u2022\u2022\u2022\u2022\u2022\u2022\u2022";
  return formatValueForDisplay(fv);
}

function renderChange(change: NodeFieldChange): string {
  switch (change.variant) {
    case "Added": {
      const def = getFieldTypeDef(change.data.field_type);
      const typeName = def
        ? t(`database.field-type.${def.i18nKey}`)
        : change.data.field_type;
      const masked = def?.masked ?? false;
      const value = formatChangeValue(change.data.value, masked);
      return t("log.node-fields-change-added", {
        name: change.data.name,
        type: typeName,
        value,
      });
    }
    case "Modified": {
      const oldDef = getFieldTypeDef(change.data.old_field_type);
      const newDef = getFieldTypeDef(change.data.new_field_type);
      const oldTypeName = oldDef
        ? t(`database.field-type.${oldDef.i18nKey}`)
        : change.data.old_field_type;
      const newTypeName = newDef
        ? t(`database.field-type.${newDef.i18nKey}`)
        : change.data.new_field_type;
      const masked = (oldDef?.masked || newDef?.masked) ?? false;
      const oldValue = formatChangeValue(change.data.old_value, masked);
      const newValue = formatChangeValue(change.data.new_value, masked);
      return t("log.node-fields-change-modified", {
        name: change.data.name,
        oldType: oldTypeName,
        newType: newTypeName,
        oldValue,
        newValue,
      });
    }
    case "Removed": {
      const def = getFieldTypeDef(change.data.field_type);
      const typeName = def
        ? t(`database.field-type.${def.i18nKey}`)
        : change.data.field_type;
      const masked = def?.masked ?? false;
      const oldValue = formatChangeValue(change.data.old_value, masked);
      return t("log.node-fields-change-removed", {
        name: change.data.name,
        type: typeName,
        oldValue,
      });
    }
  }
}

async function load() {
  loading.value = true;
  try {
    const result = await userDatabaseLogList(
      (page.value - 1) * PAGE_SIZE,
      PAGE_SIZE,
    );
    items.value = result.items;
    total.value = result.total;
  } catch (error) {
    snackbarErrorCode(error);
  } finally {
    loading.value = false;
  }
}

function open() {
  dialog.value = true;
  if (page.value !== 1) {
    page.value = 1;
  } else {
    void load();
  }
}

function close() {
  dialog.value = false;
}

watch(page, () => {
  if (dialog.value) {
    void load();
  }
});

defineExpose({ open, close });
</script>

<template>
  <VDialog v-model="dialog" width="45rem" scrollable>
    <VCard>
      <VCardTitle class="d-flex align-center">
        {{ t("log.dialog-title") }}
        <VSpacer />
        <span class="text-caption text-secondary">
          {{ t("log.total", { total }) }}
        </span>
      </VCardTitle>
      <VDivider />
      <div class="d-flex align-center px-4 py-1">
        <VSpacer />
        <VSwitch
          v-model="showSensitive"
          :label="t('log.show-sensitive-values')"
          density="compact"
          hide-details
        />
      </div>
      <VDivider />
      <VCardText class="log-list-container">
        <VProgressCircular
          v-if="loading"
          indeterminate
          class="ma-auto d-block"
        />
        <p
          v-else-if="items.length === 0"
          class="text-center text-secondary mt-4"
        >
          {{ t("log.empty") }}
        </p>
        <VList v-else class="pa-0">
          <VListItem v-for="entry in items" :key="entry.id">
            <template v-if="isNodeFieldsModify(entry)">
              <div>
                <div class="text-body-2 font-weight-medium">
                  {{ t("log.action.NodeFieldsModify.name") }}
                </div>
                <div class="text-body-2">
                  {{
                    t("log.action.NodeFieldsModify.detail", {
                      node_title: getNodeFieldsModifyData(entry).node_title,
                    })
                  }}
                </div>
                <div class="log-changes ml-4 mt-1">
                  <div
                    v-for="(change, ci) in getNodeFieldsModifyData(entry)
                      .changes"
                    :key="ci"
                    class="text-caption log-change-item"
                  >
                    {{ renderChange(change) }}
                  </div>
                </div>
              </div>
            </template>
            <template v-else>
              <VListItemTitle>
                {{
                  te(`log.action.${entry.action.variant}.name`)
                    ? t(`log.action.${entry.action.variant}.name`)
                    : entry.action.variant
                }}
              </VListItemTitle>
              <VListItemSubtitle>
                {{
                  te(`log.action.${entry.action.variant}.detail`)
                    ? t(
                        `log.action.${entry.action.variant}.detail`,
                        detailParams(entry),
                      )
                    : ""
                }}
              </VListItemSubtitle>
            </template>
            <template #append>
              <span class="text-caption text-secondary">
                {{ d(new Date(entry.time), "short") }}
              </span>
            </template>
          </VListItem>
        </VList>
      </VCardText>
      <VDivider />
      <VCardActions>
        <VPagination
          v-if="totalPages > 0"
          v-model="page"
          :length="totalPages"
          :disabled="loading"
          density="compact"
          :total-visible="7"
        />
        <VSpacer />
        <VBtn variant="text" @click="close">{{ t("common.close") }}</VBtn>
      </VCardActions>
    </VCard>
  </VDialog>
</template>

<style lang="scss" scoped>
.log-list-container {
  min-height: 30rem;
  max-height: 30rem;
  overflow-y: auto;
  padding: 0;
}

.log-changes {
  .log-change-item {
    padding-top: 0.125rem;
    padding-bottom: 0.125rem;
    color: rgba(var(--v-theme-on-surface), var(--v-medium-emphasis-opacity));
  }
}
</style>
