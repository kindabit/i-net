<!--
  单个颜色项编辑器组件。

  由标签、色块按钮、颜色选择器菜单、清除按钮和 hex 文本组成。
  色块展示当前颜色（modelValue 缺失时展示默认态斜线纹），点击弹出 VColorPicker，
  清除按钮触发 reset 事件（由父组件将该颜色键恢复为默认，即置为缺失）。
-->
<script setup lang="ts">
import { ref, computed } from "vue";
import { t } from "@/i18n";
import { parseVuetifyColor } from "./vuetify-color";

const props = defineProps<{
  /** 标签文本 */
  label: string;
  /** 当前颜色值（缺失表示默认态） */
  modelValue?: string;
}>();

const emit = defineEmits<{
  "update:modelValue": [value: string];
  reset: [];
}>();

const menu = ref(false);

/** 色块是否处于默认态（颜色键缺失） */
const isDefault = computed(() => props.modelValue === undefined);

/**
 * 颜色选择器变化回调：正规化为小写 #rrggbbaa 后向上 emit。
 * @param value VColorPicker 输出（hex 字符串或 rgba 对象等）
 */
function onColorPick(value: unknown): void {
  emit("update:modelValue", parseVuetifyColor(value));
}

/** 选择器初始值：默认态时给一个中性灰以便用户操作 */
const pickerInitial = computed(
  () => parseVuetifyColor(props.modelValue) || "#888888",
);

/** 右侧展示的 hex 文本：默认态显示 "—" */
const hexText = computed(() => (isDefault.value ? "—" : props.modelValue));
</script>

<template>
  <div class="color-field-editor">
    <span class="color-field-editor__label">{{ label }}</span>
    <VMenu v-model="menu" :close-on-content-click="false" location="start">
      <template #activator="{ props: activatorProps }">
        <div
          class="color-field-editor__swatch"
          :class="{ 'color-field-editor__swatch--default': isDefault }"
          :style="{ backgroundColor: modelValue }"
          v-bind="activatorProps"
        />
      </template>
      <VColorPicker
        :model-value="pickerInitial"
        mode="rgba"
        @update:model-value="onColorPick"
      />
    </VMenu>
    <VBtn
      icon="mdi-close"
      size="x-small"
      variant="text"
      density="comfortable"
      :title="t('database.color-dialog.field-reset')"
      @click="emit('reset')"
    />
    <span class="color-field-editor__hex">{{ hexText }}</span>
  </div>
</template>

<style lang="scss" scoped>
.color-field-editor {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.color-field-editor__label {
  flex: 1;
  font-size: 0.875rem;
}

.color-field-editor__swatch {
  width: 1.5rem;
  height: 1.5rem;
  border: 0.125rem solid rgba(var(--v-theme-on-surface), 0.24);
  border-radius: 0.25rem;
  cursor: pointer;
  flex-shrink: 0;
}

.color-field-editor__swatch--default {
  background-image: linear-gradient(
    45deg,
    transparent 46%,
    rgba(var(--v-theme-on-surface), 0.36) 46%,
    rgba(var(--v-theme-on-surface), 0.36) 54%,
    transparent 54%
  );
}

.color-field-editor__hex {
  font-family: monospace;
  font-size: 0.75rem;
  opacity: 0.7;
  min-width: 4.5rem;
}
</style>
