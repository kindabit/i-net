<!--
  数据节点颜色方案静态预览组件。

  在亮/暗两种强制主题环境下 mock 展示数据节点颜色方案的实际渲染效果，不依赖 vue-flow 上下文。
  通过 VThemeProvider 强制切换主题，左侧为亮色、右侧为暗色，实时反映草稿方案的视觉结果。
  点击预览卡片可切换选中态，用于预览选中边框色。
-->
<script setup lang="ts">
import { computed, ref } from "vue";
import { t } from "@/i18n";
import type { DataNodeColorScheme, DataNodeColorProperties } from "./index";

const props = defineProps<{
  /** 当前草稿方案 */
  scheme: DataNodeColorScheme;
  /** 预览用标题 */
  title: string;
  /** 预览用副标题 */
  subTitle: string;
}>();

/** 选中态：点击预览卡片切换（两个 pane 联动），用于预览选中边框色 */
const selected = ref(false);

/** pane 配置：数据驱动渲染两个 pane */
const panes = computed(() => [
  { theme: "light" as const, labelKey: "database.color-dialog.section-light", colors: props.scheme.light },
  { theme: "dark" as const, labelKey: "database.color-dialog.section-dark", colors: props.scheme.dark },
]);

/**
 * 卡片样式：背景、文字色、边框（按选中态取对应边框色）。
 * @param colors 当前主题颜色属性（缺失即默认值）
 * @param isSelected 是否选中态
 */
function cardStyle(colors: DataNodeColorProperties, isSelected: boolean) {
  return {
    backgroundColor: colors.background,
    color: colors.title,
    borderColor: isSelected ? colors.borderSelected : colors.borderUnselected,
  };
}

/**
 * handle 小圆点样式：有自定义色用之，否则 undefined 保留默认外观。
 * @param colors 当前主题颜色属性
 */
function handleStyle(colors: DataNodeColorProperties) {
  return colors.handle ? { background: colors.handle, borderColor: colors.handle } : undefined;
}
</script>

<template>
  <div class="node-color-preview">
    <VThemeProvider
      v-for="pane in panes"
      :key="pane.theme"
      :theme="pane.theme"
      with-background
      class="node-color-preview__provider"
    >
      <div class="node-color-preview__pane">
        <div class="node-color-preview__pane-title">
          {{ t(pane.labelKey) }}
        </div>
        <div
          class="node-color-preview__card"
          :class="{ 'node-color-preview__card--selected': selected }"
          :style="cardStyle(pane.colors, selected)"
          @click="selected = !selected"
        >
          <div class="node-color-preview__title-row">
            <VIcon
              icon="mdi-vector-square"
              size="14"
              class="node-color-preview__icon"
              :style="{ color: pane.colors.icon }"
            />
            <span class="node-color-preview__title-text">{{ title }}</span>
          </div>
          <div
            v-if="subTitle"
            class="node-color-preview__subtitle"
            :style="{ color: pane.colors.subtitle }"
          >
            {{ subTitle }}
          </div>
          <div class="node-color-preview__handle node-color-preview__handle--top" :style="handleStyle(pane.colors)" />
          <div class="node-color-preview__handle node-color-preview__handle--bottom" :style="handleStyle(pane.colors)" />
          <div class="node-color-preview__handle node-color-preview__handle--left" :style="handleStyle(pane.colors)" />
          <div class="node-color-preview__handle node-color-preview__handle--right" :style="handleStyle(pane.colors)" />
          <!-- mock 工具按钮排：与真实节点一致定位于卡片右上方 -->
          <div class="node-color-preview__actions frosted-glass">
            <VIcon icon="mdi-pencil-outline" size="12" :style="{ color: pane.colors.action }" />
            <VIcon icon="mdi-paperclip" size="12" :style="{ color: pane.colors.action }" />
            <VIcon icon="mdi-delete-outline" size="12" :style="{ color: pane.colors.action }" />
          </div>
        </div>
      </div>
    </VThemeProvider>
  </div>
</template>

<style lang="scss" scoped>
.node-color-preview {
  display: flex;
  gap: 1rem;
}

.node-color-preview__provider {
  flex: 1;
  display: flex;
  border-radius: 0.5rem;
}

.node-color-preview__pane {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 0.5rem;
  padding: 1rem;
  border-radius: 0.5rem;
}

.node-color-preview__pane-title {
  align-self: flex-start;
  font-size: 0.75rem;
  opacity: 0.7;
}

.node-color-preview__card {
  position: relative;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 0.25rem;
  padding: 0.75rem 1.25rem;
  width: 9rem;
  border-radius: 0.5rem;
  border: 0.125rem solid transparent;
  box-shadow: 0 0.125rem 0.5rem rgba(0, 0, 0, 0.12);
  cursor: pointer;
  transition: border-color 0.2s, box-shadow 0.2s;
  background-color: rgb(var(--v-theme-surface));
  color: rgb(var(--v-theme-on-surface));
}

.node-color-preview__card--selected {
  box-shadow: 0 0.25rem 1rem rgba(0, 0, 0, 0.2);
  border-color: rgb(var(--v-theme-primary));
}

.node-color-preview__title-row {
  display: flex;
  align-items: center;
  gap: 0.25rem;
}

.node-color-preview__icon {
  opacity: 0.6;
  flex-shrink: 0;
  color: rgb(var(--v-theme-on-surface));
}

.node-color-preview__title-text {
  font-size: 0.875rem;
  font-weight: 500;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  max-width: 7.5rem;
}

.node-color-preview__subtitle {
  font-size: 0.75rem;
  opacity: 0.6;
  color: rgb(var(--v-theme-on-surface));
}

.node-color-preview__handle {
  position: absolute;
  width: 0.625rem;
  height: 0.625rem;
  border-radius: 50%;
  background: #9e9e9e;
  border: 0.0625rem solid #9e9e9e;
}

.node-color-preview__handle--top {
  top: -0.375rem;
  left: 50%;
  transform: translateX(-50%);
}

.node-color-preview__handle--bottom {
  bottom: -0.375rem;
  left: 50%;
  transform: translateX(-50%);
}

.node-color-preview__handle--left {
  left: -0.375rem;
  top: 50%;
  transform: translateY(-50%);
}

.node-color-preview__handle--right {
  right: -0.375rem;
  top: 50%;
  transform: translateY(-50%);
}

.node-color-preview__actions {
  position: absolute;
  bottom: 100%;
  right: 0;
  margin-bottom: 0.375rem;
  display: flex;
  gap: 0.25rem;
  padding: 0.125rem 0.375rem;
  border-radius: 0.375rem;
  color: rgb(var(--v-theme-on-surface));
}
</style>
