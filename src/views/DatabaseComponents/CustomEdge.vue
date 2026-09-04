<!--
   自定义边组件。

   在画布中渲染带标题的贝塞尔曲线边。
   标题始终显示在边的中点位置，鼠标悬浮时显示详情 tooltip。
   当标题和详情均为空时不渲染任何标签。
   使用 De Casteljau 算法在 t=0.5 处拆分曲线，实现标题打断边的效果；
   缺口大小取标签矩形沿曲线中点切向的弦长（水平边打断宽度、垂直边只打断高度）。
   箭头为组件自绘（vue-flow 的 markerEnd 在 EdgeProps 中已序列化为 SVG url，无法在组件层改色，
   故忽略该 prop 并自绘 polygon 箭头，使箭头颜色随选中/高亮状态与边线同步变化）。
   选中（selected）时边线与箭头为实色 primary 并加粗；被邻居高亮（选中节点的相连边）时为半透明 primary 并加粗。
  -->
<script setup lang="ts">
import { EdgeLabelRenderer, useVueFlow, type EdgeProps } from "@vue-flow/core";
import { ref, computed, onMounted, watch, nextTick } from "vue";
import { highlightedEdgeIds } from "@/composables/use-neighbor-highlight";
import {
  arrowDirection,
  computeControlPoints,
  midpointSpeed,
  midpointTangent,
  rectChordHalfLength,
  splitBezierAtT,
  toPathString,
} from "@/utils/custom-edge-geometry";

const props = defineProps<EdgeProps>();

const emit = defineEmits<{ contextmenu: [payload: { id: string; x: number; y: number }] }>();

/** 标题标签 DOM 引用 */
const labelRef = ref<HTMLElement | null>(null);

/** 标签实际宽度（flow 坐标单位，默认 60 避免首帧闪烁） */
const labelWidth = ref(60);

/** 标签实际高度（flow 坐标单位，默认 20 避免首帧闪烁） */
const labelHeight = ref(20);

/** 画布视口：用于把 getBoundingClientRect 测得的屏幕像素换算为 flow 坐标单位 */
const { viewport } = useVueFlow();

/**
 * 贝塞尔曲线控制点：端点为源/目标连接桩坐标，控制点沿各自 handle 轴向偏移 |dx|/2。
 * 垂直边（dx=0）时控制点与端点重合，曲线退化为直线。
 */
const bezPoints = computed(() =>
  computeControlPoints(
    props.sourceX ?? 0,
    props.sourceY ?? 0,
    props.targetX ?? 0,
    props.targetY ?? 0,
    props.sourcePosition ?? "right",
    props.targetPosition ?? "left",
  ),
);

/**
 * 拆分参数 t1 和 t2，在曲线中点创建与标签尺寸匹配的缺口。
 * 缺口沿曲线的半长取标签矩形沿中点切向的弦半长加 padding 的一半：
 * 水平边覆盖标签宽度、垂直边只覆盖标签高度、对角边取两者的过渡；
 * 再用中点参数速度换算为参数间隔（缺口弧长 ≈ 2δ·speed），保证换算与边的朝向无关。
 * speed 为 0（两端点重合的病理场景）时按 0.08 兜底。
 */
const splitParams = computed(() => {
  if (!props.data?.title && !props.data?.description) {
    return { t1: 0.5, t2: 0.5 };
  }
  const padding = 12;
  const speed = midpointSpeed(bezPoints.value);
  const halfChord = rectChordHalfLength(
    labelWidth.value / 2,
    labelHeight.value / 2,
    midpointTangent(bezPoints.value),
  );
  const rawDelta = speed > 0 ? (halfChord + padding / 2) / speed : 0.08;
  const delta = Math.min(Math.max(rawDelta, 0.02), 0.4);
  return {
    t1: 0.5 - delta,
    t2: 0.5 + delta,
  };
});

/** 前半段路径：从 t=0 到 t=0.5-δ */
const path1 = computed(() => {
  const { t1 } = splitParams.value;
  const { first } = splitBezierAtT(bezPoints.value, t1);
  return toPathString(first[0], first[1], first[2], first[3]);
});

/** 后半段路径：从 t=0.5+δ 到 t=1 */
const path2 = computed(() => {
  const { t2 } = splitParams.value;
  const { second } = splitBezierAtT(bezPoints.value, t2);
  return toPathString(second[0], second[1], second[2], second[3]);
});

/** 完整的边路径（无缺口），用作透明命中区域 */
const hitAreaPath = computed(() => {
  const { p0, cp1, cp2, p1 } = bezPoints.value;
  return toPathString(p0, cp1, cp2, p1);
});

/** 中点坐标（标签位置） */
const midPoint = computed(() => {
  const { point } = splitBezierAtT(bezPoints.value, 0.5);
  return point;
});

/** 是否被直接选中（EdgeProps.selected 为可选，undefined 视为未选中） */
const isSelected = computed(() => props.selected === true);

/** 是否被邻居高亮（本边与某个选中节点相连，由模块级邻居高亮状态驱动） */
const isNeighborHighlighted = computed(() => highlightedEdgeIds.value.has(props.id));

/** 边线宽度（SVG 用户单位，随画布缩放）：选中或高亮时加粗 */
const strokeWidth = computed(() => (isSelected.value || isNeighborHighlighted.value ? 3 : 2));

/**
 * g 元素的颜色样式：currentColor 由此派生；选中实色 primary，高亮半透明 primary。
 * 默认显式沿用 vue-flow 主题灰 #b1b1b7（原外观：线段由 style.css 锁定该色，箭头由 defaultMarkerColor 默认该色），
 * 不继承文本色，保证非选中非高亮时外观与改动前一致。
 */
