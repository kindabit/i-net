<script setup lang="ts">
import { ref, computed, watch } from "vue";
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

const props = defineProps<{
  modelValue: [number, number] | null;
  readonly?: boolean;
}>();

const emit = defineEmits<{
  "update:modelValue": [value: [number, number] | null];
}>();

const UNIT = "year";

function msToYear(ms: number | null): number | undefined {
  if (ms === null) return undefined;
  const dt = DateTime.fromMillis(ms, { zone: "utc" });
  return dt.isValid ? dt.year : undefined;
}

function yearToMs(year: number | null | undefined): number | null {
  if (year === null || year === undefined) return null;
  const y = Math.round(Number(year));
  if (isNaN(y) || y < 0 || y > 9999) return null;
  return partsToMs({ year: y, month: 1, day: 1, hour: 0, minute: 0, second: 0 }, UNIT);
}

const startMs = ref<number | null>(null);
const endMs = ref<number | null>(null);

function initLocalState() {
  const pair = props.modelValue;
  startMs.value = pair?.[0] ?? null;
  endMs.value = pair?.[1] ?? null;
}
initLocalState();

const startYear = computed(() => msToYear(startMs.value));
const endYear = computed(() => msToYear(endMs.value));

function tryEmit() {
  const s = startMs.value;
  const e = endMs.value;
  if (s !== null && e !== null) {
    emit("update:modelValue", [s, e]);
  }
}

function onStartInput(val: number | null | undefined) {
  if (val === null || val === undefined) {
    startMs.value = null;
    emit("update:modelValue", null);
    return;
  }
  const ms = yearToMs(val);
  if (ms === null) return;
  startMs.value = ms;
  tryEmit();
}

function onEndInput(val: number | null | undefined) {
  if (val === null || val === undefined) {
    endMs.value = null;
    emit("update:modelValue", null);
    return;
  }
  const ms = yearToMs(val);
  if (ms === null) return;
  endMs.value = ms;
  tryEmit();
}

watch(() => props.modelValue, (pair) => {
  const incomingStart = pair?.[0] ?? null;
  const incomingEnd = pair?.[1] ?? null;
  if (incomingStart === startMs.value && incomingEnd === endMs.value) return;
  startMs.value = incomingStart;
  endMs.value = incomingEnd;
});
</script>

<template>
  <div class="range-row">
    <VNumberInput
      :model-value="startYear"
      :min="0"
      :max="9999"
      :readonly="readonly"
      :label="t('database.field-editor.range-start-label')"
      variant="outlined"
      density="compact"
      hide-details="auto"
      control-variant="stacked"
      @update:model-value="onStartInput"
    />
    <VNumberInput
      :model-value="endYear"
      :min="0"
      :max="9999"
      :readonly="readonly"
      :label="t('database.field-editor.range-end-label')"
      variant="outlined"
      density="compact"
      hide-details="auto"
      control-variant="stacked"
      @update:model-value="onEndInput"
    />
  </div>
</template>

<style lang="scss" scoped>
.range-row {
  display: flex;
  gap: 0.5rem;
}

.range-row > * {
  flex: 1;
}
</style>
