<!--
  string:secret 字段值编辑器。

  基于 PasswordField 提供常驻的可见性切换图标。
-->
<script setup lang="ts">
import { ref, watch } from "vue";
import PasswordField from "@/components/PasswordField.vue";

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
  <PasswordField
    v-model="text"
    :error="errorHighlight"
    :readonly="readonly"
    density="compact"
    @update:model-value="onInput()"
  />
</template>
