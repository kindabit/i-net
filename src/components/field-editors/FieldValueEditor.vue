<!--
  字段值编辑器适配层。

  按字段类型从值编辑器组件库解析并渲染具体的值编辑器，封装字典候选值的收集，
  使上层组件无需接触具体的值编辑器组件。
-->
<script setup lang="ts">
import { computed } from "vue";
import { getFieldTypeDef } from "@/field-types";
import { getDictionaryDirectChildren } from "@/dictionary";
import { fieldEditorComponent } from "./index";
import { useClipboardClear } from "@/composables/use-clipboard-clear";
import { snackbarText } from "@/composables/use-snackbar";

const props = defineProps<{
  fieldType: string;
  dictionaryId: string | null;
  /** 字段值字符串（v-model 绑定），null 表示无值。 */
  modelValue: string | null;
  /** 值错误高亮：透传给具体值编辑器，使其输入控件进入错误高亮状态（不显示错误信息）。 */
  errorHighlight?: boolean;
  /** 只读模式：透传给具体值编辑器；复制按钮保持可用。 */
  readonly?: boolean;
}>();

const emit = defineEmits<{
  "update:modelValue": [value: string | null];
}>();

/** 按字段类型解析出的值编辑器组件；无对应编辑器时为 undefined。 */
const editorComponent = computed(() => fieldEditorComponent(props.fieldType));

/** 字段类型支持字典绑定且已绑定字典时，获取绑定节点的直接子节点 value 作为候选值；否则为 undefined。 */
const editorDictionaryItems = computed(() => {
  if (!getFieldTypeDef(props.fieldType)?.supportsDictionary)
    return undefined;
  if (!props.dictionaryId) return undefined;
  return getDictionaryDirectChildren(props.dictionaryId);
});

/**
 * 复制字段值到剪贴板，成功后提示用户并启动剪贴板清空倒计时。
 */
async function copyFieldValue(): Promise<void> {
  const value = props.modelValue;
  if (value == null) return;
  try {
    await navigator.clipboard.writeText(value);
    snackbarText("字段值已复制到剪贴板", "success");
    useClipboardClear().startCountdown(value);
  } catch (e) {
    snackbarText("复制失败，请手动复制", "error");
  }
}
</script>

<template>
  <div class="d-flex align-center gap-2">
    <component
      :is="editorComponent"
      v-if="editorComponent"
      class="value-editor"
      :model-value="modelValue"
      :dictionary-items="editorDictionaryItems"
      :error-highlight="errorHighlight"
      :readonly="readonly"
      @update:model-value="emit('update:modelValue', $event)"
    />
    <VBtn
      icon="mdi-content-copy"
      variant="text"
      density="compact"
      :disabled="modelValue == null"
      tabindex="-1"
      @click="copyFieldValue"
    />
  </div>
</template>

<style lang="scss" scoped>
/* 值编辑器组件占满复制按钮之外的剩余宽度（v-input 默认自带 flex 伸展，自定义布局的编辑器如 instant 系列需要显式指定）。 */
.value-editor {
  flex: 1;
  min-width: 0;
}
</style>