const edgeColorStyle = computed(() => {
  if (isSelected.value) return { color: "rgb(var(--v-theme-primary))" };
  if (isNeighborHighlighted.value) return { color: "rgba(var(--v-theme-primary), 0.55)" };
  return { color: "#b1b1b7" };
});

/**
 * 边线内联样式：vue-flow 的 style.css 用 CSS 规则锁定了 .vue-flow__edge-path 的 stroke/stroke-width
 * （含选中态 #555），presentation attribute 优先级不足无法生效，故改用内联样式（特异性最高）驱动。
 * 颜色沿用 currentColor 机制，由 g 元素的 color 样式派生，与自绘箭头保持一致。
 */
const edgePathStyle = computed(() => ({
  stroke: "currentColor",
  strokeWidth: strokeWidth.value,
}));

/**
 * 自绘箭头路径：实心三角形，尖端在 path2 终点，朝向为曲线末端切向
 * （切向退化为零向量时按目标 handle 轴向兜底，见 custom-edge-geometry.arrowDirection）。
 * 箭头尺寸为 SVG 用户单位（随画布缩放），颜色由 fill="currentColor" 继承 g 元素的 color 样式。
 */
const arrowPath = computed(() => {
  const { p1 } = bezPoints.value;
  const dir = arrowDirection(bezPoints.value, props.targetPosition ?? "left");
  const ux = dir.x;
  const uy = dir.y;
  // 箭头长度与半宽（SVG 用户单位，视觉尺寸对齐 vue-flow 的 ArrowClosed marker）
  const size = 10;
  const halfWidth = 6;
  // 垂直于箭头朝向的单位向量（用于展开两翼）
  const px = -uy;
  const py = ux;
  const baseX = p1.x - ux * size;
  const baseY = p1.y - uy * size;
  const leftX = baseX + px * halfWidth;
  const leftY = baseY + py * halfWidth;
  const rightX = baseX - px * halfWidth;
  const rightY = baseY - py * halfWidth;
  return `M ${p1.x},${p1.y} L ${leftX},${leftY} L ${rightX},${rightY} Z`;
});

/**
 * 测量标题标签的实际尺寸。
 * getBoundingClientRect 测得的是缩放后的屏幕像素（label 容器位于画布的 scale(zoom) 变换内），
 * 除以当前 zoom 换算为 flow 坐标单位，使缺口计算在任意缩放级别下都准确；
 * 换算结果与 zoom 无关，因此 zoom 变化后无需重新测量。
 */
function measureLabel(): void {
  if (labelRef.value) {
    const rect = labelRef.value.getBoundingClientRect();
    labelWidth.value = rect.width / viewport.value.zoom;
    labelHeight.value = rect.height / viewport.value.zoom;
  }
}

onMounted(() => {
  nextTick(measureLabel);
});

/** 标签内容或节点位置变化时重新测量并更新缺口 */
watch(
  () => [
    props.data?.title,
    props.data?.description,
    props.sourceX,
    props.sourceY,
    props.targetX,
    props.targetY,
  ],
  () => nextTick(measureLabel),
);
</script>

<script lang="ts">
export default {
  inheritAttrs: false,
};
</script>

<template>
  <g :style="edgeColorStyle">
    <path
      :d="hitAreaPath"
      fill="none"
      stroke="transparent"
      stroke-width="20"
      class="vue-flow__edge-hit-area"
      @contextmenu.prevent="emit('contextmenu', { id, x: $event.clientX, y: $event.clientY })"
    />
    <path
      :d="path1"
      fill="none"
      :style="edgePathStyle"
      class="vue-flow__edge-path"
    />
    <path
      :d="path2"
      fill="none"
      :style="edgePathStyle"
      class="vue-flow__edge-path"
    />
    <path :d="arrowPath" fill="currentColor" stroke="none" />
  </g>

  <EdgeLabelRenderer>
    <div
      ref="labelRef"
      v-if="data.title || data.description"
      :style="{
        pointerEvents: 'all',
        position: 'absolute',
        transform: `translate(-50%, -50%) translate(${midPoint.x}px,${midPoint.y}px)`,
      }"
      class="custom-edge-label nodrag nopan"
      @contextmenu.prevent="emit('contextmenu', { id, x: $event.clientX, y: $event.clientY })"
    >
      <VTooltip location="top" :disabled="!data.description">
        <template #activator="{ props: tooltipProps }">
          <div class="custom-edge-label-inner" v-bind="tooltipProps">
            <span v-if="data.title" class="custom-edge-label-text">{{ data.title }}</span>
            <span v-else class="custom-edge-label-dot"></span>
          </div>
        </template>
        <div class="custom-edge-tooltip-description">{{ data.description }}</div>
      </VTooltip>
    </div>
  </EdgeLabelRenderer>
</template>

<style lang="scss" scoped>
.custom-edge-label {
  pointer-events: all;
}

.custom-edge-label-inner {
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 0.375rem;
  padding: 0.25rem 0.5rem;
  cursor: default;
}

.custom-edge-label-text {
  font-size: 0.75rem;
  font-weight: 500;
  color: rgb(var(--v-theme-on-surface));
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: 12.5rem;
}

.custom-edge-label-dot {
  display: inline-block;
  width: 0.5rem;
  height: 0.5rem;
  border-radius: 50%;
  background-color: rgb(var(--v-theme-on-surface), 0.4);
}

.custom-edge-tooltip-description {
  white-space: pre-wrap;
  max-width: 18.75rem;
}
</style>
