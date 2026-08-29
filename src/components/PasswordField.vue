<!--
  密码输入框组件。

  在 VTextField 基础上内置密码可见性切换（眼睛图标），图标常驻、不受焦点影响；
  通过 append-inner 插槽允许消费方在眼睛图标之后追加额外图标（如密码生成器入口）。
  WebView2 原生的密码显示按钮由全局样式隐藏（styles/global.scss），避免双图标。
-->
<script setup lang="ts">
import { computed, ref } from "vue";

// ---- props / emits ----

withDefaults(
  defineProps<{
    /** 当前输入值 */
    modelValue: string;
    /** 输入框标签 */
    label?: string;
    /** 错误提示文本 */
    errorMessages?: string | string[];
    /** 输入框密度 */
    density?: "default" | "comfortable" | "compact";
    /** 输入框变体样式 */
    variant?: "outlined" | "filled" | "solo" | "plain" | "underlined";
    /** 是否只读 */
    readonly?: boolean;
  }>(),
  {
    label: "",
    variant: "outlined",
  },
);

const emit = defineEmits<{
  /** 输入值变化 */
  "update:modelValue": [value: string];
}>();

// ---- state ----

/** 密码是否以明文显示 */
const visible = ref(false);

const inputRef = ref<{ focus: () => void }>();

// ---- computed ----

/** 输入框类型（明文/密文） */
const inputType = computed<string>(() =>
  visible.value ? "text" : "password",
);

/** 可见性切换图标 */
const eyeIcon = computed<string>(() =>
  visible.value ? "mdi-eye-off" : "mdi-eye",
);

// ---- events ----

/** 切换密码可见性 */
function toggleVisible() {
  visible.value = !visible.value;
}

// ---- expose ----

/** 聚焦输入框 */
function focus() {
  inputRef.value?.focus();
}

defineExpose({ focus });
</script>

<template>
  <!-- 未声明的 attrs（class、@keydown 等）自动透传到根节点 VTextField -->
  <VTextField
    ref="inputRef"
    :model-value="modelValue"
    :type="inputType"
    :label="label"
    :error-messages="errorMessages"
    :variant="variant"
    :density="density"
    :readonly="readonly"
    hide-details="auto"
    @update:model-value="emit('update:modelValue', $event)"
  >
    <template #append-inner>
      <VIcon
        :icon="eyeIcon"
        class="password-eye"
        tabindex="-1"
        @click="toggleVisible"
      />
      <slot name="append-inner" />
    </template>
  </VTextField>
</template>

<style lang="scss" scoped>
.password-eye {
  cursor: pointer;
}
</style>
