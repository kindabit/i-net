use std::collections::HashSet;
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

/// 字段类型 schema 的顶层结构。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldTypeSchema {
    /// schema 版本号。
    pub version: i64,
    /// 新建字段行时默认选用的字段类型 key。
    pub default_field_type: String,
    /// 底层数据类型列表。
    pub value_kinds: Vec<ValueKindDef>,
    /// 字段类型定义列表。
    pub field_types: Vec<FieldTypeDef>,
}

/// 底层数据类型定义。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValueKindDef {
    /// 底层数据类型的唯一标识。
    pub key: String,
}

/// 字段类型定义。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldTypeDef {
    /// 字段类型的唯一标识。
    pub key: String,
    /// 该字段类型对应的底层数据类型 key。
    pub value_kind: String,
    /// 前端国际化 key。
    pub i18n_key: String,
    /// 编辑器类型标识。
    pub editor: String,
    /// 是否掩码显示。
    pub masked: bool,
    /// 是否支持密码生成器。
    pub password_generator: bool,
    /// 是否支持字典/自动完成。
    pub supports_dictionary: bool,
    /// 该字段类型在字段编辑卡片中是否以多行展示。
    pub multi_row: bool,
    /// 校验规则定义，None 表示无内置校验。
    pub validation: Option<ValidationDef>,
    /// 类型配置定义，None 表示该类型无额外配置项。
    pub type_config: Option<TypeConfigDef>,
}

/// 校验规则定义。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationDef {
    /// 正则表达式字符串。
    pub regex: String,
    /// 校验失败时的前端国际化 key。
    pub error_i18n_key: String,
}

/// 类型配置定义。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TypeConfigDef {
    /// 精度配置，None 表示该类型不涉及精度设置。
    pub precision: Option<PrecisionConfigDef>,
}

/// 精度配置定义。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrecisionConfigDef {
    /// 可选的精度选项列表。
    pub options: Vec<String>,
    /// 默认精度值。
    pub default: String,
}

/// 内嵌的字段类型 schema JSON 文本。
const SCHEMA_JSON: &str = include_str!("../../../../../schemas/field-types.json");

/// 全局 schema 缓存。
static SCHEMA: OnceLock<FieldTypeSchema> = OnceLock::new();

/// 获取字段类型 schema 的单例引用。
///
/// 首次调用时解析 JSON 并进行自洽校验，任何失败均直接 panic。
pub fn schema() -> &'static FieldTypeSchema {
    SCHEMA.get_or_init(|| {
        parse_schema(SCHEMA_JSON).unwrap_or_else(|reason| {
            panic!("Field type schema is invalid: {reason}")
        })
    })
}

/// 解析 JSON 字符串为 FieldTypeSchema，并执行自洽校验。
fn parse_schema(json: &str) -> Result<FieldTypeSchema, String> {
    let schema: FieldTypeSchema =
        serde_json::from_str(json).map_err(|e| format!("Failed to parse JSON: {e}"))?;
    validate(&schema)?;
    Ok(schema)
}

