<!--
  单个时间位的数字滚轮选择列。

  以固定行高 2rem 的纵向滚动列表承载 min-max 范围内的连续数字，高度占满父容器（最小 10rem）；
  列表顶部与底部各保留 (容器高-行高)/2 的空白，使任意一项都能滚动至容器垂直正中；
  容器正中有一条不可交互的选中指示框，滚动停止后自动吸附到距指示框最近的数字；滚动条隐藏不显示。
  数值范围较大时（如年份 0-9999），仅渲染滚动位置前后各十项的窗口，避免一次性渲染全部条目。
  本组件不渲染标题，标题由父组件负责。
-->
<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";

/** 单项行高（像素），与 CSS 中 2rem 行高对应。 */
const ITEM_HEIGHT_PX = 32;
/** 窗口化渲染时在可视区上下各保留的缓冲项数。 */
const BUFFER_ITEM_COUNT = 10;
/** 滚动停止后吸附的防抖延迟（毫秒）。 */
const SNAP_DEBOUNCE_MS = 100;

const props = withDefaults(defineProps<{
  modelValue: number;
  min: number;
  max: number;
  /** 显示用零填充位数；0 表示不填充。 */
  digits?: number;
}>(), {
  digits: 0,
});

const emit = defineEmits<{
  "update:modelValue": [value: number];
}>();

/** 滚动容器的 DOM 引用。 */
const el = ref<HTMLElement | null>(null);
/** 最近一次滚动位置（经 requestAnimationFrame 节流更新）。 */
const scrollTop = ref(0);
/** 滚动容器的当前高度（像素），由 ResizeObserver 同步；初始 160 对应最小高度 10rem。 */
const containerHeight = ref(160);

/** 条目总数。 */
const count = computed(() => Math.max(0, props.max - props.min + 1));

/** 可视项数（容器高度除以行高，向上取整）。 */
const visibleItemCount = computed(() => Math.ceil(containerHeight.value / ITEM_HEIGHT_PX));

/** 顶部与底部各保留的空白高度：恰好 (容器高-行高)/2，使任意条目都能居中且 scrollTop = 条目序号乘行高 的对应关系与容器高度无关。 */
const edgeSpacerHeight = computed(() => Math.max(0, (containerHeight.value - ITEM_HEIGHT_PX) / 2));

/** 当前渲染窗口的条目序号范围 [start, end]，裁剪到 [0, count-1]。 */
const renderWindow = computed(() => {
  const center = Math.floor(scrollTop.value / ITEM_HEIGHT_PX);
  const start = Math.max(0, center - BUFFER_ITEM_COUNT);
  const end = Math.min(count.value - 1, center + visibleItemCount.value + BUFFER_ITEM_COUNT);
  return { start, end };
});

/** 渲染窗口内的条目序号列表。 */
const visibleIndices = computed(() => {
  const { start, end } = renderWindow.value;
  const indices: number[] = [];
  for (let i = start; i <= end; i += 1) indices.push(i);
  return indices;
});

/** 窗口上方的空白高度：顶部固定空白加窗口首条目之前的条目高度。 */
const topSpacerHeight = computed(
  () => renderWindow.value.start * ITEM_HEIGHT_PX + edgeSpacerHeight.value,
);

/** 窗口下方的空白高度：窗口尾条目之后的条目高度加底部固定空白。 */
const bottomSpacerHeight = computed(
  () => (count.value - 1 - renderWindow.value.end) * ITEM_HEIGHT_PX + edgeSpacerHeight.value,
);

/** 正在程序化平滑滚动（点击条目触发）的目标条目序号；滚动停止后清除，用于避免打断平滑动画。 */
let smoothTargetIndex: number | null = null;
/** requestAnimationFrame 节流标记。 */
let rafPending = false;
/** 吸附防抖定时器句柄。 */
let snapTimer: ReturnType<typeof setTimeout> | undefined;

/**
 * 将条目序号裁剪到合法范围。
 * @param index 条目序号
 * @returns 裁剪后的序号；条目总数为 0 时返回 0
 */
function clampIndex(index: number): number {
  if (count.value <= 0) return 0;
  return Math.max(0, Math.min(count.value - 1, index));
}

/**
 * 处理容器滚动事件：以 requestAnimationFrame 节流更新 scrollTop，并重新调度吸附防抖。
 */
function onScroll(): void {
  if (!rafPending) {
    rafPending = true;
    requestAnimationFrame(() => {
      if (el.value !== null) scrollTop.value = el.value.scrollTop;
      rafPending = false;
    });
  }
  scheduleSnap();
}

/**
 * 调度滚动停止后的吸附：清除已有防抖定时器并重新计时。
 */
function scheduleSnap(): void {
  if (snapTimer !== undefined) clearTimeout(snapTimer);
  snapTimer = setTimeout(snapToNearest, SNAP_DEBOUNCE_MS);
}

/**
 * 在滚动停止后吸附到最近条目：对齐 scrollTop，若条目对应值不同于 modelValue 则 emit。
 */
