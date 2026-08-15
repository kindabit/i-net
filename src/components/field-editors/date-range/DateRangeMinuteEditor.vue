<script setup lang="ts">
import { ref, computed, watch } from "vue";
import { DateTime } from "luxon";
import { useMenuDismiss } from "@/composables/use-menu-dismiss";
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
}>();

const emit = defineEmits<{
  "update:modelValue": [value: [number, number] | null];
}>();

const UNIT = "minute";
const TIME_FORMAT = "HH:mm";

const startDate = ref<Date | null>(null);
const startTime = ref<string | null>(null);
const endDate = ref<Date | null>(null);
const endTime = ref<string | null>(null);

const startDateMenuOpen = ref(false);
const startTimeMenuOpen = ref(false);
const endDateMenuOpen = ref(false);
const endTimeMenuOpen = ref(false);

useMenuDismiss(startDateMenuOpen, ".dt-popper");
useMenuDismiss(startTimeMenuOpen, ".dt-popper");
useMenuDismiss(endDateMenuOpen, ".dt-popper");
useMenuDismiss(endTimeMenuOpen, ".dt-popper");

function msToDateAndTime(ms: number | null): { date: Date | null; time: string | null } {
  if (ms === null) return { date: null, time: null };
  const dt = DateTime.fromMillis(ms, { zone: "utc" });
  if (!dt.isValid) return { date: null, time: null };
  return {
    date: new Date(dt.year, dt.month - 1, dt.day, dt.hour, dt.minute, dt.second),
    time: dt.toFormat(TIME_FORMAT),
  };
}

function initLocalState() {
  const pair = props.modelValue;
  const startDT = msToDateAndTime(pair?.[0] ?? null);
  const endDT = msToDateAndTime(pair?.[1] ?? null);
  startDate.value = startDT.date;
  startTime.value = startDT.time;
  endDate.value = endDT.date;
  endTime.value = endDT.time;
}
initLocalState();

const startDateDisplay = computed(() => {
  if (startDate.value === null) return "";
  const d = startDate.value;
  const dt = DateTime.fromObject(
    { year: d.getFullYear(), month: d.getMonth() + 1, day: d.getDate() },
    { zone: "utc" },
  );
  return dt.toFormat("yyyy-MM-dd");
});

const endDateDisplay = computed(() => {
  if (endDate.value === null) return "";
  const d = endDate.value;
  const dt = DateTime.fromObject(
    { year: d.getFullYear(), month: d.getMonth() + 1, day: d.getDate() },
    { zone: "utc" },
  );
  return dt.toFormat("yyyy-MM-dd");
});

function combineMs(date: Date | null, time: string | null): number | null {
  if (date === null || time === null) return null;
  const parts = time.split(":").map(Number);
  return partsToMs(
    {
      year: date.getFullYear(),
      month: date.getMonth() + 1,
      day: date.getDate(),
      hour: parts[0] ?? 0,
      minute: parts[1] ?? 0,
      second: parts[2] ?? 0,
    },
    UNIT,
  );
}

function startMs(): number | null {
  return combineMs(startDate.value, startTime.value);
}

function endMs(): number | null {
  return combineMs(endDate.value, endTime.value);
}

function tryEmit() {
  const s = startMs();
  const e = endMs();
  if (s !== null && e !== null) {
    emit("update:modelValue", [s, e]);
  }
}

watch(() => props.modelValue, (pair) => {
  const localStart = startMs();
  const localEnd = endMs();
  const incomingStart = pair?.[0] ?? null;
  const incomingEnd = pair?.[1] ?? null;
  if (incomingStart === localStart && incomingEnd === localEnd) return;
  initLocalState();
});

function onStartDateUpdate(val: unknown) {
  startDateMenuOpen.value = false;
  if (val === null || val === undefined) {
    startDate.value = null;
    emit("update:modelValue", null);
    return;
  }
  if (Array.isArray(val)) {
    if (val.length === 0) {
      startDate.value = null;
      emit("update:modelValue", null);
      return;
    }
    const d = val[0];
    if (!(d instanceof Date)) return;
    startDate.value = d;
    tryEmit();
    return;
  }
  if (val instanceof Date) {
    startDate.value = val;
    tryEmit();
  }
}

function onStartDateClear() {
  startDate.value = null;
  emit("update:modelValue", null);
}

