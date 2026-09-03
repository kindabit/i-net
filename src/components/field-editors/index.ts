/**
 * 字段值编辑器组件库：维护字段类型到具体编辑器组件的映射。
 */
import type { Component } from "vue";
import StringSingleLineEditor from "./StringSingleLineEditor.vue";
import StringEmailEditor from "./StringEmailEditor.vue";
import StringUrlEditor from "./StringUrlEditor.vue";
import StringMultipleLineEditor from "./StringMultipleLineEditor.vue";
import StringPasswordEditor from "./StringPasswordEditor.vue";
import StringSecretEditor from "./StringSecretEditor.vue";
import DecimalDecimalEditor from "./DecimalDecimalEditor.vue";
import InstantInstantEditor from "./InstantInstantEditor.vue";
import InstantRangeInstantRangeEditor from "./InstantRangeInstantRangeEditor.vue";

/** 字段类型 key → 编辑器组件的映射表。 */
const EDITOR_MAP: Record<string, Component> = {
  "string:single-line": StringSingleLineEditor,
  "string:email": StringEmailEditor,
  "string:url": StringUrlEditor,
  "string:multiple-line": StringMultipleLineEditor,
  "string:password": StringPasswordEditor,
  "string:secret": StringSecretEditor,
  "decimal:decimal": DecimalDecimalEditor,
  "instant:instant": InstantInstantEditor,
  "instant-range:instant-range": InstantRangeInstantRangeEditor,
};

/**
 * 按字段类型 key 解析对应的值编辑器组件。
 * @param fieldType 字段类型的 key
 * @returns 编辑器组件；无对应编辑器时返回 undefined
 */
export function fieldEditorComponent(fieldType: string): Component | undefined {
  return EDITOR_MAP[fieldType];
}
