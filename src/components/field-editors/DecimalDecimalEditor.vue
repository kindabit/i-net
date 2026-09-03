<!--
  decimal:decimal 字段值编辑器。

  普通单行输入框，输入时去除首尾空白；数字格式校验由字段类型目录在保存时执行。
-->
<script setup lang="ts">
import { ref, watch } from "vue";

const props = defineProps<{
  modelValue: string | null;
  /** 值错误高亮：输入控件进入错误高亮状态（不显示错误信息）。 */
  errorHighlight?: boolean;
  readonly?: boolean;
}>();

const emit = defineEmits<{
  "update:modelValue": [value: string | null];
}>();

const text = ref<string>(props.modelValue ?? "");

watch(
  () => props.modelValue,
  (v) => {
    text.value = v ?? "";
  }
);

function onInput() {
  const trimmed = text.value.trim();
  emit("update:modelValue", trimmed === "" ? null : trimmed);
}
</script>

<template>
  <VTextField
    v-model="text"
    :error="errorHighlight"
    :readonly="readonly"
    placeholder="0"
    variant="outlined"
    density="compact"
    hide-details="auto"
    @update:model-value="onInput()"
  />
</template>
