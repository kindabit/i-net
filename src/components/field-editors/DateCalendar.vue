<!--
  月历选择面板。

  以 6 行乘 7 列的网格展示指定年月，用于选择年、月、日。
  日历数学（闰年判断、月天数、星期序号）全部由 @/field-types/date-time 的纯函数完成，
  仅文案本地化（月份名、星期名）使用 Intl；年份支持 0-9999。
  允许传入非法日期（如全部为 0 的空值态），此时仅作为无选中的视图，
  仍可点击格子选择日期并向外 emit。
-->
<script setup lang="ts">
import { computed, ref } from "vue";
import { t, currentLocale } from "@/i18n";
import { daysInMonth, weekdayOf } from "@/field-types/date-time";

const props = defineProps<{
  year: number;
  month: number;
  day: number;
}>();

const emit = defineEmits<{
  "select": [date: { year: number; month: number; day: number }];
}>();

/** 日历网格单格数据。 */
interface CalendarCell {
  year: number;
  month: number;
  day: number;
  /** 是否属于上个月或下个月。 */
  isOtherMonth: boolean;
  /** 用于列表渲染的稳定键。 */
  key: string;
}

/**
 * 判断年月日是否为合法日期（年份 0-9999 且日不超出当月天数）。
 * @param year 年份
 * @param month 月份
 * @param day 日
 * @returns 合法时返回 true
 */
function isValidDate(year: number, month: number, day: number): boolean {
  if (year < 0 || year > 9999) return false;
  if (month < 1 || month > 12) return false;
  return day >= 1 && day <= daysInMonth(year, month);
}

/**
 * 求指定年月的前一个月。
 * @param year 年份
 * @param month 月份
 * @returns 前一个月的年月
 */
function prevMonth(year: number, month: number): { year: number; month: number } {
  if (month === 1) return { year: year - 1, month: 12 };
  return { year, month: month - 1 };
}

/**
 * 求指定年月的后一个月。
 * @param year 年份
 * @param month 月份
 * @returns 后一个月的年月
 */
function nextMonth(year: number, month: number): { year: number; month: number } {
  if (month === 12) return { year: year + 1, month: 1 };
  return { year, month: month + 1 };
}

/** 视图初始年月：props 合法时取 props 的年月，否则取当前本地时间的年月（仅作视图初始化，不参与日历数学）。 */
const initialView = isValidDate(props.year, props.month, props.day)
  ? { year: props.year, month: props.month }
  : (() => {
      const now = new Date();
      return { year: now.getFullYear(), month: now.getMonth() + 1 };
    })();

/** 视图显示的年月。 */
const viewYear = ref<number>(initialView.year);
const viewMonth = ref<number>(initialView.month);

/** 当前选中的合法日期；props 非法（如全 0 空值态）时为 null，表示无选中。 */
const selectedDate = computed<{ year: number; month: number; day: number } | null>(() =>
  isValidDate(props.year, props.month, props.day)
    ? { year: props.year, month: props.month, day: props.day }
    : null,
);

/** 每周首日（1=周一 … 7=周日）；取不到本地化信息时回退 1。 */
const firstDay = computed(() => {
  const locale = new Intl.Locale(currentLocale.value);
  const info = (locale as unknown as { getWeekInfo?: () => { firstDay?: number } }).getWeekInfo?.();
  const fd = info?.firstDay;
  return fd !== undefined && fd >= 1 && fd <= 7 ? fd : 1;
});

/** 本地化月份名格式化器（参考日期固定为 2000 年，安全）。 */
const monthNameFormatter = computed(
  () => new Intl.DateTimeFormat(currentLocale.value, { month: "long" }),
);

/** 头部标题：年份加本地化月份（zh 用数字月份避免与"月"字重复，en 用本地化月份名）。 */
const viewTitle = computed(() =>
  t("database.field-editor.calendar-title", {
    year: viewYear.value,
    month: monthNameFormatter.value.format(new Date(2000, viewMonth.value - 1, 1)),
    monthNumber: viewMonth.value,
  }),
);

/** 本地化星期名格式化器（参考日期固定为 2000 年 10 月，该月 1 日为周日，安全）。 */
const weekdayNameFormatter = computed(
  () => new Intl.DateTimeFormat(currentLocale.value, { weekday: "narrow" }),
);

/** 星期表头名称：按每周首日旋转后的 7 个星期名。 */
const weekdayNames = computed(() => {
  const base = Array.from({ length: 7 }, (_, i) =>
    weekdayNameFormatter.value.format(new Date(2000, 9, 1 + i)),
  );
  const offset = firstDay.value % 7;
  return Array.from({ length: 7 }, (_, i) => base[(offset + i) % 7]!);
});

/** 网格首格之前属于上个月的天数（由本月 1 日的星期序号与每周首日推算）。 */
const leadingGap = computed(() => {
  const firstWeekday = weekdayOf(viewYear.value, viewMonth.value, 1);
  return (firstWeekday - firstDay.value + 7) % 7;
});

