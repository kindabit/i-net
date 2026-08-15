<script setup lang="ts">
import { ref, watch } from "vue";

const props = defineProps<{
  modelValue: string | null;
  dictionaryItems?: string[];
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
    clearable
    variant="outlined"
    density="compact"
    hide-details="auto"
    @update:model-value="onInput()"
  />
  <VTextField
    v-else
    v-model="text"
    variant="outlined"
    density="compact"
    hide-details="auto"
    @update:model-value="onInput()"
  />
</template>
