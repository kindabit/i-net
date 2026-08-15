import type { Component } from "vue";
import TextSingleLine from "./text/TextSingleLineEditor.vue";
import Email from "./text/EmailEditor.vue";
import Url from "./text/UrlEditor.vue";
import TextMultiLine from "./text/TextMultiLineEditor.vue";
import Password from "./text/PasswordEditor.vue";
import SecretSingleLine from "./text/SecretSingleLineEditor.vue";
import NumberEditor from "./text/NumberEditor.vue";
import DateYearEditor from "./date/DateYearEditor.vue";
import DateMonthEditor from "./date/DateMonthEditor.vue";
import DateDayEditor from "./date/DateDayEditor.vue";
import DateHourEditor from "./date/DateHourEditor.vue";
import DateMinuteEditor from "./date/DateMinuteEditor.vue";
import DateSecondEditor from "./date/DateSecondEditor.vue";
import DateRangeYearEditor from "./date-range/DateRangeYearEditor.vue";
import DateRangeMonthEditor from "./date-range/DateRangeMonthEditor.vue";
import DateRangeDayEditor from "./date-range/DateRangeDayEditor.vue";
import DateRangeHourEditor from "./date-range/DateRangeHourEditor.vue";
import DateRangeMinuteEditor from "./date-range/DateRangeMinuteEditor.vue";
import DateRangeSecondEditor from "./date-range/DateRangeSecondEditor.vue";

const TEXT_EDITORS: Record<string, Component> = {
  TextSingleLine, Email, Url, TextMultiLine, Password, SecretSingleLine, Number: NumberEditor,
};
const DATE_EDITORS: Record<string, Component> = {
  year: DateYearEditor, month: DateMonthEditor, day: DateDayEditor,
  hour: DateHourEditor, minute: DateMinuteEditor, second: DateSecondEditor,
};
const DATE_RANGE_EDITORS: Record<string, Component> = {
  year: DateRangeYearEditor, month: DateRangeMonthEditor, day: DateRangeDayEditor,
  hour: DateRangeHourEditor, minute: DateRangeMinuteEditor, second: DateRangeSecondEditor,
};

/**
 * 按字段类型与类型配置解析值编辑器组件。
 * @param fieldType 字段类型 key
 * @param typeConfig 字段类型配置（Date/DateRange 读取 precision，缺省 "day"）
 */
export function fieldEditorComponent(
  fieldType: string,
  typeConfig: Record<string, unknown> | null,
): Component | undefined {
  const text = TEXT_EDITORS[fieldType];
  if (text) return text;
  const precision = (typeConfig?.precision as string) || "day";
  if (fieldType === "Date") return DATE_EDITORS[precision];
  if (fieldType === "DateRange") return DATE_RANGE_EDITORS[precision];
  return undefined;
}
