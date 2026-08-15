/** 底层数据类型定义。 */
export interface ValueKindDef {
  key: string;
}

/** 校验规则定义。 */
export interface ValidationDef {
  regex: string;
  errorI18nKey: string;
}

/** 精度配置定义。 */
export interface PrecisionConfigDef {
  options: string[];
  default: string;
}

/** 类型配置定义。 */
export interface TypeConfigDef {
  precision?: PrecisionConfigDef;
}

/** 字段类型定义（顶层业务类型，纯运行时数据）。 */
export interface FieldTypeDef {
  key: string;
  valueKind: string;
  i18nKey: string;
  editor: string;
  masked: boolean;
  passwordGenerator: boolean;
  supportsDictionary: boolean;
  multiRow: boolean;
  validation: ValidationDef | null;
  typeConfig: TypeConfigDef | null;
}

/** 字段类型 schema 的顶层结构。 */
export interface FieldTypeSchema {
  version: number;
  /** 新建字段行时默认选用的字段类型 key。 */
  defaultFieldType: string;
  valueKinds: ValueKindDef[];
  fieldTypes: FieldTypeDef[];
}
