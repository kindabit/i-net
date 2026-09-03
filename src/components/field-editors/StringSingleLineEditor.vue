<!--
  string:single-line 字段值编辑器。

  绑定字典时以 combobox 提供候选值，否则为普通单行输入框。
-->
<script setup lang="ts">
import { ref, watch } from "vue";

const props = defineProps<{
  modelValue: string | null;
  dictionaryItems?: string[];
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
  <VCombobox
    v-if="dictionaryItems && dictionaryItems.length > 0"
    v-model="text"
    :items="dictionaryItems"
    :error="errorHighlight"
    :readonly="readonly"
    clearable
    variant="outlined"
    density="compact"
    hide-details="auto"
    @update:model-value="onInput()"
  />
  <VTextField
    v-else
    v-model="text"
    :error="errorHighlight"
    :readonly="readonly"
    variant="outlined"
    density="compact"
    hide-details="auto"
    @update:model-value="onInput()"
  />
</template>
