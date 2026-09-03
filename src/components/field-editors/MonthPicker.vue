<!--
  月份选择面板（instant 悬浮选择面板的内部组件，用于"月"精度）。

  以类似日历的网格方式选择年月：3 行乘 4 列共 12 个月份格子（月份名按当前语言本地化），
  头部左右箭头按年翻动（年份范围 0-9999，到达边界时对应箭头禁用），头部标题显示当前年份。
  空值态（hasValue 为假）下视图初始化到当前本地年份且无选中高亮。
-->
<script setup lang="ts">
import { computed, ref } from "vue";
import { t, currentLocale } from "@/i18n";

/** 年份下界。 */
const MIN_YEAR = 0;
/** 年份上界。 */
const MAX_YEAR = 9999;

const props = defineProps<{
  /** 当前选中的年份。 */
  year: number;
  /** 当前选中的月份（1-12）。 */
  month: number;
  /** 字段值是否非空（全 0 部件为"字段值为空"的项目约定）；空值态下不高亮选中项。 */
  hasValue?: boolean;
}>();

const emit = defineEmits<{
  select: [date: { year: number; month: number }];
}>();

/** 视图显示的年份：有合法值时取选中年份，否则取当前本地年份。 */
const viewYear = ref(
  props.hasValue && props.year >= MIN_YEAR && props.year <= MAX_YEAR
    ? props.year
    : new Date().getFullYear(),
);

/** 本地化月份名格式化器（参考日期固定为 2000 年，安全）。 */
const monthNameFormatter = computed(
  () => new Intl.DateTimeFormat(currentLocale.value, { month: "short" }),
);

/** 12 个月份格子的本地化名称。 */
const monthNames = computed(() =>
  Array.from({ length: 12 }, (_, i) =>
    monthNameFormatter.value.format(new Date(2000, i, 1)),
  ),
);

/** 头部标题：当前视图年份。 */
const viewTitle = computed(() =>
  t("database.field-editor.calendar-year-title", { year: viewYear.value }),
);

/** 上一年箭头是否禁用（视图已到年份下界）。 */
const prevDisabled = computed(() => viewYear.value <= MIN_YEAR);
/** 下一年箭头是否禁用（视图已到年份上界）。 */
const nextDisabled = computed(() => viewYear.value >= MAX_YEAR);

/**
 * 按年翻动视图；越出 0-9999 边界时不翻动。
 * @param delta 年份增减量（正数向后、负数向前）
 */
function changeYear(delta: number): void {
  const next = viewYear.value + delta;
  if (next < MIN_YEAR || next > MAX_YEAR) return;
  viewYear.value = next;
}

/**
 * 判断月份格子是否为当前选中的年月（空值态下无选中）。
 * @param month 格子对应的月份（1-12）
 * @returns 与选中年月完全相等且非空值态时返回 true
 */
function isSelected(month: number): boolean {
  return (
    (props.hasValue ?? false) && viewYear.value === props.year && month === props.month
  );
}

/**
 * 处理月份格子点击：emit 当前视图年份与被点击的月份。
 * @param month 被点击的月份（1-12）
 */
function onCellClick(month: number): void {
  emit("select", { year: viewYear.value, month });
}
</script>

<template>
  <div class="month-picker">
    <div class="picker-header">
      <VBtn
        icon="mdi-chevron-left"
        variant="text"
        density="compact"
        :disabled="prevDisabled"
        @click="changeYear(-1)"
      />
      <div class="picker-title">{{ viewTitle }}</div>
      <VBtn
        icon="mdi-chevron-right"
        variant="text"
        density="compact"
        :disabled="nextDisabled"
        @click="changeYear(1)"
      />
    </div>
    <div class="picker-grid">
      <button
        v-for="(name, index) in monthNames"
        :key="index"
        type="button"
        class="picker-cell"
        @click="onCellClick(index + 1)"
      >
        <span class="picker-cell-inner" :class="{ 'cell-selected': isSelected(index + 1) }">{{ name }}</span>
      </button>
    </div>
  </div>
</template>

<style lang="scss" scoped>
.month-picker {
  width: 15rem;
}

.picker-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.picker-title {
  text-align: center;
}

/* 网格高度与 DateCalendar 的星期行加日期网格区域同高（2rem + 6*2rem + 5*0.125rem），
   使月份选择面板与月历面板整体等高，保持视觉一致性；3 行网格自动拉伸填满。 */
.picker-grid {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  grid-template-rows: repeat(3, 1fr);
  gap: 0.125rem;
  height: 14.625rem;
}

.picker-cell {
  display: flex;
  align-items: center;
  justify-content: center;
  border: none;
  background: transparent;
  padding: 0;
  cursor: pointer;
  font: inherit;
  border-radius: 1rem;
  text-align: center;
}

/* 高亮块按内容定宽、固定 2rem 高，保证宽大于等于高（单元格本身因网格拉伸为高大于宽）。 */
.picker-cell-inner {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 2rem;
  min-width: 2rem;
  padding: 0 0.75rem;
  border-radius: 1rem;
}

.cell-selected {
  background: rgb(var(--v-theme-primary));
  color: #fff;
  font-weight: 700;
}
</style>
