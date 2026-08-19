<script setup lang="ts">
import { ref, computed } from "vue";
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
  modelValue: number | null;
  readonly?: boolean;
}>();

const emit = defineEmits<{
  "update:modelValue": [value: number | null];
}>();

const menuOpen = ref(false);
useMenuDismiss(menuOpen, ".dt-popper");

const dayDisplay = computed(() => {
  if (props.modelValue === null) return "";
  const dt = DateTime.fromMillis(props.modelValue, { zone: "utc" });
  return dt.isValid ? dt.toFormat("yyyy-MM-dd") : "";
});

function onPickerUpdate(val: unknown) {
  menuOpen.value = false;
  if (val === null || val === undefined) {
    emit("update:modelValue", null);
    return;
  }
  if (Array.isArray(val)) {
    if (val.length === 0) {
      emit("update:modelValue", null);
      return;
    }
    const d = val[0];
    if (!(d instanceof Date)) return;
    emit(
      "update:modelValue",
      partsToMs({ year: d.getFullYear(), month: d.getMonth() + 1, day: d.getDate(), hour: 0, minute: 0, second: 0 }, "day"),
    );
    return;
  }
  if (val instanceof Date) {
    emit(
      "update:modelValue",
      partsToMs({ year: val.getFullYear(), month: val.getMonth() + 1, day: val.getDate(), hour: 0, minute: 0, second: 0 }, "day"),
    );
  }
}

function onClear() {
  emit("update:modelValue", null);
}
</script>

<template>
  <VMenu v-model="menuOpen" :close-on-content-click="false">
    <template #activator="{ props: menuProps }">
      <VTextField
        :model-value="dayDisplay"
        :readonly="!readonly"
        :disabled="readonly"
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
        :model-value="modelValue !== null ? msToLocalDate(modelValue) : undefined"
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
