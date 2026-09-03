<!--
  年份选择面板（instant 悬浮选择面板的内部组件，用于"年"精度）。

  以类似日历的网格方式选择年份：每页 3 行乘 4 列共 12 个年份，头部左右箭头按页翻动（每页 12 年），
  头部标题显示当前页的年份范围；年份范围限定 0-9999，到达边界时对应箭头禁用。
  空值态（hasValue 为假）下视图初始化到当前本地年份所在页且无选中高亮。
-->
<script setup lang="ts">
import { computed, ref } from "vue";

/** 每页显示的年份数量（3 行乘 4 列）。 */
const PAGE_SIZE = 12;
/** 年份下界。 */
const MIN_YEAR = 0;
/** 年份上界。 */
const MAX_YEAR = 9999;

const props = defineProps<{
  /** 当前选中的年份。 */
  year: number;
  /** 字段值是否非空（全 0 部件为"字段值为空"的项目约定）；空值态下不高亮选中项。 */
  hasValue?: boolean;
}>();

const emit = defineEmits<{
  select: [year: number];
}>();

/**
 * 求年份所在页的起始年份。
 * @param year 年份
 * @returns 该年份所在页的起始年份
 */
function pageStartOf(year: number): number {
  return Math.floor(year / PAGE_SIZE) * PAGE_SIZE;
}

/** 视图当前页的起始年份：有合法值时取选中年份所在页，否则取当前本地年份所在页。 */
const pageStart = ref(
  pageStartOf(
    props.hasValue && props.year >= MIN_YEAR && props.year <= MAX_YEAR
      ? props.year
      : new Date().getFullYear(),
  ),
);

/** 当前页的年份列表，不足一页（仅最后一页 9996-9999）时以 null 补位保持网格形状。 */
const pageYears = computed<(number | null)[]>(() =>
  Array.from({ length: PAGE_SIZE }, (_, i) => {
    const year = pageStart.value + i;
    return year <= MAX_YEAR ? year : null;
  }),
);

/** 当前页标题（起始年份-结束年份）。 */
const pageTitle = computed(
  () => `${pageStart.value}-${Math.min(pageStart.value + PAGE_SIZE - 1, MAX_YEAR)}`,
);

/** 上一页箭头是否禁用（当前页已到年份下界）。 */
const prevDisabled = computed(() => pageStart.value <= MIN_YEAR);
/** 下一页箭头是否禁用（翻页后超出年份上界）。 */
const nextDisabled = computed(() => pageStart.value + PAGE_SIZE > MAX_YEAR);

/**
 * 翻页：按页增减年份范围；越出 0-9999 边界时不翻页。
 * @param delta 页数增减量（正数向后、负数向前）
 */
function changePage(delta: number): void {
  const next = pageStart.value + delta * PAGE_SIZE;
  if (next < MIN_YEAR || next > MAX_YEAR) return;
  pageStart.value = next;
}

/**
 * 判断年份格子是否为当前选中的年份（空值态下无选中）。
 * @param year 格子对应的年份
 * @returns 与选中年份相等且非空值态时返回 true
 */
function isSelected(year: number): boolean {
  return (props.hasValue ?? false) && year === props.year;
}

/**
 * 处理年份格子点击：emit 选中的年份。
 * @param year 被点击的年份
 */
function onCellClick(year: number): void {
  emit("select", year);
}
</script>

<template>
  <div class="year-picker">
    <div class="picker-header">
      <VBtn
        icon="mdi-chevron-left"
        variant="text"
        density="compact"
        :disabled="prevDisabled"
        @click="changePage(-1)"
      />
      <div class="picker-title">{{ pageTitle }}</div>
      <VBtn
        icon="mdi-chevron-right"
        variant="text"
        density="compact"
        :disabled="nextDisabled"
        @click="changePage(1)"
      />
    </div>
    <div class="picker-grid">
      <template v-for="(year, index) in pageYears" :key="index">
        <button
          v-if="year !== null"
          type="button"
          class="picker-cell"
          @click="onCellClick(year)"
        >
          <span class="picker-cell-inner" :class="{ 'cell-selected': isSelected(year) }">{{ year }}</span>
        </button>
        <div v-else class="picker-cell picker-cell-empty"></div>
      </template>
    </div>
  </div>
</template>

<style lang="scss" scoped>
.year-picker {
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
   使年份选择面板与月历面板整体等高，保持视觉一致性；3 行网格自动拉伸填满。 */
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

.picker-cell-empty {
  cursor: default;
}

.cell-selected {
  background: rgb(var(--v-theme-primary));
  color: #fff;
  font-weight: 700;
}
</style>
