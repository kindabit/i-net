<script setup lang="ts">
import { ref, watch } from "vue";

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
  <VTextarea
    v-model="text"
    :readonly="readonly"
    rows="3"
    auto-grow
    variant="outlined"
    density="compact"
    hide-details="auto"
    @update:model-value="onInput()"
  />
</template>
