<!--
  模板字段行组件。

  以卡片形式编辑单个模板字段的定义：字段名、字段类型与字典绑定，并支持拖拽排序。
  模板字段不含字段值。
  校验错误按行展示：错误信息显示在卡片错误区，字段名输入框红色高亮。
-->
<script lang="ts">
import { computed } from "vue";
import { t } from "@/i18n";
import { FIELD_TYPES } from "@/field-types";

/**
 * 字段类型下拉选项（所有组件实例共享一份）。
 *
 * 仅依赖静态类型表与当前语言：t 内部对 locale 的响应式读取使该
 * computed 追踪语言，切换语言时自动重算；模块作用域只创建一次。
 */
const typeItems = computed(() =>
  FIELD_TYPES.map((ft) => ({
    title: t("database.field-type." + ft.i18nKey),
    value: ft.key,
  })),
);
</script>

<script setup lang="ts">
import { computed as setupComputed } from "vue";
import { getFieldTypeDef } from "@/field-types";
import { prunedDictionaryTree } from "@/dictionary";
import TreeSelect from "@/components/TreeSelect.vue";
import type { FieldError } from "@/composables/field-error";
import type { TemplateFieldRow } from "@/composables/use-template-field-list";

const props = defineProps<{
  row: TemplateFieldRow;
  /** 字段行校验错误表（uid → 错误）；validate 一次性完整替换该 Map，引用变化驱动组件重取本行错误。 */
  errors: Map<number, FieldError>;
  /** 只读模式：禁用编辑、删除与拖拽。 */
  readonly?: boolean;
}>();

const emit = defineEmits<{
  remove: [uid: number];
  dragStart: [uid: number];
  dropOn: [uid: number];
}>();

/** 字典树非空且当前字段类型支持字典绑定时，高级选项区显示字典绑定选择器。 */
const showDictionary = setupComputed(() => {
  if (prunedDictionaryTree.value.length === 0) return false;
  return getFieldTypeDef(props.row.fieldType)?.supportsDictionary ?? false;
});

/** 本行自己的校验错误；无错误为 undefined。 */
const ownError = setupComputed(() => props.errors.get(props.row.uid));

/**
 * 拖拽开始：只读模式下阻止拖拽，否则设置拖拽数据并通知父组件。
 * @param event 拖拽事件
 */
function onDragStart(event: DragEvent): void {
  if (props.readonly) {
    event.preventDefault();
    return;
  }
  if (event.dataTransfer) {
    event.dataTransfer.effectAllowed = "move";
    event.dataTransfer.setData("text/plain", String(props.row.uid));
  }
  emit("dragStart", props.row.uid);
}

/** 拖放落点：只读模式下忽略，否则通知父组件把拖拽行移动到本行位置。 */
function onDrop(): void {
  if (props.readonly) return;
  emit("dropOn", props.row.uid);
}

/**
 * 切换字段类型：更新字段类型；新类型不支持字典绑定时清空 dictionaryId。
 * @param newType 新字段类型的 key
 */
function onFieldTypeChange(newType: string): void {
  props.row.fieldType = newType;
  const def = getFieldTypeDef(newType);
  if (def && !def.supportsDictionary) {
    props.row.dictionaryId = null;
  }
}
</script>

<template>
  <div
    class="template-field-card"
    :draggable="!readonly"
    @dragstart="onDragStart"
    @dragover.prevent
    @drop="onDrop"
  >
    <div class="field-main-row">
      <VIcon v-if="!readonly" class="drag-handle" icon="mdi-drag-vertical" />
      <VTextField
        v-model="row.name"
        :label="t('database.field.name-label')"
        :error="ownError?.highlight === 'name'"
        :readonly="readonly"
        variant="outlined"
        density="compact"
        hide-details="auto"
        class="field-name"
      />
      <!-- no-auto-scroll：规避 Vuetify 4 的 scrollToIndex 无限重试 bug（长列表选中靠近底部的项时，菜单宽度持续振荡、滚动被压制）。 -->
      <VSelect
        :model-value="row.fieldType"
        :items="typeItems"
        :readonly="readonly"
        @update:model-value="(val: unknown) => onFieldTypeChange(val as string)"
        density="compact"
        variant="outlined"
        hide-details="auto"
        no-auto-scroll
        class="field-type-select"
      />
      <VSpacer />
      <VBtn
        :icon="row.expanded ? 'mdi-chevron-up' : 'mdi-chevron-down'"
        variant="text"
        density="compact"
        tabindex="-1"
        @click="row.expanded = !row.expanded"
      />
      <VBtn
        v-if="!readonly"
        icon="mdi-delete-outline"
        variant="text"
        density="compact"
        color="error"
        tabindex="-1"
        @click="emit('remove', row.uid)"
      />
    </div>
    <div v-if="ownError" class="field-error">
      {{ ownError.msg }}
    </div>
    <div v-show="row.expanded" class="field-advanced">
      <TreeSelect
        v-if="showDictionary"
        v-model="row.dictionaryId"
        :label="t('database.field.dictionary-label')"
        :items="prunedDictionaryTree"
        item-title="entry.value"
        item-value="entry.id"
        item-children="children"
        :readonly="readonly"
      />
    </div>
  </div>
</template>

<style lang="scss" scoped>
.template-field-card {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.field-main-row {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.drag-handle {
  cursor: grab;
  opacity: 0.38;
  color: rgb(var(--v-theme-on-surface));
  flex: none;
}

.field-name {
  flex: 0 1 12rem;
}

.field-type-select {
  flex: none;
  width: 12rem;
}

.field-error {
  color: rgb(var(--v-theme-error));
  font-size: 0.75rem;
  margin-top: 0.25rem;
}

.field-advanced {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}
</style>