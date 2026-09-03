<!--
  instant 悬浮选择面板的内容组件。

  按精度组合选择器："年"精度显示年份选择面板（YearPicker），"月"精度显示月份选择面板（MonthPicker），
  "日"精度仅显示月历（DateCalendar），"时"及更高精度显示月历加小时起的数字滚轮列（WheelColumn）。
  所有组合逻辑收敛在本组件：每次选择只修改对应位、保留其它位并立即整体 emit（无确认按钮）。
-->
<script setup lang="ts">
import { computed } from "vue";
import { t } from "@/i18n";
import type { LocalParts, Precision } from "@/field-types/date-time";
import { DEFAULT_LOCAL_PARTS, sameLocalParts } from "@/field-types/date-time";
import WheelColumn from "./WheelColumn.vue";
import DateCalendar from "./DateCalendar.vue";
import YearPicker from "./YearPicker.vue";
import MonthPicker from "./MonthPicker.vue";

const props = defineProps<{
  modelValue: LocalParts;
  precision: Precision;
}>();

const emit = defineEmits<{
  "update:modelValue": [value: LocalParts];
}>();

/** 全部时间位从年到毫秒的顺序，索引即精度等级。 */
const PART_ORDER: readonly Precision[] = [
  "year",
  "month",
  "day",
  "hour",
  "minute",
  "second",
  "millisecond",
];

/** 各滚轮列的取值与显示配置（滚轮列只会出现在 hour 及更高精度，year/month/day 条目仅为保持类型完整）。 */
const WHEEL_PART_CONFIG: Record<Precision, { min: number; max: number; digits: number }> = {
  year: { min: 0, max: 9999, digits: 4 },
  month: { min: 1, max: 12, digits: 2 },
  day: { min: 1, max: 31, digits: 2 },
  hour: { min: 0, max: 23, digits: 2 },
  minute: { min: 0, max: 59, digits: 2 },
  second: { min: 0, max: 59, digits: 2 },
  millisecond: { min: 0, max: 999, digits: 3 },
};

/** 当前精度下需要显示的滚轮列：hour 及以上精度从 hour 起到该精度位，更低精度（年/月/日）无滚轮列。 */
const visibleWheels = computed<readonly Precision[]>(() => {
  const precisionIndex = PART_ORDER.indexOf(props.precision);
  const hourIndex = PART_ORDER.indexOf("hour");
  if (precisionIndex < hourIndex) return [];
  return PART_ORDER.slice(hourIndex, precisionIndex + 1);
});

/** 字段值是否非空（全 0 的 DEFAULT_LOCAL_PARTS 为"字段值为空"的项目既有约定）。 */
const hasValue = computed(() => !sameLocalParts(props.modelValue, DEFAULT_LOCAL_PARTS));

/**
 * 滚轮列更新：只修改对应位并保留其它位，立即整体 emit。
 * @param part 被修改的时间位
 * @param val 该位的新值
 */
function onWheel(part: Precision, val: number): void {
  emit("update:modelValue", { ...props.modelValue, [part]: val });
}

/**
 * 月历选择更新：覆盖年月日并保留其它位，立即整体 emit。
 * @param date 新选中的年月日
 */
function onCalendarSelect(date: { year: number; month: number; day: number }): void {
  emit("update:modelValue", { ...props.modelValue, ...date });
}

/**
 * 年份选择更新：覆盖年份并保留其它位，立即整体 emit。
 * @param year 新选中的年份
 */
function onYearSelect(year: number): void {
  emit("update:modelValue", { ...props.modelValue, year });
}

/**
 * 月份选择更新：覆盖年月并保留其它位，立即整体 emit。
 * @param date 新选中的年月
 */
function onMonthSelect(date: { year: number; month: number }): void {
  emit("update:modelValue", { ...props.modelValue, ...date });
}
</script>

<template>
  <div class="instant-picker-panel">
    <YearPicker
      v-if="precision === 'year'"
      :year="modelValue.year"
      :has-value="hasValue"
      @select="onYearSelect"
    />
    <MonthPicker
      v-else-if="precision === 'month'"
      :year="modelValue.year"
      :month="modelValue.month"
      :has-value="hasValue"
      @select="onMonthSelect"
    />
    <template v-else>
      <DateCalendar
        :year="modelValue.year"
        :month="modelValue.month"
        :day="modelValue.day"
        @select="onCalendarSelect"
      />
      <div v-if="visibleWheels.length > 0" class="wheel-group">
        <div v-for="part in visibleWheels" :key="part" class="wheel-column-wrap">
          <div class="text-caption text-secondary">{{ t(`database.field-type.precision-${part}`) }}</div>
          <WheelColumn
            :model-value="modelValue[part]"
            :min="WHEEL_PART_CONFIG[part].min"
            :max="WHEEL_PART_CONFIG[part].max"
            :digits="WHEEL_PART_CONFIG[part].digits"
            @update:model-value="(val: number) => onWheel(part, val)"
          />
        </div>
      </div>
    </template>
  </div>
</template>

<style lang="scss" scoped>
.instant-picker-panel {
  display: flex;
  gap: 0.75rem;
  padding: 0.75rem;
  align-items: stretch;
}

.wheel-group {
  display: flex;
  gap: 0.25rem;
}

.wheel-column-wrap {
  display: flex;
  flex-direction: column;
  align-items: stretch;
  gap: 0.25rem;
}

.wheel-column-wrap > .text-caption {
  text-align: center;
}
</style>