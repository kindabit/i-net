<script setup lang="ts">
import { ref, computed, watch } from "vue";
import { DateTime } from "luxon";
import { useMenuDismiss } from "@/composables/use-menu-dismiss";
import { VMonthPicker } from "vuetify/labs/VMonthPicker";

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

const menuOpen = ref(false);
useMenuDismiss(menuOpen, ".dt-popper");

function msToMonthStr(ms: number | null): string | null {
  if (ms === null) return null;
  const dt = DateTime.fromMillis(ms, { zone: "utc" });
  return dt.isValid ? dt.toFormat("yyyy-MM") : null;
}

const monthDisplay = computed(() => {
  if (props.modelValue === null) return "";
  return msToMonthStr(props.modelValue) ?? "";
});

const pickerModel = ref<string | null>(null);

watch(menuOpen, (open) => {
  if (open) {
    pickerModel.value = msToMonthStr(props.modelValue);
  }
});

function onPickerUpdate(val: string | null) {
  menuOpen.value = false;
  if (val === null || val === "") {
    emit("update:modelValue", null);
    return;
  }
  const [y, m] = val.split("-").map(Number);
  if (isNaN(y) || isNaN(m)) return;
  emit(
    "update:modelValue",
    partsToMs({ year: y, month: m, day: 1, hour: 0, minute: 0, second: 0 }, "month"),
  );
}

function onClear() {
  emit("update:modelValue", null);
}
</script>

<template>
  <VMenu v-model="menuOpen" :close-on-content-click="false">
    <template #activator="{ props: menuProps }">
      <VTextField
        :model-value="monthDisplay"
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
      <VMonthPicker
        v-if="menuOpen"
        :model-value="pickerModel"
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
