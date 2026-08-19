<script setup lang="ts">
import { ref, computed, watch } from "vue";
import { DateTime } from "luxon";
import { useMenuDismiss } from "@/composables/use-menu-dismiss";
import { VMonthPicker } from "vuetify/labs/VMonthPicker";
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

const UNIT = "month";

function msToMonthStr(ms: number | null): string | null {
  if (ms === null) return null;
  const dt = DateTime.fromMillis(ms, { zone: "utc" });
  return dt.isValid ? dt.toFormat("yyyy-MM") : null;
}

const startMs = ref<number | null>(null);
const endMs = ref<number | null>(null);

function initLocalState() {
  const pair = props.modelValue;
  startMs.value = pair?.[0] ?? null;
  endMs.value = pair?.[1] ?? null;
}
initLocalState();

const startMenuOpen = ref(false);
const endMenuOpen = ref(false);
useMenuDismiss(startMenuOpen, ".dt-popper");
useMenuDismiss(endMenuOpen, ".dt-popper");

const startDisplay = computed(() => {
  if (startMs.value === null) return "";
  return msToMonthStr(startMs.value) ?? "";
});

const endDisplay = computed(() => {
  if (endMs.value === null) return "";
  return msToMonthStr(endMs.value) ?? "";
});

const startPickerModel = ref<string | null>(null);
const endPickerModel = ref<string | null>(null);

watch(startMenuOpen, (open) => {
  if (open) {
    startPickerModel.value = msToMonthStr(startMs.value);
  }
});

watch(endMenuOpen, (open) => {
  if (open) {
    endPickerModel.value = msToMonthStr(endMs.value);
  }
});

function tryEmit() {
  const s = startMs.value;
  const e = endMs.value;
  if (s !== null && e !== null) {
    emit("update:modelValue", [s, e]);
  }
}

function onStartPickerUpdate(val: string | null) {
  startMenuOpen.value = false;
  if (val === null || val === "") {
    startMs.value = null;
    emit("update:modelValue", null);
    return;
  }
  const [y, m] = val.split("-").map(Number);
  if (isNaN(y) || isNaN(m)) return;
  startMs.value = partsToMs(
    { year: y, month: m, day: 1, hour: 0, minute: 0, second: 0 },
    UNIT,
  );
  tryEmit();
}

function onEndPickerUpdate(val: string | null) {
  endMenuOpen.value = false;
  if (val === null || val === "") {
    endMs.value = null;
    emit("update:modelValue", null);
    return;
  }
  const [y, m] = val.split("-").map(Number);
  if (isNaN(y) || isNaN(m)) return;
  endMs.value = partsToMs(
    { year: y, month: m, day: 1, hour: 0, minute: 0, second: 0 },
    UNIT,
  );
  tryEmit();
}

function onStartClear() {
  startMs.value = null;
  emit("update:modelValue", null);
}

function onEndClear() {
  endMs.value = null;
  emit("update:modelValue", null);
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
    <VMenu v-model="startMenuOpen" :close-on-content-click="false">
      <template #activator="{ props: menuProps }">
        <VTextField
          :model-value="startDisplay"
          :readonly="!readonly"
          :disabled="readonly"
          clearable
          prepend-inner-icon="mdi-calendar"
          :label="t('database.field-editor.range-start-label')"
          variant="outlined"
          density="compact"
          hide-details="auto"
          v-bind="menuProps"
          @click:clear="onStartClear"
        />
      </template>
      <div class="dt-popper">
        <VMonthPicker
          v-if="startMenuOpen"
          :model-value="startPickerModel"
          @update:model-value="onStartPickerUpdate"
        />
      </div>
    </VMenu>
    <VMenu v-model="endMenuOpen" :close-on-content-click="false">
      <template #activator="{ props: menuProps }">
        <VTextField
          :model-value="endDisplay"
          :readonly="!readonly"
          :disabled="readonly"
          clearable
          prepend-inner-icon="mdi-calendar"
          :label="t('database.field-editor.range-end-label')"
          variant="outlined"
          density="compact"
          hide-details="auto"
          v-bind="menuProps"
          @click:clear="onEndClear"
        />
      </template>
      <div class="dt-popper">
        <VMonthPicker
          v-if="endMenuOpen"
          :model-value="endPickerModel"
          @update:model-value="onEndPickerUpdate"
        />
      </div>
    </VMenu>
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

.dt-popper {
  display: inline-block;
  background: rgb(var(--v-theme-surface));
  border-radius: 0.5rem;
  box-shadow: 0 0.5rem 1.5rem rgba(0, 0, 0, 0.25);
}
</style>
