<!--
  字段值编辑器适配层。

  按字段类型与类型配置，从值编辑器组件库解析并渲染具体的值编辑器，
  封装 FieldValue 与编辑器原始值之间的解包/包装、字典候选值的收集，
  使上层组件无需接触具体的值编辑器组件。
-->
<script setup lang="ts">
import { computed } from "vue";
import type { FieldValue } from "@/api-types";
import { getFieldTypeDef, valueKindOf } from "@/field-types";
import { getDictionaryDirectChildren } from "@/dictionary";
import { fieldEditorComponent } from "./index";
import { useClipboardClear } from "@/composables/use-clipboard-clear";
import { snackbarText } from "@/composables/use-snackbar";

const props = defineProps<{
  fieldType: string;
  typeConfig: Record<string, unknown> | null;
  dictionaryId: string | null;
  /** 字段值（v-model 绑定，FieldValue 包装形态）。 */
  modelValue: FieldValue | null;
}>();

const emit = defineEmits<{
  "update:modelValue": [value: FieldValue | null];
}>();

/** 按字段类型与类型配置解析出的值编辑器组件；无对应编辑器时为 undefined。 */
const editorComponent = computed(() =>
  fieldEditorComponent(props.fieldType, props.typeConfig),
);

/** 将 FieldValue 解包为编辑器所需的原始值。 */
const editorModelValue = computed(() => props.modelValue?.data ?? null);

/** 字段类型支持字典绑定且已绑定字典时，获取绑定节点的直接子节点 value 作为候选值；否则为 undefined。 */
const editorDictionaryItems = computed(() => {
  if (!getFieldTypeDef(props.fieldType)?.supportsDictionary)
    return undefined;
  if (!props.dictionaryId) return undefined;
  return getDictionaryDirectChildren(props.dictionaryId);
});

/**
 * 将编辑器输出的原始值按字段类型的底层数据类型包装为 FieldValue 并 emit。
 * @param data 编辑器输出的原始值
 */
function onEditorUpdate(data: string | number | [number, number] | null): void {
  const kind = valueKindOf(props.fieldType);
  if (kind === "string")
    emit("update:modelValue", {
      variant: "string",
      data: data as string | null,
    });
  else if (kind === "decimal")
    emit("update:modelValue", {
      variant: "decimal",
      data: data as string | null,
    });
  else if (kind === "instant")
    emit("update:modelValue", {
      variant: "instant",
      data: data as number | null,
    });
  else if (kind === "instantRange")
    emit("update:modelValue", {
      variant: "instantRange",
      data: data as [number, number] | null,
    });
}

/**
 * 复制字段值到剪贴板，成功后提示用户并启动剪贴板清空倒计时。
 */
async function copyFieldValue(): Promise<void> {
  const value = props.modelValue?.data;
  if (value == null) return;
  try {
    await navigator.clipboard.writeText(String(value));
    snackbarText("字段值已复制到剪贴板", "success");
    useClipboardClear().startCountdown(String(value));
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
      :model-value="editorModelValue"
      :dictionary-items="editorDictionaryItems"
      @update:model-value="onEditorUpdate"
    />
    <VBtn
      icon="mdi-content-copy"
      variant="text"
      density="compact"
      :disabled="props.modelValue?.data == null"
      @click="copyFieldValue"
    />
  </div>
</template>
