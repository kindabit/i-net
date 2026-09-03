<!--
  节点字段行组件。

  以卡片形式编辑单个节点字段的定义：字段名、字段类型、字段值与字典绑定，并支持拖拽排序。
  拖拽手柄位于卡片左侧并纵跨两行；字段名默认展示、双击进入编辑态，失焦或回车提交、Esc 取消。
  字段名与操作按钮位于第一行，字段类型下拉与值编辑器位于第二行，两行之间以分隔线隔开。
  字典绑定收纳在齿轮按钮的悬浮扩展面板中，通过 TreeSelect 选择。
  值编辑器固定独占新行展示；instant 系列的精度由其值编辑器自行维护（由值本身的格式表达）。
  readonly 模式（影子节点只读查看）下隐藏删除按钮与拖拽手柄，所有输入控件只读。
  校验错误按行展示：错误信息统一显示在卡片错误区，出错的输入部位（字段名输入框或值编辑器）红色高亮。
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
import { computed as setupComputed, nextTick, ref } from "vue";
import { getFieldTypeDef, valueKindOf } from "@/field-types";
import { prunedDictionaryTree } from "@/dictionary";
import { useMenuDismiss } from "@/composables/use-menu-dismiss";
import TreeSelect from "@/components/TreeSelect.vue";
import type { FieldError } from "@/composables/field-error";
import type { NodeFieldRow } from "@/composables/use-node-field-list";
import FieldValueEditor from "@/components/field-editors/FieldValueEditor.vue";

