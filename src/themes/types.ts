/**
 * 主题类型定义与校验。
 *
 * AppThemeDefinition 与 Vuetify 4 的 ThemeDefinition（DeepPartial）结构兼容，
 * 可直接写入 vuetify.theme.themes；name、displayName 等额外字段会被 Vuetify
 * 忽略，但随主题数据一同分享、存储。
 *
 * 数据结构校验基于 JSON Schema，由 Ajv 编译生成（用于导入与持久化读取）。
 */
import Ajv from "ajv";

/** 应用主题定义（即分享与持久化的数据单元） */
export interface AppThemeDefinition {
  /** 主题唯一名称（同时作为 Vuetify 主题名） */
  name: string;
  /** 展示名称（用于主题切换器显示） */
  displayName: string;
  /** 是否暗色基调 */
  dark: boolean;
  /** 颜色表（键为 Vuetify 颜色名，值为 CSS 颜色） */
  colors: Record<string, string>;
  /** 额外主题变量（对应 Vuetify ThemeDefinition.variables） */
  variables?: Record<string, string | number>;
}

/** AppThemeDefinition 的 JSON Schema */
const schema = {
  type: "object",
  required: ["name", "displayName", "dark", "colors"],
  properties: {
    name: { type: "string", minLength: 1 },
    displayName: { type: "string", minLength: 1 },
    dark: { type: "boolean" },
    colors: {
      type: "object",
      required: ["background", "surface", "primary"],
      properties: {
        background: { type: "string" },
        surface: { type: "string" },
        primary: { type: "string" },
      },
      // 其余颜色键的值也必须是字符串
      additionalProperties: { type: "string" },
    },
    variables: {
      type: "object",
      additionalProperties: { type: ["string", "number"] },
    },
  },
} as const;

const ajv = new Ajv({ allowUnionTypes: true });

const validate = ajv.compile<AppThemeDefinition>(schema);

/**
 * 校验未知数据是否为合法的主题定义（Ajv 由 JSON Schema 编译生成）。
 * @param data 待校验数据
 * @returns 是否合法
 */
export function isAppThemeDefinition(
  data: unknown,
): data is AppThemeDefinition {
  return validate(data);
}

/**
 * 校验失败后的可读错误描述（在 isAppThemeDefinition 返回 false 后调用）。
 * @returns Ajv 错误文本
 */
export function themeValidationErrors(): string {
  return ajv.errorsText(validate.errors);
}