/** 日历网格的 42 个格子（6 行乘 7 列），含上个月与下个月的补位格子。 */
const gridCells = computed<CalendarCell[]>(() => {
  const days = daysInMonth(viewYear.value, viewMonth.value);
  const prev = prevMonth(viewYear.value, viewMonth.value);
  const next = nextMonth(viewYear.value, viewMonth.value);
  const prevDays = daysInMonth(prev.year, prev.month);
  const cells: CalendarCell[] = [];
  for (let k = 0; k < 42; k += 1) {
    const dayIndex = k - leadingGap.value + 1;
    let year = viewYear.value;
    let month = viewMonth.value;
    let day = dayIndex;
    let isOtherMonth = false;
    if (dayIndex < 1) {
      year = prev.year;
      month = prev.month;
      day = prevDays + dayIndex;
      isOtherMonth = true;
    } else if (dayIndex > days) {
      year = next.year;
      month = next.month;
      day = dayIndex - days;
      isOtherMonth = true;
    }
    cells.push({
      year,
      month,
      day,
      isOtherMonth,
      key: `${viewYear.value}-${viewMonth.value}-${k}`,
    });
  }
  return cells;
});

/** 上一个月箭头是否禁用（视图已到最小边界 0 年 1 月）。 */
const prevDisabled = computed(() => viewYear.value === 0 && viewMonth.value === 1);
/** 下一个月箭头是否禁用（视图已到最大边界 9999 年 12 月）。 */
const nextDisabled = computed(() => viewYear.value === 9999 && viewMonth.value === 12);

/**
 * 切换视图月份，跨年时进位或借位年份；越出 0-9999 边界时不切换。
 * @param delta 月份增减量（正数向后、负数向前）
 */
function changeMonth(delta: number): void {
  let year = viewYear.value;
  let month = viewMonth.value + delta;
  if (month < 1) {
    month = 12;
    year -= 1;
  } else if (month > 12) {
    month = 1;
    year += 1;
  }
  if (year < 0 || year > 9999) return;
  viewYear.value = year;
  viewMonth.value = month;
}

/**
 * 判断格子是否为当前选中的日期。
 * @param cell 格子数据
 * @returns 与选中日期完全相等时返回 true
 */
function isSelected(cell: CalendarCell): boolean {
  return (
    selectedDate.value !== null &&
    cell.year === selectedDate.value.year &&
    cell.month === selectedDate.value.month &&
    cell.day === selectedDate.value.day
  );
}

/**
 * 处理格子点击：emit 该格实际的年月日；若属于上个月或下个月，则把视图切换到该格所在月（越出 0-9999 边界时不切换视图）。
 * @param cell 被点击的格子数据
 */
function onCellClick(cell: CalendarCell): void {
  emit("select", { year: cell.year, month: cell.month, day: cell.day });
  if (cell.isOtherMonth && cell.year >= 0 && cell.year <= 9999) {
    viewYear.value = cell.year;
    viewMonth.value = cell.month;
  }
}
</script>

<template>
  <div class="date-calendar">
    <div class="calendar-header">
      <VBtn
        icon="mdi-chevron-left"
        variant="text"
        density="compact"
        :disabled="prevDisabled"
        @click="changeMonth(-1)"
      />
      <div class="calendar-title">{{ viewTitle }}</div>
      <VBtn
        icon="mdi-chevron-right"
        variant="text"
        density="compact"
        :disabled="nextDisabled"
        @click="changeMonth(1)"
      />
    </div>
    <div class="calendar-weekdays">
      <div v-for="(name, index) in weekdayNames" :key="index" class="calendar-weekday">
        {{ name }}
      </div>
    </div>
    <div class="calendar-grid">
      <button
        v-for="cell in gridCells"
        :key="cell.key"
        type="button"
        class="calendar-cell"
        :class="{ 'cell-other-month': cell.isOtherMonth, 'cell-selected': isSelected(cell) }"
        @click="onCellClick(cell)"
      >
        {{ cell.day }}
      </button>
    </div>
  </div>
</template>

<style lang="scss" scoped>
.date-calendar {
  width: 15rem;
}

.calendar-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.calendar-title {
  text-align: center;
}

.calendar-weekdays {
  display: grid;
  grid-template-columns: repeat(7, 2rem);
  gap: 0.125rem;
}

.calendar-weekday {
  width: 2rem;
  height: 2rem;
  line-height: 2rem;
  text-align: center;
  color: rgb(var(--v-theme-secondary));
}

.calendar-grid {
  display: grid;
  grid-template-columns: repeat(7, 2rem);
  gap: 0.125rem;
}

.calendar-cell {
  width: 2rem;
  height: 2rem;
  line-height: 2rem;
  text-align: center;
  border: none;
  background: transparent;
  padding: 0;
  cursor: pointer;
  font: inherit;
}

.cell-other-month {
  color: rgba(var(--v-theme-on-surface), 0.38);
}

.cell-selected {
  background: rgb(var(--v-theme-primary));
  color: #fff;
  font-weight: 700;
  border-radius: 50%;
}
</style>