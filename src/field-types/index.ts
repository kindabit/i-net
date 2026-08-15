/**
 * 字段类型注册表，提供字段类型定义的查询 API。
 *
 * 字段类型的唯一事实来源是 schemas/field-types.json（前后端共享）。
 */
import { DateTime } from "luxon";
import rawSchema from "../../schemas/field-types.json";
import type { FieldValue } from "@/api-types";
import type { FieldTypeDef, FieldTypeSchema } from "./schema-types";

const schema = rawSchema as FieldTypeSchema;

/**
 * 获取新建字段行时默认选用的字段类型 key。
 * @returns schema 中声明的默认字段类型 key
 */
export function defaultFieldType(): string {
  return schema.defaultFieldType;
}

/**
 * 按 key 查询字段类型定义。
 * @param key 字段类型的 key
 * @returns 字段类型定义，不存在时返回 undefined
 */
export function getFieldTypeDef(key: string): FieldTypeDef | undefined {
  return schema.fieldTypes.find((ft) => ft.key === key);
}

/**
 * 按字段类型 key 查询对应的底层数据类型（valueKind）。
 * @param key 字段类型的 key
 * @returns 底层数据类型 key，类型不存在时返回 undefined
 */
export function valueKindOf(key: string): string | undefined {
  return getFieldTypeDef(key)?.valueKind;
}

/**
 * 为指定字段类型创建空的（无值）FieldValue 变体。
 * @param fieldTypeKey 字段类型的 key
 * @returns 对应的无值 FieldValue
 */
export function createEmptyValue(fieldTypeKey: string): FieldValue {
  const def = getFieldTypeDef(fieldTypeKey);
  if (!def) {
    throw new Error(`Unknown field type: ${fieldTypeKey}`);
  }
  switch (def.valueKind) {
    case "string":
      return { variant: "string", data: null };
    case "decimal":
      return { variant: "decimal", data: null };
    case "instant":
      return { variant: "instant", data: null };
    case "instantRange":
      return { variant: "instantRange", data: null };
    default:
      throw new Error(
        `Unknown value kind "${def.valueKind}" for field type: ${fieldTypeKey}`,
      );
  }
}

/**
 * 返回指定字段类型的 typeConfig 中各项配置的默认值。
 * @param fieldTypeKey 字段类型的 key
 * @returns 配置默认值组成的对象，无 typeConfig 声明时返回 null
 */
export function defaultTypeConfig(
  fieldTypeKey: string,
): Record<string, string> | null {
  const def = getFieldTypeDef(fieldTypeKey);
  if (!def || !def.typeConfig) return null;
  const config: Record<string, string> = {};
  if (def.typeConfig.precision) {
    config.precision = def.typeConfig.precision.default;
  }
  return Object.keys(config).length > 0 ? config : null;
}

/**
 * 对字段值执行前端即时校验。
 * @param fieldTypeKey 字段类型的 key
 * @param value 字段值
 * @returns 校验不通过时返回错误提示的 i18n key，通过或无需校验时返回 null
 */
export function validateValue(
  fieldTypeKey: string,
  value: FieldValue,
): string | null {
  const def = getFieldTypeDef(fieldTypeKey);
  if (!def) return null;
  if (value.data === null) return null;
  if (def.validation) {
    if (
      (value.variant === "string" || value.variant === "decimal") &&
      typeof value.data === "string"
    ) {
      if (!new RegExp(def.validation.regex).test(value.data)) {
        return def.validation.errorI18nKey;
      }
    }
  }
  if (
    value.variant === "instantRange" &&
    Array.isArray(value.data) &&
    value.data.length === 2
  ) {
    const [start, end] = value.data;
    if (typeof start === "number" && typeof end === "number" && start > end) {
      return "invalid-range";
    }
  }
  return null;
}

/**
 * 将字段类型按底层数据类型（valueKind）组织为分组，供类型选择下拉分组展示。
 * @returns 分组列表，每项包含 valueKind key 及其下的字段类型；
 * 分组顺序与组内顺序均保持 schema 中的声明顺序
 */
export function fieldTypeGroups(): {
  valueKind: string;
  types: FieldTypeDef[];
}[] {
  return schema.valueKinds.map((vk) => ({
    valueKind: vk.key,
    types: schema.fieldTypes.filter((ft) => ft.valueKind === vk.key),
  }));
}

/**
 * 将字段值格式化为用于展示的文本（日志等只读场景）。
 * 时间点/区间按 UTC 格式化为 "yyyy-MM-dd HH:mm:ss"，字符串与数字原样返回。
 */
export function formatValueForDisplay(value: FieldValue): string {
  if (value.data === null) return "";
  switch (value.variant) {
    case "string":
    case "decimal":
      return value.data;
    case "instant": {
      const ts = value.data as number;
      return DateTime.fromMillis(ts, { zone: "utc" }).toFormat(
        "yyyy-MM-dd HH:mm:ss",
      );
    }
    case "instantRange": {
      const [start, end] = value.data as [number, number];
      const s = DateTime.fromMillis(start, { zone: "utc" }).toFormat(
        "yyyy-MM-dd HH:mm:ss",
      );
      const e = DateTime.fromMillis(end, { zone: "utc" }).toFormat(
        "yyyy-MM-dd HH:mm:ss",
      );
      return `${s} ~ ${e}`;
    }
  }
}
