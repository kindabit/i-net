/**
 * 字段行校验错误契约。
 *
 * 描述单行字段的校验错误：错误信息与需要高亮的输入部位。
 * 错误表由字段列表编辑逻辑（use-node-field-list / use-template-field-list）的
 * validate 一次性完整替换，行组件按行 uid 自取属于自己的那条错误。
 */

/** 字段行校验错误。 */
export interface FieldError {
  /** 错误信息（已国际化）。 */
  msg: string;
  /** 需要高亮的部位：name 为字段名输入框，value 为字段值编辑器。 */
  highlight: "name" | "value";
}