/// 对已解析的 schema 执行自洽校验，返回 Ok 或描述问题的字符串。
fn validate(schema: &FieldTypeSchema) -> Result<(), String> {
    let mut seen_vk = HashSet::new();
    for vk in &schema.value_kinds {
        if vk.key.is_empty() {
            return Err("A valueKind key is empty, which is prohibited".to_string());
        }
        if !seen_vk.insert(&vk.key) {
            return Err(format!("Duplicate valueKind key: {}", vk.key));
        }
    }

    let mut seen_ft = HashSet::new();
    for ft in &schema.field_types {
        if ft.key.is_empty() {
            return Err("A fieldType key is empty, which is prohibited".to_string());
        }
        if !seen_ft.insert(&ft.key) {
            return Err(format!("Duplicate fieldType key: {}", ft.key));
        }
    }

    for ft in &schema.field_types {
        if !seen_vk.contains(&ft.value_kind) {
            return Err(format!(
                "fieldType '{}' references unknown valueKind: '{}'",
                ft.key, ft.value_kind
            ));
        }

        if ft.i18n_key.is_empty() {
            return Err(format!("fieldType '{}' has empty i18n_key", ft.key));
        }
        if ft.editor.is_empty() {
            return Err(format!("fieldType '{}' has empty editor", ft.key));
        }

        if let Some(vd) = &ft.validation {
            regex::Regex::new(&vd.regex).map_err(|e| {
                format!(
                    "fieldType '{}' has invalid validation regex: {}",
                    ft.key, e
                )
            })?;
        }

        if let Some(tc) = &ft.type_config {
            if let Some(precision) = &tc.precision {
                if precision.options.is_empty() {
                    return Err(format!(
                        "fieldType '{}' precision options is empty",
                        ft.key
                    ));
                }
                let mut opt_set = HashSet::new();
                for opt in &precision.options {
                    if !opt_set.insert(opt) {
                        return Err(format!(
                            "fieldType '{}' has duplicate precision option: '{}'",
                            ft.key, opt
                        ));
                    }
                }
                if !opt_set.contains(&precision.default) {
                    return Err(format!(
                        "fieldType '{}' precision default '{}' is not in the options",
                        ft.key, precision.default
                    ));
                }
            }
        }
    }

    if schema.default_field_type.is_empty() {
        return Err("defaultFieldType is empty, which is prohibited".to_string());
    }
    if !seen_ft.contains(&schema.default_field_type) {
        return Err(format!(
            "defaultFieldType '{}' references unknown fieldType",
            schema.default_field_type
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 完整覆盖 schema 模块：成功路径（内嵌 JSON 可解析、内容符合预期）
    /// 与各自洽校验失败路径（parse_schema 报错）。
    #[test]
    fn schema_module_full_coverage() {
        // 成功路径：schema() 不 panic，version 为 1。
        let s = schema();
        assert_eq!(s.version, 1);

        // 成功路径：default_field_type 为 schema 中声明的默认字段类型。
        assert_eq!(s.default_field_type, "TextSingleLine");

        // 成功路径：field_types 包含全部 9 个类型定义（已移除 SecretMultiLine）。
        assert_eq!(s.field_types.len(), 9);

        // 成功路径：4 个 valueKind 均被至少一个 fieldType 引用。
        let expected: HashSet<&str> =
            ["string", "decimal", "instant", "instantRange"]
                .iter()
                .copied()
                .collect();
        let referenced: HashSet<&str> = s
            .field_types
            .iter()
            .map(|ft| ft.value_kind.as_str())
            .collect();
        assert_eq!(expected, referenced);

        // 失败路径：valueKind key 为空字符串时报错。
        let json = r#"{
            "version": 1,
            "defaultFieldType": "T1",
            "valueKinds": [{ "key": "" }],
            "fieldTypes": []
        }"#;
        assert!(parse_schema(json).is_err());

        // 失败路径：valueKind key 重复时报错。
        let json = r#"{
            "version": 1,
            "defaultFieldType": "T1",
            "valueKinds": [{ "key": "string" }, { "key": "string" }],
            "fieldTypes": []
        }"#;
        assert!(parse_schema(json).is_err());

        // 失败路径：fieldType key 重复时报错。
        let json = r#"{
            "version": 1,
            "defaultFieldType": "T1",
            "valueKinds": [{ "key": "string" }],
            "fieldTypes": [
                { "key": "T1", "valueKind": "string", "i18nKey": "t1", "editor": "e1", "masked": false, "passwordGenerator": false, "supportsDictionary": false, "multiRow": false, "validation": null, "typeConfig": null },
                { "key": "T1", "valueKind": "string", "i18nKey": "t2", "editor": "e2", "masked": false, "passwordGenerator": false, "supportsDictionary": false, "multiRow": false, "validation": null, "typeConfig": null }
            ]
        }"#;
        assert!(parse_schema(json).is_err());

        // 失败路径：fieldType 引用的 valueKind 不存在时报错。
        let json = r#"{
            "version": 1,
            "defaultFieldType": "T1",
            "valueKinds": [{ "key": "string" }],
            "fieldTypes": [
                { "key": "T1", "valueKind": "decimal", "i18nKey": "t1", "editor": "e1", "masked": false, "passwordGenerator": false, "supportsDictionary": false, "multiRow": false, "validation": null, "typeConfig": null }
            ]
        }"#;
        assert!(parse_schema(json).is_err());

        // 失败路径：validation.regex 无法被 regex crate 编译时报错。
        let json = r#"{
            "version": 1,
            "defaultFieldType": "T1",
            "valueKinds": [{ "key": "string" }],
            "fieldTypes": [
                { "key": "T1", "valueKind": "string", "i18nKey": "t1", "editor": "e1", "masked": false, "passwordGenerator": false, "supportsDictionary": false, "multiRow": false, "validation": { "regex": "[invalid", "errorI18nKey": "e" }, "typeConfig": null }
            ]
        }"#;
        assert!(parse_schema(json).is_err());

        // 失败路径：precision default 不在 options 内时报错。
        let json = r#"{
            "version": 1,
            "defaultFieldType": "T1",
            "valueKinds": [{ "key": "instant" }],
            "fieldTypes": [
                { "key": "T1", "valueKind": "instant", "i18nKey": "t1", "editor": "e1", "masked": false, "passwordGenerator": false, "supportsDictionary": false, "multiRow": false, "validation": null, "typeConfig": { "precision": { "options": ["a", "b"], "default": "c" } } }
            ]
        }"#;
        assert!(parse_schema(json).is_err());

        // 失败路径：defaultFieldType 为空字符串时报错。
        let json = r#"{
            "version": 1,
            "defaultFieldType": "",
            "valueKinds": [{ "key": "string" }],
            "fieldTypes": [
                { "key": "T1", "valueKind": "string", "i18nKey": "t1", "editor": "e1", "masked": false, "passwordGenerator": false, "supportsDictionary": false, "multiRow": false, "validation": null, "typeConfig": null }
            ]
        }"#;
        assert!(parse_schema(json).is_err());

        // 失败路径：defaultFieldType 引用的 fieldType 不存在时报错。
        let json = r#"{
            "version": 1,
            "defaultFieldType": "Nope",
            "valueKinds": [{ "key": "string" }],
            "fieldTypes": [
                { "key": "T1", "valueKind": "string", "i18nKey": "t1", "editor": "e1", "masked": false, "passwordGenerator": false, "supportsDictionary": false, "multiRow": false, "validation": null, "typeConfig": null }
            ]
        }"#;
        assert!(parse_schema(json).is_err());
    }
}
