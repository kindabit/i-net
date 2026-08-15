<!--
  字段定义行组件（节点字段与模板字段共用）。

  以卡片形式编辑单个字段的定义：字段名、字段类型、按类型动态渲染的
  类型配置项与值编辑器、字典绑定，并支持拖拽排序。
-->
<script lang="ts">
import { computed } from "vue";
import { t } from "@/i18n";
import { fieldTypeGroups } from "@/field-types";

/**
 * 字段类型下拉选项（按 valueKind 分组，所有组件实例共享一份）。
 *
 * 仅依赖静态 schema 与当前语言：t 内部对 locale 的响应式读取使该
 * computed 追踪语言，切换语言时自动重算；模块作用域只创建一次。
 */
const typeItems = computed(() => {
  const groups = fieldTypeGroups();
  const items: Array<{
    type?: string;
    title: string;
    value?: string;
    props?: Record<string, unknown>;
  }> = [];
  for (const group of groups) {
    items.push({
      type: "subheader",
      title: t("database.field-type.value-kind-" + group.valueKind),
    });
    for (const ft of group.types) {
      items.push({
        title: t("database.field-type." + ft.i18nKey),
        value: ft.key,
        props: { class: "pl-6" },
      });
    }
  }
  return items;
});
</script>

<script setup lang="ts">
import {
  createEmptyValue,
  defaultTypeConfig,
  getFieldTypeDef,
  valueKindOf,
} from "@/field-types";
import { prunedDictionaryTree } from "@/dictionary";
import TreeSelect from "@/components/TreeSelect.vue";
import type { FieldRow } from "@/composables/use-field-list";
import FieldValueEditor from "@/components/field-editors/FieldValueEditor.vue";

const props = defineProps<{
  row: FieldRow;
  /** 是否启用值编辑（模板模式为 false）。 */
  withValues: boolean;
  /** 字段名错误文本（空名/重名），无错误为 undefined。 */
  nameError?: string;
  /** 值错误文本，无错误为 undefined。 */
  valueError?: string;
}>();

const emit = defineEmits<{
  remove: [uid: number];
  dragStart: [uid: number];
  dropOn: [uid: number];
}>();

const showDictionary = computed(() => {
  if (prunedDictionaryTree.value.length === 0) return false;
  return getFieldTypeDef(props.row.fieldType)?.supportsDictionary ?? false;
});

/** 字段类型是否以多行展示（决定值编辑器在卡片中的行布局）。 */
const multiRow = computed(
  () => getFieldTypeDef(props.row.fieldType)?.multiRow ?? false,
);

function onDragStart(event: DragEvent) {
  if (event.dataTransfer) {
    event.dataTransfer.effectAllowed = "move";
    event.dataTransfer.setData("text/plain", String(props.row.uid));
  }
  emit("dragStart", props.row.uid);
}

function onDrop() {
  emit("dropOn", props.row.uid);
}

/**
 * 切换字段类型。valueKind 相同时保留现有值，不同时重建空值；
 * 新类型不支持字典绑定时清空 dictionaryId；typeConfig 按新类型默认值重置。
 * @param newType 新字段类型的 key
 */
function onFieldTypeChange(newType: string): void {
  const oldKind = valueKindOf(props.row.fieldType);
  const newKind = valueKindOf(newType);
  if (oldKind !== newKind || !props.withValues) {
    props.row.value = props.withValues ? createEmptyValue(newType) : null;
  }
  props.row.fieldType = newType;
  props.row.typeConfig = defaultTypeConfig(newType);
  const def = getFieldTypeDef(newType);
  if (def && !def.supportsDictionary) {
    props.row.dictionaryId = null;
  }
}

// type config 处理

const precisionConfig = computed(() => {
  return getFieldTypeDef(props.row.fieldType)?.typeConfig?.precision;
});

const precisionItems = computed(() => {
  if (!precisionConfig.value) return [];
  return precisionConfig.value.options.map((opt) => ({
    title: t("database.field-type.precision-" + opt),
    value: opt,
  }));
});

function onPrecisionChange(val: unknown) {
  props.row.typeConfig = {
    ...(props.row.typeConfig ?? {}),
    precision: val as string,
  };
}
</script>

<template>
  <div
    class="field-def-card"
    draggable="true"
    @dragstart="onDragStart"
    @dragover.prevent
    @drop="onDrop"
  >
    <div class="field-def-main-row">
      <VIcon class="drag-handle" icon="mdi-drag-vertical" />
      <VTextField
        v-model="row.name"
        :label="t('database.field.name-label')"
        :error-messages="nameError"
        variant="outlined"
        density="compact"
        hide-details="auto"
        class="field-name"
      />
      <!-- no-auto-scroll：规避 Vuetify 4 的 scrollToIndex 无限重试 bug（长列表选中靠近底部的项时，菜单宽度持续振荡、滚动被压制）。 -->
      <VSelect
        :model-value="row.fieldType"
        :items="typeItems"
        @update:model-value="(val: unknown) => onFieldTypeChange(val as string)"
        density="compact"
        variant="outlined"
        hide-details="auto"
        no-auto-scroll
        class="field-type-select"
      />
      <div class="field-value-slot">
        <FieldValueEditor
          v-if="withValues && !multiRow"
          v-model="row.value"
          :field-type="row.fieldType"
          :type-config="row.typeConfig"
          :dictionary-id="row.dictionaryId"
        />
      </div>
      <VBtn
        :icon="row.expanded ? 'mdi-chevron-up' : 'mdi-chevron-down'"
        variant="text"
        density="compact"
        @click="row.expanded = !row.expanded"
      />
      <VBtn
        icon="mdi-delete-outline"
        variant="text"
        density="compact"
        color="error"
        @click="emit('remove', row.uid)"
      />
    </div>
    <div v-if="withValues && multiRow" class="field-value-row">
      <FieldValueEditor
        v-model="row.value"
        :field-type="row.fieldType"
        :type-config="row.typeConfig"
        :dictionary-id="row.dictionaryId"
      />
    </div>
    <div v-if="withValues && valueError" class="field-value-error">
      {{ t("database.field-type." + valueError) }}
    </div>
    <div v-show="row.expanded" class="field-advanced">
      <VSelect
        v-if="precisionConfig"
        :label="t('database.field-type.precision')"
        :model-value="
          row.typeConfig?.precision ?? precisionConfig.default
        "
        :items="precisionItems"
        @update:model-value="onPrecisionChange"
        density="compact"
        variant="outlined"
        hide-details="auto"
      />
      <TreeSelect
        v-if="showDictionary"
        v-model="row.dictionaryId"
        :label="t('database.field.dictionary-label')"
        :items="prunedDictionaryTree"
        item-title="entry.value"
        item-value="entry.id"
        item-children="children"
      />
    </div>
  </div>
</template>

<style lang="scss" scoped>
.field-def-card {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.field-def-main-row {
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
  width: 9rem;
}

.field-value-slot {
  flex: 1;
  min-width: 0;
}

.field-value-row {
  width: 100%;
}

.field-advanced {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.field-value-error {
  color: rgb(var(--v-theme-error));
  font-size: 0.75rem;
  margin-top: 0.25rem;
}
</style>
