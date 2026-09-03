<!--
  string:multiple-line 字段值编辑器。

  多行文本域。
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
  emit("update:modelValue", text.value === "" ? null : text.value);
}
</script>

<template>
  <VTextarea
    v-model="text"
    :error="errorHighlight"
    :readonly="readonly"
    rows="3"
    auto-grow
    variant="outlined"
    density="compact"
    hide-details="auto"
    @update:model-value="onInput()"
  />
</template>
