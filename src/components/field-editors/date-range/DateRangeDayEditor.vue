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

function msToLocalDate(ms: number): Date {
  const dt = DateTime.fromMillis(ms, { zone: "utc" });
  return new Date(dt.year, dt.month - 1, dt.day, dt.hour, dt.minute, dt.second);
}

const props = defineProps<{
  modelValue: [number, number] | null;
}>();

const emit = defineEmits<{
  "update:modelValue": [value: [number, number] | null];
}>();

const UNIT = "day";

const menuOpen = ref(false);
useMenuDismiss(menuOpen, ".dt-popper");

const rangePickerModel = ref<Date[] | undefined>(undefined);

function initLocalState() {
  const pair = props.modelValue;
  if (pair === null) {
    rangePickerModel.value = undefined;
  } else {
    rangePickerModel.value = [msToLocalDate(pair[0]), msToLocalDate(pair[1])];
  }
}
initLocalState();

const displayText = computed(() => {
  if (props.modelValue === null) return "";
  const dt0 = DateTime.fromMillis(props.modelValue[0], { zone: "utc" });
  const dt1 = DateTime.fromMillis(props.modelValue[1], { zone: "utc" });
  if (!dt0.isValid || !dt1.isValid) return "";
  return `${dt0.toFormat("yyyy-MM-dd")} ~ ${dt1.toFormat("yyyy-MM-dd")}`;
});

function dateToMs(d: Date): number {
  return partsToMs(
    {
      year: d.getFullYear(),
      month: d.getMonth() + 1,
      day: d.getDate(),
      hour: 0,
      minute: 0,
      second: 0,
    },
    UNIT,
  );
}

function localPairMs(): { start: number | null; end: number | null } {
  const rp = rangePickerModel.value;
  if (rp && rp.length >= 2) {
    const d0 = rp[0];
    const d1 = rp[rp.length - 1];
    if (d0 instanceof Date && d1 instanceof Date) {
      return { start: dateToMs(d0), end: dateToMs(d1) };
    }
  }
  return { start: null, end: null };
}

watch(menuOpen, (open) => {
  if (open) {
    const pair = props.modelValue;
    if (pair === null) {
      rangePickerModel.value = undefined;
    } else {
      rangePickerModel.value = [msToLocalDate(pair[0]), msToLocalDate(pair[1])];
    }
  }
});

function onPickerUpdate(val: unknown) {
  if (Array.isArray(val)) {
    rangePickerModel.value = val as Date[];
    if (val.length >= 2) {
      const d0 = val[0];
      const d1 = val[val.length - 1];
      if (d0 instanceof Date && d1 instanceof Date) {
        emit("update:modelValue", [dateToMs(d0), dateToMs(d1)]);
        menuOpen.value = false;
      }
    }
  }
}

function onClear() {
  rangePickerModel.value = undefined;
  emit("update:modelValue", null);
}

watch(() => props.modelValue, (pair) => {
  const local = localPairMs();
  const incomingStart = pair?.[0] ?? null;
  const incomingEnd = pair?.[1] ?? null;
  if (incomingStart === local.start && incomingEnd === local.end) return;
  if (pair === null) {
    rangePickerModel.value = undefined;
  } else {
    rangePickerModel.value = [msToLocalDate(pair[0]), msToLocalDate(pair[1])];
  }
});
</script>

<template>
  <VMenu v-model="menuOpen" :close-on-content-click="false">
    <template #activator="{ props: menuProps }">
      <VTextField
        :model-value="displayText"
        readonly
        clearable
        prepend-inner-icon="mdi-calendar"
        variant="outlined"
        density="compact"
        hide-details="auto"
        v-bind="menuProps"
        @click:clear="onClear"
      />
    </template>
    <div class="dt-popper">
      <VDatePicker
        v-if="menuOpen"
        :model-value="rangePickerModel"
        multiple="range"
        hide-header
        @update:model-value="onPickerUpdate"
      />
    </div>
  </VMenu>
</template>

<style lang="scss" scoped>
.dt-popper {
  display: inline-block;
  background: rgb(var(--v-theme-surface));
  border-radius: 0.5rem;
  box-shadow: 0 0.5rem 1.5rem rgba(0, 0, 0, 0.25);
}
</style>
