import { ref } from "vue";
import type { TemplateFieldVO } from "@/api-types";
import type { FieldError } from "@/composables/field-error";
import { DEFAULT_FIELD_TYPE } from "@/field-types";
import { t } from "@/i18n";

/** 模板字段编辑行模型。 */
export interface TemplateFieldRow {
  /** 行的唯一标识，用作列表渲染的 key。 */
  uid: number;
  name: string;
  fieldType: string;
  dictionaryId: string | null;
  /** 高级选项区是否展开。 */
  expanded: boolean;
}

/**
 * 将行列表归一化为 JSON 字符串（剔除 uid/expanded），供快照比较。
 * @param rows 模板字段行列表
 * @returns 归一化后的 JSON 字符串
 */
function normalizedJson(rows: TemplateFieldRow[]): string {
  return JSON.stringify(
    rows.map((r) => ({
      name: r.name,
      fieldType: r.fieldType,
      dictionaryId: r.dictionaryId,
    })),
  );
}

/**
 * 模板字段列表编辑逻辑。
 * @returns 行列表、错误表，以及增删、拖拽排序、装载、导出、校验与脏检查方法
 */
export function useTemplateFieldList() {
  let nextUid = 1;
  const rows = ref<TemplateFieldRow[]>([]);
  const errors = ref<Map<number, FieldError>>(new Map());
  const initialSnapshot = ref("");

  /** 追加一行空字段。 */
  function addRow(): void {
    rows.value.push({
      uid: nextUid++,
      name: "",
      fieldType: DEFAULT_FIELD_TYPE,
      dictionaryId: null,
      expanded: false,
    });
  }

  /**
   * 按 uid 删除一行。
   * @param uid 待删除行的唯一标识
   */
  function removeRow(uid: number): void {
    rows.value = rows.value.filter((r) => r.uid !== uid);
  }

  /**
   * 拖拽排序：把 fromUid 行移动到 toUid 行的位置。
   * @param fromUid 被拖拽行的 uid
   * @param toUid 落点行的 uid
   */
  function moveRow(fromUid: number, toUid: number): void {
    const fromIdx = rows.value.findIndex((r) => r.uid === fromUid);
    const toIdx = rows.value.findIndex((r) => r.uid === toUid);
    if (fromIdx === -1 || toIdx === -1) return;
    const [removed] = rows.value.splice(fromIdx, 1);
    rows.value.splice(toIdx, 0, removed);
  }

  /**
   * 从模板字段 VO 列表装入行并记录初始快照；同时清空校验错误（装载即状态重置，供对话框每次打开或切换模板时复用）。
   * @param vos 模板字段 VO 列表
   */
  function loadFromTemplateFields(vos: TemplateFieldVO[]): void {
    nextUid = 1;
    rows.value = vos.map((vo) => ({
      uid: nextUid++,
      name: vo.name,
      fieldType: vo.field_type,
      dictionaryId: vo.dictionary_id,
      expanded: false,
    }));
    errors.value = new Map();
    initialSnapshot.value = normalizedJson(rows.value);
  }

  /**
   * 导出为模板字段 VO 列表。
   * @returns 模板字段 VO 列表，字段顺序由数组位置表达
   */
  function toTemplateFieldVOs(): TemplateFieldVO[] {
    return rows.value.map((r) => ({
      name: r.name,
      field_type: r.fieldType,
      dictionary_id: r.dictionaryId,
    }));
  }

  /**
   * 校验所有行：空名、重名。每行至多记录一个错误。
   * @returns 全部通过返回 true 并清空 errors，否则返回 false
   */
  function validate(): boolean {
    const newErrors = new Map<number, FieldError>();
    const nameSet = new Map<string, number>();

    for (const row of rows.value) {
      const trimmed = row.name.trim();
      if (trimmed === "") {
        newErrors.set(row.uid, {
          msg: t("database.field.name-required"),
          highlight: "name",
        });
      } else if (nameSet.has(trimmed)) {
        const firstUid = nameSet.get(trimmed)!;
        const duplicated: FieldError = {
          msg: t("database.field.name-duplicated"),
          highlight: "name",
        };
        newErrors.set(row.uid, duplicated);
        newErrors.set(firstUid, { ...duplicated });
      } else {
        nameSet.set(trimmed, row.uid);
      }
    }

    errors.value = newErrors;
    return newErrors.size === 0;
  }

  /**
   * 当前 rows 与初始快照比较（剔除 uid/expanded），判断是否有未保存修改。
   * @returns 有未保存修改时返回 true
   */
  function isDirty(): boolean {
    return normalizedJson(rows.value) !== initialSnapshot.value;
  }

  return {
    rows,
    errors,
    addRow,
    removeRow,
    moveRow,
    loadFromTemplateFields,
    toTemplateFieldVOs,
    validate,
    isDirty,
  };
}