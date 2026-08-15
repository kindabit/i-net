import { ref } from "vue";
import type { FieldValue, NodeFieldVO, TemplateFieldVO } from "@/api-types";
import {
  createEmptyValue,
  defaultFieldType,
  validateValue,
} from "@/field-types";
import { t } from "@/i18n";

/** 字段编辑行模型。 */
export interface FieldRow {
  /** 行的唯一标识，用作列表渲染的 key。 */
  uid: number;
  name: string;
  fieldType: string;
  typeConfig: Record<string, unknown> | null;
  /** 字段值；模板模式（withValues=false）下恒为 null。 */
  value: FieldValue | null;
  dictionaryId: string | null;
  /** 高级选项区是否展开。 */
  expanded: boolean;
}

function normalizedJson(rows: FieldRow[]): string {
  return JSON.stringify(
    rows.map((r) => ({
      name: r.name,
      fieldType: r.fieldType,
      typeConfig: r.typeConfig,
      value: r.value,
      dictionaryId: r.dictionaryId,
    })),
  );
}

/**
 * 字段列表编辑逻辑，节点字段与模板字段共用。
 * @param options.withValues 是否为值编辑模式（节点），模板模式传 false
 */
export function useFieldList(options: { withValues: boolean }) {
  let nextUid = 1;
  const rows = ref<FieldRow[]>([]);
  const errors = ref<Map<number, { name?: string; value?: string }>>(
    new Map(),
  );
  const initialSnapshot = ref("");

  /** 追加一行空字段。 */
  function addRow(): void {
    const fieldType = defaultFieldType();
    rows.value.push({
      uid: nextUid++,
      name: "",
      fieldType,
      typeConfig: null,
      value: options.withValues ? createEmptyValue(fieldType) : null,
      dictionaryId: null,
      expanded: false,
    });
  }

  /** 按 uid 删除一行。 */
  function removeRow(uid: number): void {
    rows.value = rows.value.filter((r) => r.uid !== uid);
  }

  /** 拖拽排序：把 fromUid 行移动到 toUid 行的位置。 */
  function moveRow(fromUid: number, toUid: number): void {
    const fromIdx = rows.value.findIndex((r) => r.uid === fromUid);
    const toIdx = rows.value.findIndex((r) => r.uid === toUid);
    if (fromIdx === -1 || toIdx === -1) return;
    const [removed] = rows.value.splice(fromIdx, 1);
    rows.value.splice(toIdx, 0, removed);
  }

  /** 从节点字段 VO 列表装入行并记录初始快照。 */
  function loadFromNodeFields(vos: NodeFieldVO[]): void {
    nextUid = 1;
    rows.value = vos.map((vo) => ({
      uid: nextUid++,
      name: vo.name,
      fieldType: vo.field_type,
      typeConfig: vo.type_config,
      value: vo.value,
      dictionaryId: vo.dictionary_id,
      expanded: false,
    }));
    initialSnapshot.value = normalizedJson(rows.value);
  }

  /** 从模板字段 VO 列表装入行并记录初始快照（不含值）。 */
  function loadFromTemplateFields(vos: TemplateFieldVO[]): void {
    nextUid = 1;
    rows.value = vos.map((vo) => ({
      uid: nextUid++,
      name: vo.name,
      fieldType: vo.field_type,
      typeConfig: vo.type_config,
      value: null,
      dictionaryId: vo.dictionary_id,
      expanded: false,
    }));
    initialSnapshot.value = normalizedJson(rows.value);
  }

  /** 导出为节点字段 VO 列表（value 缺省时补 createEmptyValue）。 */
  function toNodeFieldVOs(): NodeFieldVO[] {
    return rows.value.map((r) => ({
      name: r.name,
      field_type: r.fieldType,
      type_config: r.typeConfig,
      value: r.value ?? createEmptyValue(r.fieldType),
      dictionary_id: r.dictionaryId,
    }));
  }

  /** 导出为模板字段 VO 列表。 */
  function toTemplateFieldVOs(): TemplateFieldVO[] {
    return rows.value.map((r) => ({
      name: r.name,
      field_type: r.fieldType,
      type_config: r.typeConfig,
      dictionary_id: r.dictionaryId,
    }));
  }

  /**
   * 校验所有行：空名、重名、值格式。
   * @returns 全部通过返回 true 并清空 errors，否则返回 false
   */
  function validate(): boolean {
    const newErrors = new Map<number, { name?: string; value?: string }>();
    const nameSet = new Map<string, number>();

    for (const row of rows.value) {
      const trimmed = row.name.trim();
      if (trimmed === "") {
        newErrors.set(row.uid, {
          name: t("database.field.name-required"),
        });
      } else if (nameSet.has(trimmed)) {
        const firstUid = nameSet.get(trimmed)!;
        newErrors.set(row.uid, {
          ...newErrors.get(row.uid),
          name: t("database.field.name-duplicated"),
        });
        if (!newErrors.has(firstUid)) {
          newErrors.set(firstUid, {
            name: t("database.field.name-duplicated"),
          });
        }
      } else {
        nameSet.set(trimmed, row.uid);
      }

      if (options.withValues) {
        const errKey = validateValue(row.fieldType, row.value!);
        if (errKey !== null) {
          newErrors.set(row.uid, {
            ...newErrors.get(row.uid),
            value: t(`database.field-type.${errKey}`),
          });
        }
      }
    }

    errors.value = newErrors;
    return newErrors.size === 0;
  }

  /** 当前 rows 与初始快照比较（剔除 uid/expanded），判断是否有未保存修改。 */
  function isDirty(): boolean {
    return normalizedJson(rows.value) !== initialSnapshot.value;
  }

  return {
    rows,
    errors,
    addRow,
    removeRow,
    moveRow,
    loadFromNodeFields,
    loadFromTemplateFields,
    toNodeFieldVOs,
    toTemplateFieldVOs,
    validate,
    isDirty,
  };
}