function onStartTimeUpdate(val: string | null) {
  startTime.value = val;
  tryEmit();
}

function onStartTimeClear() {
  startTime.value = null;
  emit("update:modelValue", null);
}

function onEndDateUpdate(val: unknown) {
  endDateMenuOpen.value = false;
  if (val === null || val === undefined) {
    endDate.value = null;
    emit("update:modelValue", null);
    return;
  }
  if (Array.isArray(val)) {
    if (val.length === 0) {
      endDate.value = null;
      emit("update:modelValue", null);
      return;
    }
    const d = val[0];
    if (!(d instanceof Date)) return;
    endDate.value = d;
    tryEmit();
    return;
  }
  if (val instanceof Date) {
    endDate.value = val;
    tryEmit();
  }
}

function onEndDateClear() {
  endDate.value = null;
  emit("update:modelValue", null);
}

function onEndTimeUpdate(val: string | null) {
  endTime.value = val;
  tryEmit();
}

function onEndTimeClear() {
  endTime.value = null;
  emit("update:modelValue", null);
}
</script>

<template>
  <div class="range-datetime">
    <div class="datetime-row">
      <VMenu v-model="startDateMenuOpen" :close-on-content-click="false">
        <template #activator="{ props: menuProps }">
          <VTextField
            :model-value="startDateDisplay"
            readonly
            clearable
            prepend-inner-icon="mdi-calendar"
            :label="t('database.field-editor.range-start-label')"
            variant="outlined"
            density="compact"
            hide-details="auto"
            v-bind="menuProps"
            @click:clear="onStartDateClear"
          />
        </template>
        <div class="dt-popper">
          <VDatePicker
            v-if="startDateMenuOpen"
            :model-value="startDate"
            hide-header
            @update:model-value="onStartDateUpdate"
          />
        </div>
      </VMenu>
      <VMenu v-model="startTimeMenuOpen" :close-on-content-click="false">
        <template #activator="{ props: menuProps }">
          <VTextField
            :model-value="startTime ?? ''"
            readonly
            clearable
            prepend-inner-icon="mdi-clock-outline"
            variant="outlined"
            density="compact"
            hide-details="auto"
            v-bind="menuProps"
            @click:clear="onStartTimeClear"
          />
        </template>
        <div class="dt-popper">
          <VTimePicker
            v-if="startTimeMenuOpen"
            :model-value="startTime"
            format="24hr"
            @update:model-value="onStartTimeUpdate"
          />
        </div>
      </VMenu>
    </div>
    <div class="datetime-row">
      <VMenu v-model="endDateMenuOpen" :close-on-content-click="false">
        <template #activator="{ props: menuProps }">
          <VTextField
            :model-value="endDateDisplay"
            readonly
            clearable
            prepend-inner-icon="mdi-calendar"
            :label="t('database.field-editor.range-end-label')"
            variant="outlined"
            density="compact"
            hide-details="auto"
            v-bind="menuProps"
            @click:clear="onEndDateClear"
          />
        </template>
        <div class="dt-popper">
          <VDatePicker
            v-if="endDateMenuOpen"
            :model-value="endDate"
            hide-header
            @update:model-value="onEndDateUpdate"
          />
        </div>
      </VMenu>
      <VMenu v-model="endTimeMenuOpen" :close-on-content-click="false">
        <template #activator="{ props: menuProps }">
          <VTextField
            :model-value="endTime ?? ''"
            readonly
            clearable
            prepend-inner-icon="mdi-clock-outline"
            variant="outlined"
            density="compact"
            hide-details="auto"
            v-bind="menuProps"
            @click:clear="onEndTimeClear"
          />
        </template>
        <div class="dt-popper">
          <VTimePicker
            v-if="endTimeMenuOpen"
            :model-value="endTime"
            format="24hr"
            @update:model-value="onEndTimeUpdate"
          />
        </div>
      </VMenu>
    </div>
  </div>
</template>

<style lang="scss" scoped>
.range-datetime {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.datetime-row {
  display: flex;
  gap: 0.5rem;
}

.datetime-row > * {
  flex: 1;
}

.dt-popper {
  display: inline-block;
  background: rgb(var(--v-theme-surface));
  border-radius: 0.5rem;
  box-shadow: 0 0.5rem 1.5rem rgba(0, 0, 0, 0.25);
}
</style>
