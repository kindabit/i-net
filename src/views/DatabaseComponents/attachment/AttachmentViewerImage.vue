<!--
  附件图片查看器。

  将附件明文字节包装为内存 Blob URL 并以图片形式渲染，全程不落盘；组件卸载时回收 Blob URL。
  提供"适应窗口"与"原始大小"两种显示模式；原始大小模式下可通过滚轮、滑块调整缩放倍率，
  并通过鼠标拖拽平移图片。
-->
<script setup lang="ts">
import { computed, nextTick, onUnmounted, ref } from "vue";
import { t } from "@/i18n";
import { mimeOf } from "./attachment-types";

const props = defineProps<{
  /** 附件明文内容 */
  bytes: Uint8Array;
  /** 附件文件名（用于推断 MIME 类型） */
  fileName: string;
}>();

/** 缩放倍率下限 */
const SCALE_MIN = 0.1;
/** 缩放倍率上限 */
const SCALE_MAX = 8;
/** 滚轮每次缩放的乘性因子 */
const WHEEL_FACTOR = 1.15;

/** 显示模式：fit=适应窗口，actual=原始大小（可调倍率） */
const mode = ref<"fit" | "actual">("fit");
/** 原始大小模式下的缩放倍率（1 表示原始像素尺寸） */
const scale = ref(1);
/** 图片原始像素宽度（加载完成后写入，写入前图片按样式默认尺寸显示） */
const naturalWidth = ref(0);
/** 滚动容器 */
const containerRef = ref<HTMLDivElement>();
/** 拖拽平移进行中 */
const dragging = ref(false);

/** 附件字节对应的内存 Blob URL */
const blobUrl = createBlobUrl(props.bytes, props.fileName);

/** 图片元素的内联样式：原始大小模式下按倍率设置显示宽度，其余情况交给样式类 */
const imgStyle = computed(() => {
  if (mode.value === "fit" || naturalWidth.value === 0) return {};
  return { width: `${naturalWidth.value * scale.value}px` };
});

/**
 * 将附件字节包装为内存 Blob URL。
 * @param bytes 附件明文内容
 * @param fileName 附件文件名
 * @returns Blob URL
 */
function createBlobUrl(bytes: Uint8Array, fileName: string): string {
  const mime = mimeOf(fileName);
  const blob = new Blob([bytes], { type: mime ?? undefined });
  return URL.createObjectURL(blob);
}

/**
 * 图片加载完成后记录原始像素宽度，供原始大小模式计算显示宽度。
 * @param event 图片 load 事件
 */
function onImageLoad(event: Event): void {
  naturalWidth.value = (event.target as HTMLImageElement).naturalWidth;
}

/**
 * 以容器中心为锚点设置缩放倍率（滚轮与滑块共用）：先记录中心点对应的内容坐标，
 * 倍率生效后恢复滚动位置，使中心内容在缩放前后保持不动。
 * @param value 目标倍率，会被裁剪到允许范围
 */
function setScale(value: number): void {
  const clamped = Math.min(SCALE_MAX, Math.max(SCALE_MIN, value));
  const old = scale.value;
  if (clamped === old) return;
  const container = containerRef.value;
  scale.value = clamped;
  if (!container) return;
  const ratio = clamped / old;
  const centerX = container.scrollLeft + container.clientWidth / 2;
  const centerY = container.scrollTop + container.clientHeight / 2;
  void nextTick(() => {
    container.scrollLeft = centerX * ratio - container.clientWidth / 2;
    container.scrollTop = centerY * ratio - container.clientHeight / 2;
  });
}

/**
 * 滚轮缩放（仅原始大小模式）：向上放大、向下缩小。
 * @param event 滚轮事件
 */
function onWheel(event: WheelEvent): void {
  if (mode.value !== "actual") return;
  event.preventDefault();
  setScale(event.deltaY < 0 ? scale.value * WHEEL_FACTOR : scale.value / WHEEL_FACTOR);
}

/** 拖拽起点（鼠标坐标与容器滚动位置） */
let dragStartX = 0;
let dragStartY = 0;
let dragStartScrollLeft = 0;
let dragStartScrollTop = 0;

