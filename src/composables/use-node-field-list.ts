import { ref } from "vue";
import type { NodeFieldVO } from "@/api-types";
import type { FieldError } from "@/composables/field-error";
import { DEFAULT_FIELD_TYPE, validateValue } from "@/field-types";
import { t } from "@/i18n";

/** 节点字段编辑行模型。 */
export interface NodeFieldRow {
  /** 行的唯一标识，用作列表渲染的 key。 */
  uid: number;
  name: string;
  fieldType: string;
  /** 字段值字符串（格式见 field-types 模块），null 表示无值。 */
  value: string | null;
  dictionaryId: string | null;
}

/**
 * 将行列表归一化为 JSON 字符串（剔除 uid），供快照比较。
 * @param rows 节点字段行列表
 * @returns 归一化后的 JSON 字符串
 */
function normalizedJson(rows: NodeFieldRow[]): string {
  return JSON.stringify(
    rows.map((r) => ({
      name: r.name,
      fieldType: r.fieldType,
      value: r.value,
      dictionaryId: r.dictionaryId,
    })),
  );
}

/**
 * 节点字段列表编辑逻辑。
 * @returns 行列表、错误表，以及增删、拖拽排序、装载、导出、校验与脏检查方法
 */
export function useNodeFieldList() {
  let nextUid = 1;
  const rows = ref<NodeFieldRow[]>([]);
  const errors = ref<Map<number, FieldError>>(new Map());
  const initialSnapshot = ref("");

  /** 追加一行空字段。 */
  function addRow(): void {
    rows.value.push({
      uid: nextUid++,
      name: "",
      fieldType: DEFAULT_FIELD_TYPE,
      value: null,
      dictionaryId: null,
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
   * 从节点字段 VO 列表装入行并记录初始快照；同时清空校验错误（装载即状态重置，供对话框每次打开时复用）。
   * @param vos 节点字段 VO 列表
   */
  function loadFromNodeFields(vos: NodeFieldVO[]): void {
    nextUid = 1;
    rows.value = vos.map((vo) => ({
      uid: nextUid++,
      name: vo.name,
      fieldType: vo.field_type,
      value: vo.value,
      dictionaryId: vo.dictionary_id,
    }));
    errors.value = new Map();
    initialSnapshot.value = normalizedJson(rows.value);
  }

  /**
   * 导出为节点字段 VO 列表。
   * @returns 节点字段 VO 列表，字段顺序由数组位置表达
   */
  function toNodeFieldVOs(): NodeFieldVO[] {
    return rows.value.map((r) => ({
      name: r.name,
      field_type: r.fieldType,
      value: r.value,
      dictionary_id: r.dictionaryId,
    }));
  }

  /**
   * 校验所有行：空名、重名、值格式。每行至多记录一个错误，名称错误优先于值错误。
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
        continue;
      }
      if (nameSet.has(trimmed)) {
        const firstUid = nameSet.get(trimmed)!;
        const duplicated: FieldError = {
          msg: t("database.field.name-duplicated"),
          highlight: "name",
        };
        newErrors.set(row.uid, duplicated);
        // 名称错误优先：首次出现行即使已有值错误也被重名错误覆盖。
        newErrors.set(firstUid, { ...duplicated });
        continue;
      }
      nameSet.set(trimmed, row.uid);

      const errKey = validateValue(row.fieldType, row.value);
      if (errKey !== null) {
        newErrors.set(row.uid, {
          msg: t(`database.field-type.${errKey}`),
          highlight: "value",
        });
      }
    }

    errors.value = newErrors;
    return newErrors.size === 0;
  }

  /**
   * 当前 rows 与初始快照比较（剔除 uid），判断是否有未保存修改。
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
    loadFromNodeFields,
    toNodeFieldVOs,
    validate,
    isDirty,
  };
}