const props = defineProps<{
  row: NodeFieldRow;
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

/** 字典树非空且当前字段类型支持字典绑定时，齿轮扩展面板显示字典绑定选择器。 */
const showDictionary = setupComputed(() => {
  if (prunedDictionaryTree.value.length === 0) return false;
  return getFieldTypeDef(props.row.fieldType)?.supportsDictionary ?? false;
});

/** 本行自己的校验错误；无错误为 undefined。 */
const ownError = setupComputed(() => props.errors.get(props.row.uid));

/** 齿轮扩展面板是否打开。 */
const extensionMenuOpen = ref(false);
// .tree-select-popper 同为内部：面板内 TreeSelect 的树形下拉 teleport 到 body，点击树节点不应关闭齿轮面板。
useMenuDismiss(extensionMenuOpen, ".field-extension-popper, .tree-select-popper");

/** 字段名是否处于编辑态。 */
const editingName = ref(false);

/** 字段名编辑草稿，进入编辑态时以 row.name 初始化；提交时才写回 row.name。 */
const nameDraft = ref("");

/** 字段名编辑态输入框的 DOM 引用（用于进入编辑态时聚焦）。 */
const nameInput = ref<HTMLInputElement | null>(null);

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
 * 切换字段类型：valueKind 相同时保留值，不同时清空值；新类型不支持字典绑定时清空 dictionaryId。
 * @param newType 新字段类型的 key
 */
function onFieldTypeChange(newType: string): void {
  const oldKind = valueKindOf(props.row.fieldType);
  const newKind = valueKindOf(newType);
  if (oldKind !== newKind) {
    props.row.value = null;
  }
  props.row.fieldType = newType;
  const def = getFieldTypeDef(newType);
  if (def && !def.supportsDictionary) {
    props.row.dictionaryId = null;
  }
}

/**
 * 双击字段名进入编辑态：以当前字段名初始化草稿，并聚焦选中新出现的输入框（只读模式不响应）。
 */
async function startNameEdit(): Promise<void> {
  if (props.readonly) return;
  nameDraft.value = props.row.name;
  editingName.value = true;
  await nextTick();
  nameInput.value?.focus();
  nameInput.value?.select();
}

/**
 * 失焦提交字段名：以草稿 trim 后的结果写回行模型并退出编辑态。
 */
function commitNameEdit(): void {
  props.row.name = nameDraft.value.trim();
  editingName.value = false;
}

/**
 * 取消字段名编辑：不写回行模型，直接退出编辑态。
 */
function cancelNameEdit(): void {
  editingName.value = false;
}

/**
 * 回车键提交字段名：触发输入框失焦，由失焦处理器统一完成提交。
 * @param event 键盘事件
 */
function submitNameEditOnEnter(event: KeyboardEvent): void {
  (event.target as HTMLInputElement | null)?.blur();
}
</script>

<template>
  <div
    class="node-field-card"
    @dragover.prevent
    @drop="onDrop"
  >
    <div class="field-body">
      <div
        v-if="!readonly"
        class="drag-handle"
        draggable="true"
        @dragstart="onDragStart"
      >
        <VIcon icon="mdi-drag-vertical" />
      </div>
      <div class="field-rows">
        <div class="field-name-row">
          <div
            v-if="!editingName"
            class="field-name-display"
            :class="{ 'field-name-display--error': ownError?.highlight === 'name' }"
            :title="t('database.field.name-edit-hint')"
            @click="startNameEdit"
          >
            <span v-if="row.name.trim() !== ''">{{ row.name }}</span>
            <span v-else class="field-name-placeholder">
              {{ t("database.field.name-empty") }}
            </span>
          </div>
          <!-- 编辑态使用原生 input 而非 VTextField：需要与展示态精确等高（1.75rem），避免状态切换时行高跳动。 -->
          <input
            v-else
            ref="nameInput"
            v-model="nameDraft"
            type="text"
            class="field-name-edit"
            :class="{ 'field-name-edit--error': ownError?.highlight === 'name' }"
            @blur="commitNameEdit"
            @keyup.enter="submitNameEditOnEnter"
            @keyup.esc="cancelNameEdit"
          />
          <VSpacer />
          <VMenu
            v-model="extensionMenuOpen"
            location="end"
            :close-on-content-click="false"
          >
            <template #activator="{ props: menuProps }">
              <VBtn
                icon="mdi-cog-outline"
                variant="text"
                density="compact"
                tabindex="-1"
                :disabled="!showDictionary"
                v-bind="menuProps"
              />
            </template>
            <VCard class="field-extension-popper">
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
            </VCard>
          </VMenu>
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
        <VDivider />
        <div class="field-value-row">
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
          <FieldValueEditor
            v-model="row.value"
            :field-type="row.fieldType"
            :dictionary-id="row.dictionaryId"
            :error-highlight="ownError?.highlight === 'value'"
            :readonly="readonly"
            class="field-value-editor"
          />
        </div>
      </div>
    </div>
    <div v-if="ownError" class="field-error">
      {{ ownError.msg }}
    </div>
  </div>
</template>

<style lang="scss" scoped>
.node-field-card {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.field-body {
  display: flex;
  gap: 0.5rem;
}

.drag-handle {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 1.25rem;
  flex: none;
  align-self: stretch;
  cursor: grab;
  opacity: 0.38;
  color: rgb(var(--v-theme-on-surface));
  border-right: 1px solid rgba(var(--v-theme-on-surface), 0.12);
}

.field-rows {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  flex: 1;
  min-width: 0;
}

.field-name-row {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

/* 展示态与编辑态统一为固定 1.75rem 高（border-box），状态切换时行高不跳动；
   展示态的 1px 透明边框与编辑态的可见边框占位一致，保证两种状态宽度也相同。 */
.field-name-display,
.field-name-edit {
  height: 1.75rem;
  box-sizing: border-box;
  font-size: 0.8125rem;
  font-family: inherit;
  border-radius: 0.25rem;
  padding: 0 0.375rem;
}

.field-name-display {
  display: flex;
  align-items: center;
  cursor: text;
  border: 1px solid transparent;

  &:hover {
    background: rgba(var(--v-theme-on-surface), 0.06);
  }
}

.field-name-display--error {
  color: rgb(var(--v-theme-error));
}

.field-name-placeholder {
  color: rgba(var(--v-theme-on-surface), 0.6);
}

.field-name-edit {
  flex: 0 1 12rem;
  min-width: 0;
  border: 1px solid rgba(var(--v-border-color), var(--v-border-opacity));
  outline: none;
  background: transparent;
  color: inherit;

  &:focus {
    border-color: rgb(var(--v-theme-primary));
  }
}

.field-name-edit--error,
.field-name-edit--error:focus {
  border-color: rgb(var(--v-theme-error));
}

.field-value-row {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.field-type-select {
  flex: none;
  width: 9rem;
}

.field-value-editor {
  flex: 1;
  min-width: 0;
}

.field-error {
  color: rgb(var(--v-theme-error));
  font-size: 0.75rem;
  margin-top: 0.25rem;
}

.field-extension-popper {
  min-width: 16rem;
  padding: 0.75rem;
}
</style>