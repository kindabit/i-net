<script setup lang="ts">
import { ref, watch } from "vue";

const props = defineProps<{
  modelValue: string | null;
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
    placeholder="0"
    variant="outlined"
    density="compact"
    hide-details="auto"
    @update:model-value="onInput()"
  />
</template>
