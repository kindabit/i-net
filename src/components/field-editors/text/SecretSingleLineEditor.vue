<script setup lang="ts">
import { computed, ref, watch } from "vue";

const props = defineProps<{
  modelValue: string | null;
  readonly?: boolean;
}>();

const emit = defineEmits<{
  "update:modelValue": [value: string | null];
}>();

const text = ref<string>(props.modelValue ?? "");

const visible = ref(false);

watch(
  () => props.modelValue,
  (v) => {
    text.value = v ?? "";
  }
);

function onInput() {
  emit("update:modelValue", text.value === "" ? null : text.value);
}

const inputType = computed<string>(() =>
  visible.value ? "text" : "password"
);

const eyeIcon = computed<string>(() =>
  visible.value ? "mdi-eye-off" : "mdi-eye"
);
</script>

<template>
  <VTextField
    v-model="text"
    :type="inputType"
    :append-inner-icon="eyeIcon"
    :readonly="readonly"
    variant="outlined"
    density="compact"
    hide-details="auto"
    @update:model-value="onInput()"
    @click:append-inner="visible = !visible"
  />
</template>
