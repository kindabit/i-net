<!--
  日志查看与搜索对话框。

  分页查看当前用户数据库的操作日志（每页 20 条，按时间倒序），
  并支持按时间范围、行为类型与内容关键词筛选日志。
  defineExpose 暴露 open() / close() 供父组件控制显隐。
-->
<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { d, t, te, tm, currentLocale } from "@/i18n";
import { userDatabaseLogList } from "@/api";
import { snackbarErrorCode, snackbarText } from "@/composables/use-snackbar";
import type { LogListResponse, NodeFieldChange } from "@/api-types";
import { fieldTypeDisplayName, isFieldTypeMasked } from "@/field-types";

const PAGE_SIZE = 20;

const dialog = ref(false);
const page = ref(1);
const total = ref(0);
const items = ref<LogListResponse[]>([]);
const loading = ref(false);
const showSensitive = ref(false);

// 输入中的筛选：用户可在输入框中自由修改，未触发搜索前不影响列表。
const filterStartDate = ref("");
const filterEndDate = ref("");
const filterActions = ref<string[]>([]);
const filterKeyword = ref("");

// 已生效的筛选：仅由搜索与清除筛选操作修改，load() 始终使用这些值。
const appliedStartTime = ref<number | null>(null);
const appliedEndTime = ref<number | null>(null);
const appliedActions = ref<string[] | null>(null);
const appliedKeyword = ref<string | null>(null);

const totalPages = computed(() => Math.ceil(total.value / PAGE_SIZE));

/**
 * 判断是否已生效任意一项筛选条件。
 * @returns 已生效任意一项筛选时返回 true，否则返回 false
 */
const hasAppliedFilter = computed(
  () =>
    appliedStartTime.value !== null ||
    appliedEndTime.value !== null ||
    (appliedActions.value?.length ?? 0) > 0 ||
    appliedKeyword.value !== null,
);

/**
 * 计算行为类型筛选下拉的选项列表。
 * 选项取自 i18n 资源 log.action 下的 key，语言切换时文案随之更新。
 * @returns 行为类型选项列表（title 为本地化名称，value 为 variant 名）
 */
const actionOptions = computed(() => {
  void currentLocale.value;
  return Object.keys(tm("log.action")).map((variant) => ({
    title: t(`log.action.${variant}.name`),
    value: variant,
  }));
});

/**
 * 将日志条目的 action.data 转换为 i18n 插值参数。
 * @param entry 日志条目
 * @returns 键值对形式的插值参数对象
 */
function detailParams(entry: LogListResponse): Record<string, string> {
  return Object.fromEntries(
    Object.entries(entry.action.data).map(([k, v]) => [k, String(v)]),
  );
}

/**
 * 判断日志条目是否为节点字段修改操作。
 * @param entry 日志条目
 * @returns 是节点字段修改操作时返回 true，否则返回 false
 */
function isNodeFieldsModify(entry: LogListResponse): boolean {
  return entry.action.variant === "NodeFieldsModify";
}

/**
 * 从日志条目中提取节点字段修改的详细数据。
 * @param entry 日志条目，必须为 NodeFieldsModify 类型
 * @returns 包含节点标题和变更列表的数据对象
 */
function getNodeFieldsModifyData(entry: LogListResponse) {
  return entry.action.data as unknown as {
    node_title: string;
    changes: NodeFieldChange[];
  };
}

/**
 * 格式化日志中的字段变更值：无值显示空值文案；掩码且未开启明文显示时显示圆点；否则直接显示值原文。
 * @param value 字段值字符串
 * @param masked 该字段类型是否掩码显示
 * @returns 用于展示的字符串
 */
function formatChangeValue(value: unknown, masked: boolean): string {
  const str = value as string | null;
  if (str === null || str === undefined) return t("log.empty-value");
  if (masked && !showSensitive.value) return "\u2022\u2022\u2022\u2022\u2022\u2022\u2022\u2022";
  return str;
}

/**
 * 渲染单条字段变更的日志文案。
 * @param change 节点字段变更对象
 * @returns 可直接展示的文案字符串
 */
function renderChange(change: NodeFieldChange): string {
  switch (change.variant) {
    case "Added": {
      const typeName = fieldTypeDisplayName(change.data.field_type);
      const masked = isFieldTypeMasked(change.data.field_type);
      const value = formatChangeValue(change.data.value, masked);
      return t("log.node-fields-change-added", {
        name: change.data.name,
        type: typeName,
        value,
      });
    }
    case "Modified": {
      const oldTypeName = fieldTypeDisplayName(change.data.old_field_type);
      const newTypeName = fieldTypeDisplayName(change.data.new_field_type);
      // 新旧类型任一为掩码类型时，新旧值均掩码显示。
      const masked =
        isFieldTypeMasked(change.data.old_field_type) ||
        isFieldTypeMasked(change.data.new_field_type);
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
      const typeName = fieldTypeDisplayName(change.data.field_type);
      const masked = isFieldTypeMasked(change.data.field_type);
      const oldValue = formatChangeValue(change.data.old_value, masked);
      return t("log.node-fields-change-removed", {
        name: change.data.name,
        type: typeName,
        oldValue,
      });
    }
  }
}

