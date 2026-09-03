<!--
  内部共享的单个时间点端点展示与选择块（instant 系列编辑器的内部共享组件，不属于字段编辑器映射表）。
  以只读展示框展示当前值，点击弹出按精度组合的日历/滚轮悬浮选择面板（面板内容由 InstantPickerPanel 提供）；
  值以本地时间部件（LocalParts，不含时区后缀）经 v-model 传入传出，每次选择只改对应位保留其它位并立即传出。
  全 0 的 DEFAULT_LOCAL_PARTS 表示"字段值为空"，展示占位文案而非时间文本；
  readonly 时悬浮面板不可打开，仅作静态展示。
-->
<script setup lang="ts">
import { computed, ref } from "vue";
import { t } from "@/i18n";
import type { LocalParts, Precision } from "@/field-types/date-time";
import {
  assemblePartsToTime,
  DEFAULT_LOCAL_PARTS,
  sameLocalParts,
} from "@/field-types/date-time";
import { useMenuDismiss } from "@/composables/use-menu-dismiss";
import InstantPickerPanel from "./InstantPickerPanel.vue";

const props = defineProps<{
  modelValue: LocalParts;
  precision: Precision;
  /** 值错误高亮：输入控件进入错误高亮状态（不显示错误信息）。 */
  errorHighlight?: boolean;
  readonly?: boolean;
}>();

const emit = defineEmits<{
  "update:modelValue": [value: LocalParts];
}>();

/** 悬浮选择面板的展开状态。 */
const menuOpen = ref(false);
// 点击悬浮面板外部时收起面板（兜底实现，处理对话框内 VMenu 的 clickOutside bug）。
useMenuDismiss(menuOpen, ".endpoint-picker-popper");

/** 是否为空值态（全 0 的 DEFAULT_LOCAL_PARTS 表示"字段值为空"，项目既有约定）。 */
const isEmpty = computed(() => sameLocalParts(props.modelValue, DEFAULT_LOCAL_PARTS));

/** 展示框内显示的文本：空值态显示占位文案，否则按精度组装本地时间字符串（本地时间、不含时区后缀、格式即精度）。 */
const displayText = computed(() =>
  isEmpty.value
    ? t("database.field-editor.instant-empty")
    : assemblePartsToTime(props.modelValue, props.precision),
);
</script>

<template>
  <div class="endpoint-editor">
    <VMenu
      v-model="menuOpen"
      :close-on-content-click="false"
      :disabled="readonly"
      location="bottom"
    >
      <template #activator="{ props: menuProps }">
        <div
          class="endpoint-display"
          :class="{ error: errorHighlight, readonly: readonly }"
          v-bind="menuProps"
        >
          <span v-if="isEmpty" class="endpoint-placeholder">{{ displayText }}</span>
          <span v-else>{{ displayText }}</span>
        </div>
      </template>
      <VCard class="endpoint-picker-popper">
        <InstantPickerPanel
          :model-value="modelValue"
          :precision="precision"
          @update:model-value="emit('update:modelValue', $event)"
        />
      </VCard>
    </VMenu>
  </div>
</template>

<style lang="scss" scoped>
.endpoint-editor {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
}

.endpoint-display {
  display: flex;
  align-items: center;
  min-height: 2.5rem;
  padding: 0 0.75rem;
  border: 1px solid rgba(var(--v-border-color), var(--v-border-opacity));
  border-radius: 0.25rem;
  cursor: pointer;
  font-variant-numeric: tabular-nums;
}

.endpoint-display:hover {
  border-color: rgba(var(--v-border-color), 1);
}

.endpoint-display.error {
  border-color: rgb(var(--v-theme-error));
  color: rgb(var(--v-theme-error));
}

.endpoint-display.readonly {
  cursor: default;
}

.endpoint-display.readonly:hover {
  border-color: rgba(var(--v-border-color), var(--v-border-opacity));
}

.endpoint-placeholder {
  color: rgb(var(--v-theme-secondary));
}
</style>