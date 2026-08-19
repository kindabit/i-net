<script setup lang="ts">
import { DateTime } from "luxon";
import { t } from "@/i18n";

interface DateTimeParts {
  year: number;
  month: number;
  day: number;
  hour: number;
  minute: number;
  second: number;
}

function partsToMs(parts: DateTimeParts, unit: string): number {
  return DateTime.fromObject(parts, { zone: "utc" })
    .startOf(unit as any)
    .toMillis();
}

defineProps<{
  modelValue: number | null;
  readonly?: boolean;
}>();

const emit = defineEmits<{
  "update:modelValue": [value: number | null];
}>();

function msToYear(ms: number | null): number | undefined {
  if (ms === null) return undefined;
  const dt = DateTime.fromMillis(ms, { zone: "utc" });
  return dt.isValid ? dt.year : undefined;
}

function onYearInput(val: number | null | undefined) {
  if (val === null || val === undefined) {
    emit("update:modelValue", null);
    return;
  }
  const year = Math.round(Number(val));
  if (isNaN(year) || year < 0 || year > 9999) return;
  emit(
    "update:modelValue",
    partsToMs({ year, month: 1, day: 1, hour: 0, minute: 0, second: 0 }, "year"),
  );
}
</script>

<template>
  <VNumberInput
    :model-value="msToYear(modelValue)"
    :min="0"
    :max="9999"
    :readonly="readonly"
    :label="t('database.field-editor.year-label')"
    variant="outlined"
    density="compact"
    hide-details="auto"
    control-variant="stacked"
    @update:model-value="onYearInput"
  />
</template>