/**
 * 加载当前页的日志数据。
 * 异步获取日志列表，更新 items 和 total，出错时显示错误提示。
 */
async function load() {
  loading.value = true;
  try {
    const result = await userDatabaseLogList(
      (page.value - 1) * PAGE_SIZE,
      PAGE_SIZE,
      {
        startTime: appliedStartTime.value,
        endTime: appliedEndTime.value,
        actions: appliedActions.value,
        keyword: appliedKeyword.value,
      },
    );
    items.value = result.items;
    total.value = result.total;
  } catch (error) {
    snackbarErrorCode(error);
  } finally {
    loading.value = false;
  }
}

/**
 * 校验输入中的日期范围是否合理（开始日期不晚于结束日期）。
 * @returns 日期范围合理时返回 true；不合理时提示用户并返回 false
 */
function validateDateRange(): boolean {
  if (filterStartDate.value && filterEndDate.value) {
    const start = new Date(filterStartDate.value + "T00:00:00.000").getTime();
    const end = new Date(filterEndDate.value + "T23:59:59.999").getTime();
    if (start > end) {
      snackbarText(t("log.invalid-time-range"), "warning");
      return false;
    }
  }
  return true;
}

/**
 * 将输入中的筛选值换算为已生效的筛选值。
 * 日期按日期级粒度换算为本地时区的毫秒时间戳；空输入换算为不限（null）。
 */
function applyInputFilter() {
  appliedStartTime.value = filterStartDate.value
    ? new Date(filterStartDate.value + "T00:00:00.000").getTime()
    : null;
  appliedEndTime.value = filterEndDate.value
    ? new Date(filterEndDate.value + "T23:59:59.999").getTime()
    : null;
  appliedActions.value = filterActions.value.length
    ? [...filterActions.value]
    : null;
  appliedKeyword.value = filterKeyword.value.trim() || null;
}

/**
 * 触发搜索：校验日期范围后，将输入中的筛选值换算为已生效值并回到第一页重新加载。
 */
function search() {
  if (!validateDateRange()) return;
  applyInputFilter();
  if (page.value !== 1) {
    page.value = 1;
  } else {
    void load();
  }
}

/**
 * 清除筛选：清空全部输入中与已生效的筛选值，并回到第一页重新加载。
 */
function clearFilter() {
  filterStartDate.value = "";
  filterEndDate.value = "";
  filterActions.value = [];
  filterKeyword.value = "";
  appliedStartTime.value = null;
  appliedEndTime.value = null;
  appliedActions.value = null;
  appliedKeyword.value = null;
  if (page.value !== 1) {
    page.value = 1;
  } else {
    void load();
  }
}

/**
 * 打开日志对话框，并重置到第一页开始加载日志。
 */
function open() {
  dialog.value = true;
  filterStartDate.value = "";
  filterEndDate.value = "";
  filterActions.value = [];
  filterKeyword.value = "";
  appliedStartTime.value = null;
  appliedEndTime.value = null;
  appliedActions.value = null;
  appliedKeyword.value = null;
  if (page.value !== 1) {
    page.value = 1;
  } else {
    void load();
  }
}

/**
 * 关闭日志对话框。
 */
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
      <div class="d-flex flex-column ga-2 px-4 py-2 log-filter">
        <div class="d-flex align-center ga-2 log-filter-row">
          <VTextField
            v-model="filterStartDate"
            type="date"
            :label="t('log.filter-start-date')"
            density="compact"
            hide-details
            class="log-date-input"
          />
          <VTextField
            v-model="filterEndDate"
            type="date"
            :label="t('log.filter-end-date')"
            density="compact"
            hide-details
            class="log-date-input"
          />
          <VSelect
            v-model="filterActions"
            :items="actionOptions"
            :label="t('log.filter-actions')"
            multiple
            chips
            closable-chips
            clearable
            density="compact"
            hide-details
            class="log-action-select"
          />
        </div>
        <div class="d-flex align-center ga-2 log-filter-row">
          <VTextField
            v-model="filterKeyword"
            :label="t('log.search-placeholder')"
            prepend-inner-icon="mdi-magnify"
            clearable
            density="compact"
            hide-details
            class="log-search-input"
            @keyup.enter="search"
          />
          <VBtn variant="tonal" @click="search">{{ t("log.search") }}</VBtn>
          <VBtn variant="text" @click="clearFilter">
            {{ t("log.clear-filter") }}
          </VBtn>
          <VSpacer />
          <VSwitch
            v-model="showSensitive"
            :label="t('log.show-sensitive-values')"
            density="compact"
            hide-details
          />
        </div>
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
          {{ hasAppliedFilter ? t("log.empty-search") : t("log.empty") }}
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
.log-filter {
  white-space: nowrap;

  .log-date-input {
    flex: 0 0 12rem;
    width: 12rem;
  }

  .log-action-select {
    flex: 1 1 auto;
    min-width: 0;

    // 行为类型选中的 chips 单行显示不换行，溢出时横向滚动。
    :deep(.v-field__input) {
      flex-wrap: nowrap;
      overflow-x: auto;
    }

    :deep(.v-chip) {
      flex: 0 0 auto;
    }
  }

  .log-search-input {
    flex: 1 1 auto;
    min-width: 0;
  }
}

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