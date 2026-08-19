<script setup lang="ts">
import { computed, ref, watch } from "vue";
import PasswordGeneratorDialog from "./PasswordGeneratorDialog.vue";

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

const pgDialogRef = ref<InstanceType<typeof PasswordGeneratorDialog> | null>(null);

async function openPasswordGenerator() {
  const result = await pgDialogRef.value?.open();
  if (result) {
    text.value = result;
    onInput();
  }
}
</script>

<template>
  <VTextField
    v-model="text"
    :type="inputType"
    :readonly="readonly"
    variant="outlined"
    density="compact"
    hide-details="auto"
    @update:model-value="onInput()"
  >
    <template #append-inner>
      <VIcon
        :icon="eyeIcon"
        class="cursor-pointer me-1"
        @click="visible = !visible"
      />
      <VIcon
        v-if="!readonly"
        icon="mdi-auto-fix"
        class="cursor-pointer"
        @click="openPasswordGenerator()"
      />
    </template>
  </VTextField>
  <PasswordGeneratorDialog ref="pgDialogRef" />
</template>

<style lang="scss" scoped>
.cursor-pointer {
  cursor: pointer;
}

.me-1 {
  margin-inline-end: 0.25rem;
}
</style>
