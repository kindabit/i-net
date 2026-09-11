<!--
  string:url 字段值编辑器。

  普通单行输入框；网址格式校验由字段类型目录在保存时执行。
  当字段类型定义携带 opener 时，输入框右侧显示快捷打开按钮，调用系统默认浏览器或邮件应用。
-->
<script setup lang="ts">
import { ref, watch } from "vue";
import { t } from "@/i18n";
import type { FieldTypeDef } from "@/field-types";

const props = defineProps<{
  modelValue: string | null;
  /** 值错误高亮：输入控件进入错误高亮状态（不显示错误信息）。 */
  errorHighlight?: boolean;
  readonly?: boolean;
  /** 字段类型定义；含 opener 时显示快捷打开按钮。 */
  fieldTypeDef?: FieldTypeDef;
}>();

const emit = defineEmits<{
  "update:modelValue": [value: string | null];
}>();

const text = ref<string>(props.modelValue ?? "");

watch(
  () => props.modelValue,
  (v) => {
    text.value = v ?? "";
  }
);

function onInput() {
  emit("update:modelValue", text.value === "" ? null : text.value);
}
</script>

<template>
  <div class="editor-root">
    <VTextField
      v-model="text"
      class="editor-input"
      :error="errorHighlight"
      :readonly="readonly"
      variant="outlined"
      density="compact"
      hide-details="auto"
      @update:model-value="onInput()"
    />
    <VBtn
      v-if="fieldTypeDef?.opener"
      icon="mdi-open-in-new"
      variant="text"
      density="compact"
      :disabled="!text"
      :title="t('database.field-editor.open-value')"
      tabindex="-1"
      @click="fieldTypeDef?.opener?.(text)"
    />
  </div>
</template>

<style lang="scss" scoped>
/* 根容器继承 FieldValueEditor 下发的 flex:1 与 min-width:0，内部输入框占满打开按钮之外的剩余宽度。 */
.editor-root {
  display: flex;
  align-items: center;
  gap: 0.25rem;
  flex: 1;
  min-width: 0;
}

.editor-input {
  flex: 1;
  min-width: 0;
}
</style>
