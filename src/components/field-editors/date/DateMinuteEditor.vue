<script setup lang="ts">
import { ref, computed, watch } from "vue";
import { DateTime } from "luxon";
import { useMenuDismiss } from "@/composables/use-menu-dismiss";

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
  modelValue: number | null;
  readonly?: boolean;
}>();

const emit = defineEmits<{
  "update:modelValue": [value: number | null];
}>();

const datePart = ref<Date | null>(null);
const timePart = ref<string | null>(null);
const dateMenuOpen = ref(false);
const timeMenuOpen = ref(false);
useMenuDismiss(dateMenuOpen, ".dt-popper");
useMenuDismiss(timeMenuOpen, ".dt-popper");

const UNIT = "minute";
const TIME_FORMAT = "HH:mm";

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
  const { date, time } = msToDateAndTime(props.modelValue);
  datePart.value = date;
  timePart.value = time;
}
initLocalState();

const dateDisplay = computed(() => {
  if (datePart.value === null) return "";
  const d = datePart.value;
  const dt = DateTime.fromObject(
    { year: d.getFullYear(), month: d.getMonth() + 1, day: d.getDate() },
    { zone: "utc" },
  );
  return dt.toFormat("yyyy-MM-dd");
});

function combineMs(): number | null {
  if (datePart.value === null || timePart.value === null) return null;
  const d = datePart.value;
  const parts = timePart.value.split(":").map(Number);
  return partsToMs(
    { year: d.getFullYear(), month: d.getMonth() + 1, day: d.getDate(), hour: parts[0] ?? 0, minute: parts[1] ?? 0, second: parts[2] ?? 0 },
    UNIT,
  );
}

function tryEmit() {
  const ms = combineMs();
  if (ms !== null) {
    emit("update:modelValue", ms);
  }
}

watch(() => props.modelValue, (ms) => {
  const local = combineMs();
  if (ms === local) return;
  initLocalState();
});

function onDatePickerUpdate(val: unknown) {
  dateMenuOpen.value = false;
  if (val === null || val === undefined) {
    datePart.value = null;
    emit("update:modelValue", null);
    return;
  }
  if (Array.isArray(val)) {
    if (val.length === 0) {
      datePart.value = null;
      emit("update:modelValue", null);
      return;
    }
    const d = val[0];
    if (!(d instanceof Date)) return;
    datePart.value = d;
    tryEmit();
    return;
  }
  if (val instanceof Date) {
    datePart.value = val;
    tryEmit();
  }
}

function onDateClear() {
  datePart.value = null;
  emit("update:modelValue", null);
}

function onTimePickerUpdate(val: string | null) {
  timePart.value = val;
  tryEmit();
}

function onTimeClear() {
  timePart.value = null;
  emit("update:modelValue", null);
}
</script>

<template>
  <div class="datetime-row">
    <VMenu v-model="dateMenuOpen" :close-on-content-click="false">
      <template #activator="{ props: menuProps }">
        <VTextField
          :model-value="dateDisplay"
          :readonly="!readonly"
          :disabled="readonly"
          clearable
          prepend-inner-icon="mdi-calendar"
          variant="outlined"
          density="compact"
          hide-details="auto"
          v-bind="menuProps"
          @click:clear="onDateClear"
        />
      </template>
      <div class="dt-popper">
        <VDatePicker
          v-if="dateMenuOpen"
          :model-value="datePart"
          hide-header
          @update:model-value="onDatePickerUpdate"
        />
      </div>
    </VMenu>
    <VMenu v-model="timeMenuOpen" :close-on-content-click="false">
      <template #activator="{ props: menuTimeProps }">
        <VTextField
          :model-value="timePart ?? ''"
          :readonly="!readonly"
          :disabled="readonly"
          clearable
          prepend-inner-icon="mdi-clock-outline"
          variant="outlined"
          density="compact"
          hide-details="auto"
          v-bind="menuTimeProps"
          @click:clear="onTimeClear"
        />
      </template>
      <div class="dt-popper">
        <VTimePicker
          v-if="timeMenuOpen"
          :model-value="timePart"
          format="24hr"
          @update:model-value="onTimePickerUpdate"
        />
      </div>
    </VMenu>
  </div>
</template>

<style lang="scss" scoped>
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