function snapToNearest(): void {
  snapTimer = undefined;
  smoothTargetIndex = null;
  const e = el.value;
  if (e === null) return;
  const index = clampIndex(Math.round(e.scrollTop / ITEM_HEIGHT_PX));
  e.scrollTop = index * ITEM_HEIGHT_PX;
  const value = props.min + index;
  if (value !== props.modelValue) emit("update:modelValue", value);
}

/**
 * 处理条目点击：emit 对应值，并平滑滚动至该条目。
 * @param index 被点击的条目序号
 */
function selectIndex(index: number): void {
  const target = clampIndex(index);
  emit("update:modelValue", props.min + target);
  smoothTargetIndex = target;
  el.value?.scrollTo({ top: target * ITEM_HEIGHT_PX, behavior: "smooth" });
}

/**
 * 将容器滚动位置同步到 modelValue 对应的条目。
 * 使用直接赋值而非平滑滚动，避免初始化与外部同步时的抖动；
 * 若正处于点击触发的程序化平滑滚动途中则跳过，避免打断动画。
 */
function syncScrollFromModelValue(): void {
  const e = el.value;
  if (e === null) return;
  const target = clampIndex(props.modelValue - props.min);
  if (target === smoothTargetIndex) return;
  const current = clampIndex(Math.round(e.scrollTop / ITEM_HEIGHT_PX));
  if (current !== target) e.scrollTop = target * ITEM_HEIGHT_PX;
}

/**
 * 将条目序号显示为文本：按 digits 零填充。
 * @param index 条目序号
 * @returns 显示文本
 */
function displayTextOf(index: number): string {
  const value = props.min + index;
  return props.digits > 0 ? String(value).padStart(props.digits, "0") : String(value);
}

watch(() => props.modelValue, syncScrollFromModelValue);

/** 容器高度监听器：面板高度变化（如精度切换增减滚轮列）时同步 containerHeight。 */
let resizeObserver: ResizeObserver | undefined;

onMounted(() => {
  syncScrollFromModelValue();
  if (el.value !== null) {
    containerHeight.value = el.value.clientHeight;
    resizeObserver = new ResizeObserver(() => {
      if (el.value !== null) {
        containerHeight.value = el.value.clientHeight;
        // 尺寸变化后重新对齐滚动位置，确保 scrollTop = 条目序号乘行高 的不变量不被布局变化破坏。
        syncScrollFromModelValue();
      }
    });
    resizeObserver.observe(el.value);
  }
});

onBeforeUnmount(() => {
  resizeObserver?.disconnect();
});
</script>

<template>
  <div class="wheel-column">
    <div ref="el" class="wheel-scroll" @scroll="onScroll">
      <div class="wheel-spacer" :style="{ height: `${topSpacerHeight}px` }"></div>
      <div
        v-for="index in visibleIndices"
        :key="index"
        class="wheel-item"
        :class="{ 'wheel-item-selected': min + index === modelValue }"
        @click="selectIndex(index)"
      >
        {{ displayTextOf(index) }}
      </div>
      <div class="wheel-spacer" :style="{ height: `${bottomSpacerHeight}px` }"></div>
    </div>
    <!-- 选中指示框置于滚动容器之外：滚动容器内的绝对定位子元素会随内容一起滚动。 -->
    <div class="wheel-marker"></div>
  </div>
</template>

<style lang="scss" scoped>
.wheel-column {
  position: relative;
  flex: 1 1 auto;
  align-self: stretch;
  min-height: 10rem;
  /* 最小宽度保证多位数字（如三位毫秒）单行放得下，避免数字换行后视觉上与相邻条目重叠。 */
  min-width: 3rem;
}

/* 绝对定位充满父容器：滚动容器的高度只由 flex 布局决定，其内容高度不参与父级高度计算，
   避免 ResizeObserver 同步 containerHeight 与上下留白高度之间形成高度自反馈循环。
   overflow-anchor: none：禁用浏览器滚动锚定，否则留白高度变化时浏览器会自动偏移 scrollTop
   （且不触发 scroll 事件），破坏 scrollTop = 条目序号乘行高 的不变量并导致吸附到错误条目。 */
.wheel-scroll {
  position: absolute;
  inset: 0;
  overflow-y: auto;
  overscroll-behavior: contain;
  scrollbar-width: none;
  overflow-anchor: none;
}

.wheel-scroll::-webkit-scrollbar {
  display: none;
}

.wheel-item {
  height: 2rem;
  line-height: 2rem;
  text-align: center;
  cursor: pointer;
  user-select: none;
  /* 数字必须单行显示：换行会让第二行溢出到相邻条目的位置，看起来像数字重叠。 */
  white-space: nowrap;
}

.wheel-item-selected {
  font-weight: 700;
  color: rgb(var(--v-theme-primary));
}

.wheel-marker {
  position: absolute;
  top: 50%;
  left: 0;
  right: 0;
  height: 2rem;
  transform: translateY(-50%);
  pointer-events: none;
  border-top: 1px solid rgb(var(--v-theme-primary));
  border-bottom: 1px solid rgb(var(--v-theme-primary));
}
</style>