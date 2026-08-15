<!--
  双色对角三角预览色块组件。

  以对角三角分割方式同时展示亮色与暗色两个颜色值，用于预设和历史记录的快速视觉辨识。
  左上三角为亮色主题色，右下三角为暗色主题色；对应主题色缺失（使用默认值）时该三角显示默认斜线纹。
  可选 tooltip 展示详细信息，文本中的换行符会被真实渲染为多行。
-->
<script setup lang="ts">
withDefaults(
  defineProps<{
    /** 亮色主题色（左上三角），为空表示使用默认值 */
    lightColor?: string;
    /** 暗色主题色（右下三角），为空表示使用默认值 */
    darkColor?: string;
    /** tooltip 文本，为空时不渲染 tooltip */
    tooltip?: string;
  }>(),
  { lightColor: undefined, darkColor: undefined, tooltip: "" },
);

const emit = defineEmits<{ click: [] }>();
</script>

<template>
  <VTooltip :disabled="!tooltip" location="top">
    <template #activator="{ props: activatorProps }">
      <div class="color-pair-swatch" v-bind="activatorProps" @click="emit('click')">
        <div
          class="color-pair-swatch__top-left"
          :class="{ 'color-pair-swatch__triangle--default': lightColor === undefined }"
          :style="{ backgroundColor: lightColor }"
        />
        <div
          class="color-pair-swatch__bottom-right"
          :class="{ 'color-pair-swatch__triangle--default': darkColor === undefined }"
          :style="{ backgroundColor: darkColor }"
        />
      </div>
    </template>
    <div class="color-pair-swatch__tooltip">{{ tooltip }}</div>
  </VTooltip>
</template>

<style lang="scss" scoped>
.color-pair-swatch {
  position: relative;
  width: 1.5rem;
  height: 1.5rem;
  border-radius: 0.25rem;
  overflow: hidden;
  cursor: pointer;
  flex-shrink: 0;
  transition: box-shadow 0.15s ease;

  &:hover {
    box-shadow: inset 0 0 0 0.125rem rgb(var(--v-theme-primary));
  }
}

.color-pair-swatch__top-left,
.color-pair-swatch__bottom-right {
  position: absolute;
  inset: 0;
}

.color-pair-swatch__top-left {
  clip-path: polygon(0 0, 100% 0, 0 100%);
}

.color-pair-swatch__bottom-right {
  clip-path: polygon(100% 100%, 100% 0, 0 100%);
}

.color-pair-swatch__triangle--default {
  background-image: linear-gradient(
    45deg,
    transparent 46%,
    rgba(var(--v-theme-on-surface), 0.36) 46%,
    rgba(var(--v-theme-on-surface), 0.36) 54%,
    transparent 54%
  );
}

.color-pair-swatch__tooltip {
  white-space: pre-line;
}
</style>
