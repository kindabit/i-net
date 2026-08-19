<!--
  单行密文字段值编辑器。

  基于 PasswordField 提供常驻的可见性切换图标。
-->
<script setup lang="ts">
import { ref, watch } from "vue";
import PasswordField from "@/components/PasswordField.vue";

const props = defineProps<{
  modelValue: string | null;
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
    :readonly="readonly"
    density="compact"
    @update:model-value="onInput()"
  />
</template>
