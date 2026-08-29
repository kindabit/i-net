<!--
  密码字段值编辑器。

  基于 PasswordField 提供常驻的可见性切换图标，并通过其 append-inner
  插槽追加密码生成器入口图标。
-->
<script setup lang="ts">
import { ref, watch } from "vue";
import PasswordField from "@/components/PasswordField.vue";
import PasswordGeneratorDialog from "./PasswordGeneratorDialog.vue";

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
  <PasswordField
    v-model="text"
    :readonly="readonly"
    density="compact"
    @update:model-value="onInput()"
  >
    <template #append-inner>
      <VIcon
        v-if="!readonly"
        icon="mdi-auto-fix"
        class="cursor-pointer ms-1"
        tabindex="-1"
        @click="openPasswordGenerator()"
      />
    </template>
  </PasswordField>
  <PasswordGeneratorDialog ref="pgDialogRef" />
</template>

<style lang="scss" scoped>
.cursor-pointer {
  cursor: pointer;
}

.ms-1 {
  margin-inline-start: 0.25rem;
}
</style>
