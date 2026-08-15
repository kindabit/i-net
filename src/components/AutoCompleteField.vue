<!--
  自动完成输入框组件。

  在 VTextField 基础上提供可过滤的下拉候选列表，支持键盘导航与选择。
-->
<script setup lang="ts">
import { ref, computed, nextTick } from "vue";
import { t } from "@/i18n";

// ---- props / emits ----

const props = withDefaults(defineProps<{
  /** 当前输入值 */
  modelValue: string;
  /** 候选条目列表 */
  items: { title: string; subtitle?: string; value: string }[];
  /** 输入框标签 */
  label?: string;
  /** 错误提示文本 */
  errorMessages?: string | string[];
  /** 输入框密度 */
  density?: "default" | "comfortable" | "compact";
  /** 输入框变体样式 */
  variant?: "outlined" | "filled" | "solo" | "plain" | "underlined";
}>(), {
  label: "",
});

const emit = defineEmits<{
  /** 输入值变化 */
  "update:modelValue": [value: string];
  /** 用户按下 Enter 提交 */
  submit: [];
}>();

// ---- state ----

/** 下拉框交互阶段 */
type Phase = "idle" | "input-dropdown" | "focus-only";

const phase = ref<Phase>("idle");
/** 下拉框是否打开 */
const open = computed(() => phase.value === "input-dropdown");
/** 当前高亮项索引 */
const highlightedIndex = ref(0);

const inputRef = ref<{ focus: () => void; blur: () => void }>();
const dropdownRef = ref<{ $el: HTMLElement }>();

// ---- computed ----

/** 根据输入过滤后的候选列表 */
const filtered = computed(() => {
  const q = props.modelValue.toLowerCase();
  if (!q) return props.items;
  return props.items.filter((item) => item.title.toLowerCase().includes(q));
});

/** 当前高亮的候选项 */
const highlightedItem = computed(() => filtered.value[highlightedIndex.value] ?? null);

// ---- helpers ----

/** 打开下拉框 */
function openDropdown() {
  highlightedIndex.value = 0;
  phase.value = "input-dropdown";
}

/** 关闭下拉框但保持焦点 */
function closeDropdown() {
  phase.value = "focus-only";
}

/**
 * 选中指定项并更新输入值。
 * @param item 候选项
 */
function selectItem(item: { value: string }) {
  emit("update:modelValue", item.value);
  closeDropdown();
}

/** 重置为空闲状态 */
function resetToIdle() {
  phase.value = "idle";
  highlightedIndex.value = 0;
}

// ---- events ----

/** 聚焦时打开下拉框 */
function onFocus() {
  if (phase.value === "idle") {
    openDropdown();
  }
}

/** 失焦时回到空闲状态 */
function onBlur() {
  // 失焦 → idle
  resetToIdle();
}

/** 输入时保持下拉框打开并重置高亮 */
function onInput() {
  if (phase.value === "focus-only") {
    // 重新输入 → 打开下拉框
    openDropdown();
  }
  // 已在 input-dropdown 状态，highlight 复位到第一项
  highlightedIndex.value = 0;
}

/**
 * 将当前高亮项滚动到可视区域。
 * @returns 无返回值
 */
async function scrollToHighlighted() {
  await nextTick();
  const dropdownEl = dropdownRef.value?.$el;
  if (!dropdownEl) return;
  const highlightedEl = dropdownEl.querySelector(".auto-complete-highlighted") as HTMLElement | null;
  highlightedEl?.scrollIntoView({ block: "nearest" });
}

/**
 * 处理键盘导航事件。
 * @param e 键盘事件
 */
function onKeydown(e: KeyboardEvent) {
  if (phase.value !== "input-dropdown") {
    if (e.key === "Enter") {
      e.preventDefault();
      emit("submit");
    }
    return;
  }

  switch (e.key) {
    case "ArrowDown":
      e.preventDefault();
      if (filtered.value.length > 0) {
        highlightedIndex.value = (highlightedIndex.value + 1) % filtered.value.length;
        void scrollToHighlighted();
      }
      break;
    case "ArrowUp":
      e.preventDefault();
      if (filtered.value.length > 0) {
        highlightedIndex.value = (highlightedIndex.value - 1 + filtered.value.length) % filtered.value.length;
        void scrollToHighlighted();
      }
      break;
    case "Enter":
      e.preventDefault();
      if (highlightedItem.value) {
        selectItem(highlightedItem.value);
      } else {
        // 无匹配项 → 关闭下拉，保持当前输入
        closeDropdown();
      }
      break;
    case "Escape":
      e.preventDefault();
      closeDropdown();
      break;
  }
}

/**
 * 点击候选项时触发选择。
 * @param item 候选项
 */
function onClickItem(item: { value: string }) {
  selectItem(item);
}

// ---- expose ----

/** 聚焦输入框 */
function focus() {
  inputRef.value?.focus();
}

/** 失焦输入框 */
function blur() {
  inputRef.value?.blur();
}

defineExpose({ focus, blur });
</script>

<template>
  <div class="auto-complete-root">
    <VTextField
      ref="inputRef"
      :model-value="modelValue"
      :label="label"
      :error-messages="errorMessages"
      :variant="variant"
      :density="density"
      hide-details="auto"
      @update:model-value="emit('update:modelValue', $event)"
      @focus="onFocus"
      @blur="onBlur"
      @input="onInput"
      @keydown="onKeydown"
    />
    <Transition name="dropdown">
      <VCard
        ref="dropdownRef"
        v-show="open"
        class="auto-complete-dropdown"
        elevation="8"
        rounded="lg"
      >
        <VList v-if="filtered.length > 0" density="compact">
          <VListItem
            v-for="(item, i) in filtered"
            :key="item.value"
            :class="{ 'auto-complete-highlighted': i === highlightedIndex }"
            :title="item.title"
            :subtitle="item.subtitle"
            :active="i === highlightedIndex"
            @mousedown.prevent="onClickItem(item)"
          />
        </VList>
        <div v-else class="text-body-2 text-disabled pa-3 text-center">
          {{ t("common.auto-complete.no-matches") }}
        </div>
      </VCard>
    </Transition>
  </div>
</template>

<style lang="scss" scoped>
.auto-complete-root {
  position: relative;
}

.auto-complete-dropdown {
  position: absolute;
  top: 100%;
  left: 0;
  right: 0;
  z-index: 200;
  max-height: 15rem;
  overflow-y: auto;
  transform-origin: top center;
}

.dropdown-enter-active,
.dropdown-leave-active {
  transition: opacity 0.2s ease, transform 0.2s ease;
}

.dropdown-enter-from,
.dropdown-leave-to {
  opacity: 0;
  transform: scaleY(0.8) translateY(-0.5rem);
}
</style>
