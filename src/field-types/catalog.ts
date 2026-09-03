/**
 * 字段类型目录：字段类型元数据的唯一事实来源。
 *
 * 字段类型系统历史上由前后端共享的 schemas/field-types.json 定义；重构后后端对
 * 字段类型与字段值完全无感（只负责读写），类型元数据内化为本模块的常量。
 * 字段类型到编辑器组件的映射由 components/field-editors/index.ts 维护。
 */
import { isValidInstantValue, validateRangeValue } from "./date-time";
import type { Precision } from "./date-time";
import { t } from "@/i18n";

/** 底层数据类型，与字段类型 key 的冒号前缀一致。 */
export type ValueKind = "string" | "decimal" | "instant" | "instant-range";

/** 字段类型定义。 */
export interface FieldTypeDef {
  /** 字段类型的唯一标识，格式为 "valueKind:subtype"。 */
  key: string;
  /** 底层数据类型。 */
  valueKind: ValueKind;
  /** 类型显示名的 i18n key 后缀（配合 database.field-type. 前缀使用）。 */
  i18nKey: string;
  /** 是否在日志等展示场景掩码显示。 */
  masked: boolean;
  /** 是否在编辑器中提供密码生成器入口。 */
  passwordGenerator: boolean;
  /** 是否支持绑定字典。 */
  supportsDictionary: boolean;
  /**
   * 前端校验函数。
   * @param value 字段值字符串（非空；null 与空串在调用前已被跳过）
   * @returns 校验不通过时返回错误提示的 i18n key 后缀（配合 database.field-type. 前缀使用），通过时返回 null；一个类型可返回多种不同的错误 key
   */
  validator: (value: string) => string | null;
}

/**
 * 无需校验的字段类型共用的预定义校验函数。
 * @param value 字段值字符串（忽略）
 * @returns 恒为 null（无错误）
 */
function noValidation(_value: string): string | null {
  return null;
}

/** 全部字段类型定义，顺序即类型选择器中的展示顺序。 */
export const FIELD_TYPES: readonly FieldTypeDef[] = [
  {
    key: "string:single-line",
    valueKind: "string",
    i18nKey: "string-single-line",
    masked: false,
    passwordGenerator: false,
    supportsDictionary: true,
    validator: noValidation,
  },
  {
    key: "string:email",
    valueKind: "string",
    i18nKey: "string-email",
    masked: false,
    passwordGenerator: false,
    supportsDictionary: false,
    validator: (value: string): string | null =>
      /^[^@\s]+@[^@\s]+\.[^@\s]+$/.test(value) ? null : "invalid-email",
  },
  {
    key: "string:url",
    valueKind: "string",
    i18nKey: "string-url",
    masked: false,
    passwordGenerator: false,
    supportsDictionary: false,
    validator: (value: string): string | null =>
      /^https?:\/\/\S+$/.test(value) ? null : "invalid-url",
  },
  {
    key: "string:multiple-line",
    valueKind: "string",
    i18nKey: "string-multiple-line",
    masked: false,
    passwordGenerator: false,
    supportsDictionary: false,
    validator: noValidation,
  },
  {
    key: "string:password",
    valueKind: "string",
    i18nKey: "string-password",
    masked: true,
    passwordGenerator: true,
    supportsDictionary: false,
    validator: noValidation,
  },
  {
    key: "string:secret",
    valueKind: "string",
    i18nKey: "string-secret",
    masked: true,
    passwordGenerator: false,
    supportsDictionary: false,
    validator: noValidation,
  },
  {
    key: "decimal:decimal",
    valueKind: "decimal",
    i18nKey: "decimal-decimal",
    masked: false,
    passwordGenerator: false,
    supportsDictionary: false,
    validator: (value: string): string | null =>
      /^[+-]?(\d+(\.\d*)?|\.\d+)([eE][+-]?\d+)?$/.test(value) ? null : "invalid-number",
  },
  {
    key: "instant:instant",
    valueKind: "instant",
    i18nKey: "instant-instant",
    masked: false,
    passwordGenerator: false,
    supportsDictionary: false,
    validator: (value: string): string | null =>
      isValidInstantValue(value) ? null : "invalid-instant",
  },
  {
    key: "instant-range:instant-range",
    valueKind: "instant-range",
    i18nKey: "instant-range-instant-range",
    masked: false,
    passwordGenerator: false,
    supportsDictionary: false,
    validator: (value: string): string | null => {
      const error = validateRangeValue(value);
      // 区分格式非法与顺序错误两种校验失败，分别对应不同的错误提示。
      if (error === "format") return "invalid-range";
      if (error === "order") return "invalid-range-order";
      return null;
    },
  },
];

/** 新建字段行时默认选用的字段类型 key。 */
export const DEFAULT_FIELD_TYPE = "string:single-line";

/**
 * 按 key 查询字段类型定义。
 * @param key 字段类型的 key
 * @returns 字段类型定义，不存在时返回 undefined
 */
export function getFieldTypeDef(key: string): FieldTypeDef | undefined {
  return FIELD_TYPES.find((ft) => ft.key === key);
}

/**
 * 按字段类型 key 查询对应的底层数据类型。
 * @param key 字段类型的 key
 * @returns 底层数据类型，类型不存在时返回 undefined
 */
export function valueKindOf(key: string): ValueKind | undefined {
  return getFieldTypeDef(key)?.valueKind;
}

/** instant 系列字段的全部显示精度选项。 */
export const PRECISIONS: readonly Precision[] = [
  "year",
  "month",
  "day",
  "hour",
  "minute",
  "second",
  "millisecond",
];

/** instant 系列字段的默认显示精度。 */
export const DEFAULT_PRECISION: Precision = "day";

/**
 * 对字段值执行前端即时校验。
 * @param fieldType 字段类型的 key
 * @param value 字段值字符串；null 或空串视为无值，跳过校验
 * @returns 校验不通过时返回错误提示的 i18n key 后缀（配合 database.field-type. 前缀使用），通过或类型不存在时返回 null
 */
export function validateValue(fieldType: string, value: string | null): string | null {
  const def = getFieldTypeDef(fieldType);
  if (!def) return null;
  if (value === null || value === "") return null;
  return def.validator(value);
}

/**
 * 获取字段类型的国际化显示名称。
 * @param key 字段类型的 key
 * @returns 国际化后的类型名称；类型不存在时原样返回 key
 */
export function fieldTypeDisplayName(key: string): string {
  const def = getFieldTypeDef(key);
  return def ? t(`database.field-type.${def.i18nKey}`) : key;
}

/**
 * 查询字段类型是否在日志等展示场景掩码显示。
 * @param key 字段类型的 key
 * @returns 掩码显示时返回 true；类型不存在时返回 false
 */
export function isFieldTypeMasked(key: string): boolean {
  return getFieldTypeDef(key)?.masked ?? false;
}
