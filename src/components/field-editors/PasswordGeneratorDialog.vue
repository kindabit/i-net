<script setup lang="ts">
import { ref, watch } from "vue";
import { t } from "@/i18n";

const dialog = ref(false);
const length = ref(20);
const uppercase = ref(true);
const lowercase = ref(true);
const digits = ref(true);
const symbols = ref(false);
const preview = ref("");

let resolveOpen: ((value: string | null) => void) | null = null;

function open(): Promise<string | null> {
  settle(null);
  length.value = 20;
  uppercase.value = true;
  lowercase.value = true;
  digits.value = true;
  symbols.value = false;
  generate();
  dialog.value = true;
  return new Promise((resolve) => {
    resolveOpen = resolve;
  });
}

function settle(value: string | null) {
  resolveOpen?.(value);
  resolveOpen = null;
}

function generate() {
  const chars: string[] = [];
  if (uppercase.value) chars.push("ABCDEFGHIJKLMNOPQRSTUVWXYZ");
  if (lowercase.value) chars.push("abcdefghijklmnopqrstuvwxyz");
  if (digits.value) chars.push("0123456789");
  if (symbols.value) chars.push("!@#$%^&*()_+-=[]{}|;:,.<>?/~`");
  if (chars.length === 0) {
    preview.value = "";
    return;
  }
  const all = chars.join("");
  const random = crypto.getRandomValues(new Uint32Array(length.value));
  const result: string[] = [];
  for (let i = 0; i < length.value; i++) {
    result.push(all[random[i] % all.length]);
  }
  preview.value = result.join("");
}

const noCharsetSelected = ref(false);

function onConfirm() {
  if (!preview.value) {
    noCharsetSelected.value = true;
    return;
  }
  settle(preview.value);
  dialog.value = false;
}

watch(dialog, (value) => {
  if (!value) settle(null);
});

watch([uppercase, lowercase, digits, symbols, length], () => {
  noCharsetSelected.value = false;
  generate();
});

defineExpose({ open });
</script>

<template>
  <VDialog v-model="dialog" max-width="400">
    <VCard>
      <VCardTitle>{{ t("database.field-editor.password-generator-title") }}</VCardTitle>
      <VCardText>
        <div class="mb-4">
          <div class="text-body-2 mb-1">
            {{ t("database.field-editor.password-length") }}: {{ length }}
          </div>
          <VSlider
            v-model="length"
            :min="8"
            :max="64"
            thumb-label
            hide-details
          />
        </div>
        <VCheckbox
          v-model="uppercase"
          :label="t('database.field-editor.charset-uppercase')"
          density="compact"
          hide-details
        />
        <VCheckbox
          v-model="lowercase"
          :label="t('database.field-editor.charset-lowercase')"
          density="compact"
          hide-details
        />
        <VCheckbox
          v-model="digits"
          :label="t('database.field-editor.charset-digits')"
          density="compact"
          hide-details
        />
        <VCheckbox
          v-model="symbols"
          :label="t('database.field-editor.charset-symbols')"
          density="compact"
          hide-details
        />
        <div v-if="noCharsetSelected" class="text-error text-caption mt-1">
          {{ t("database.field-editor.charset-required") }}
        </div>
        <div class="d-flex align-center ga-2 mt-4">
          <VTextField
            :model-value="preview"
            readonly
            variant="outlined"
            density="compact"
            hide-details="auto"
            class="flex-grow-1"
          />
          <VIcon icon="mdi-refresh" class="cursor-pointer" @click="generate()" />
        </div>
      </VCardText>
      <VCardActions class="justify-end ga-2">
        <VBtn variant="text" @click="dialog = false">
          {{ t("common.cancel") }}
        </VBtn>
        <VBtn
          color="primary"
          variant="flat"
          :disabled="!preview"
          @click="onConfirm"
        >
          {{ t("database.field-editor.use-password") }}
        </VBtn>
      </VCardActions>
    </VCard>
  </VDialog>
</template>

<style lang="scss" scoped>
.mb-4 {
  margin-bottom: 1rem;
}

.mt-4 {
  margin-top: 1rem;
}

.mt-1 {
  margin-top: 0.25rem;
}

.mb-1 {
  margin-bottom: 0.25rem;
}
</style>