/**
 * 开始拖拽平移（仅原始大小模式），在 window 上跟踪移动与结束，保证移出容器也能正常拖拽。
 * @param event 鼠标按下事件
 */
function onDragStart(event: MouseEvent): void {
  const container = containerRef.value;
  if (mode.value !== "actual" || !container || event.button !== 0) return;
  event.preventDefault();
  dragging.value = true;
  dragStartX = event.clientX;
  dragStartY = event.clientY;
  dragStartScrollLeft = container.scrollLeft;
  dragStartScrollTop = container.scrollTop;
  window.addEventListener("mousemove", onDragMove);
  window.addEventListener("mouseup", onDragEnd);
}

/**
 * 拖拽中：按鼠标位移反向滚动容器，形成拖动图片的手感。
 * @param event 鼠标移动事件
 */
function onDragMove(event: MouseEvent): void {
  const container = containerRef.value;
  if (!container) return;
  container.scrollLeft = dragStartScrollLeft - (event.clientX - dragStartX);
  container.scrollTop = dragStartScrollTop - (event.clientY - dragStartY);
}

/** 结束拖拽平移并移除 window 监听。无输入参数，无返回值。 */
function onDragEnd(): void {
  dragging.value = false;
  window.removeEventListener("mousemove", onDragMove);
  window.removeEventListener("mouseup", onDragEnd);
}

onUnmounted(() => {
  // 组件卸载时若拖拽未完成，移除 window 监听避免泄漏
  window.removeEventListener("mousemove", onDragMove);
  window.removeEventListener("mouseup", onDragEnd);
  URL.revokeObjectURL(blobUrl);
});
</script>

<template>
  <div class="viewer-image">
    <div class="viewer-image-toolbar">
      <VBtnToggle
        v-model="mode"
        mandatory
        divided
        density="compact"
        variant="outlined"
        color="primary"
      >
        <VBtn value="fit" size="small">{{ t("database.canvas.attachment.view-fit") }}</VBtn>
        <VBtn value="actual" size="small">{{ t("database.canvas.attachment.view-actual") }}</VBtn>
      </VBtnToggle>
      <template v-if="mode === 'actual'">
        <VSlider
          :model-value="scale"
          :min="SCALE_MIN"
          :max="SCALE_MAX"
          :step="0.05"
          hide-details
          class="viewer-image-slider"
          @update:model-value="setScale"
        />
        <span class="viewer-image-scale">{{ Math.round(scale * 100) }}%</span>
      </template>
    </div>
    <div
      ref="containerRef"
      class="viewer-image-wrap"
      :class="{ 'is-actual': mode === 'actual', dragging }"
      @wheel="onWheel"
      @mousedown="onDragStart"
    >
      <img
        :src="blobUrl"
        :alt="fileName"
        :style="imgStyle"
        draggable="false"
        @load="onImageLoad"
      />
    </div>
  </div>
</template>

<style lang="scss" scoped>
.viewer-image {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  height: 100%;
}

.viewer-image-toolbar {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 0.75rem;
}

.viewer-image-slider {
  max-width: 16rem;
}

.viewer-image-scale {
  min-width: 3rem;
  text-align: right;
  font-size: 0.875rem;
}

.viewer-image-wrap {
  display: flex;
  overflow: auto;
  flex: 1;
  min-height: 0;

  img {
    // 阻止 flex 布局把超出容器的图片压缩回容器宽度（放大倍率依赖内联宽度生效）
    flex-shrink: 0;
    margin: auto;
    max-width: 100%;
    max-height: 100%;
    object-fit: contain;
  }

  // 原始大小模式：图片按倍率算出的内联宽度显示，超出容器时出滚动条；
  // margin auto 在空间充足时居中、不足时退化为左对齐，避免内容被裁剪
  &.is-actual {
    cursor: grab;
    user-select: none;

    img {
      max-width: none;
      max-height: none;
      object-fit: unset;
    }
  }

  &.dragging {
    cursor: grabbing;
  }
}
</style>
