<!--
  数据节点专属颜色编辑对话框。

  通过 defineExpose 的 open() 以 Promise 形式获取保存结果。
  草稿为可选属性对象（键缺失即默认值），保存时序列化；提供预设、历史、自定义编辑和实时预览。
-->
<script setup lang="ts">
import { ref, watch } from "vue";
import { t } from "@/i18n";
import {
  deserializeNodeColor,
  serializeNodeColor,
  collectNodeColorList,
  type DataNodeColorScheme,
  type DataNodeColorProperties,
  type DataNodeHistoryColor,
} from "./index";
import { DATA_NODE_COLOR_PRESETS } from "./color-presets";
import { userDatabaseNodeSetColor } from "@/api";
import { snackbarErrorCode } from "@/composables/use-snackbar";
import ColorPairSwatch from "./ColorPairSwatch.vue";
import ColorFieldEditor from "./ColorFieldEditor.vue";
import DataNodeColorPreview from "./DataNodeColorPreview.vue";

/** 字段定义项类型 */
type FieldDef = { key: keyof DataNodeColorProperties; labelKey: string };

/** 数据节点字段定义表（8 项，卡片色 → 文字色 → 细节色） */
const NODE_FIELDS: FieldDef[] = [
  { key: "background", labelKey: "database.color-dialog.field-background" },
  { key: "borderUnselected", labelKey: "database.color-dialog.field-border" },
  { key: "borderSelected", labelKey: "database.color-dialog.field-selected-border" },
  { key: "title", labelKey: "database.color-dialog.field-title" },
  { key: "subtitle", labelKey: "database.color-dialog.field-subtitle" },
  { key: "icon", labelKey: "database.color-dialog.field-icon" },
  { key: "handle", labelKey: "database.color-dialog.field-handle" },
  { key: "action", labelKey: "database.color-dialog.field-action" },
];

/** 对话框显示状态 */
const dialog = ref(false);
/** 当前编辑的节点 id */
const currentNodeId = ref("");
/** 预览用标题 */
const currentTitle = ref("");
/** 预览用副标题 */
const currentSubTitle = ref("");
/** 草稿：可选属性对象（键缺失即默认值） */
const draft = ref<DataNodeColorScheme>({ light: {}, dark: {} });
/** 历史颜色组合列表 */
const history = ref<DataNodeHistoryColor[]>([]);
/** 等待 Promise 结算的 resolve */
let resolveOpen: ((value: string | null) => void) | null = null;

/**
 * 结算等待中的 Promise。
 * @param value 结果字符串或 null（取消）
 */
function settle(value: string | null): void {
  resolveOpen?.(value);
  resolveOpen = null;
}

/**
 * 加载数据节点历史颜色组合列表。
 */
async function loadHistory(): Promise<void> {
  history.value = [];
  try {
    history.value = await collectNodeColorList();
  } catch (e) {
    snackbarErrorCode(e);
  }
}

/**
 * 持久化颜色到后端。
 * 成功时结算 Promise 并关闭对话框；失败时报 snackbar 且保持对话框打开、不结算 Promise。
 * @param color 要持久化的颜色字符串（空串表示恢复默认）
 */
async function persist(color: string): Promise<void> {
  try {
    await userDatabaseNodeSetColor(currentNodeId.value, color);
    settle(color);
    dialog.value = false;
  } catch (e) {
    snackbarErrorCode(e);
  }
}

/**
 * 打开对话框。
 * 先结算上一个未关闭的 Promise，再记录参数、反序列化当前颜色为草稿，打开对话框并异步加载历史。
 * @param nodeId 节点 id
 * @param title 预览用标题
 * @param subTitle 预览用副标题
 * @param currentColor 实体 color 字段原值
 * @returns 保存成功 resolve 新序列化串；恢复默认成功 resolve ""；取消/关闭 resolve null
 */
function open(nodeId: string, title: string, subTitle: string, currentColor: string): Promise<string | null> {
  settle(null);
  currentNodeId.value = nodeId;
  currentTitle.value = title;
  currentSubTitle.value = subTitle;
  draft.value = deserializeNodeColor(currentColor);
  dialog.value = true;
  void loadHistory();
  return new Promise((resolve) => {
    resolveOpen = resolve;
  });
}

/** 应用预设到草稿 */
function applyPreset(index: number): void {
  draft.value = structuredClone(DATA_NODE_COLOR_PRESETS[index].scheme);
}

/** 应用历史记录到草稿 */
function applyHistory(entry: DataNodeHistoryColor): void {
  draft.value = structuredClone(entry.scheme);
}

/** 保存草稿 */
function onSave(): void {
  void persist(serializeNodeColor(draft.value));
}

/** 全部恢复默认 */
function onResetAll(): void {
  void persist("");
}

/** 取消 */
function onCancel(): void {
  settle(null);
  dialog.value = false;
}

// 任何途径关闭都按取消结算
watch(dialog, (value) => {
  if (!value) settle(null);
});

defineExpose({ open });
</script>

<template>
  <VDialog v-model="dialog" max-width="48rem" scrollable>
    <VCard>
      <VCardTitle>
        {{ t("database.color-dialog.title-node") }}：{{ currentTitle }}
      </VCardTitle>
      <VCardText>
        <!-- 预设区 -->
        <div class="color-dialog-section">
          <div class="color-dialog-section__title">
            {{ t("database.color-dialog.section-presets") }}
          </div>
          <div class="color-dialog-swatches">
            <ColorPairSwatch
              v-for="(preset, index) in DATA_NODE_COLOR_PRESETS"
              :key="preset.nameKey"
              :light-color="preset.scheme.light.background"
              :dark-color="preset.scheme.dark.background"
              :tooltip="t(preset.nameKey)"
              @click="applyPreset(index)"
            />
          </div>
        </div>

        <!-- 历史区 -->
        <div v-if="history.length" class="color-dialog-section">
          <div class="color-dialog-section__title">
            {{ t("database.color-dialog.section-history") }}
          </div>
          <div class="color-dialog-swatches">
            <ColorPairSwatch
              v-for="(entry, index) in history"
              :key="index"
              :light-color="entry.scheme.light.background"
              :dark-color="entry.scheme.dark.background"
              :tooltip="entry.description"
              @click="applyHistory(entry)"
            />
          </div>
        </div>

        <!-- 编辑区 -->
        <div class="color-dialog-section">
          <div class="color-dialog-editor-grid">
            <div class="color-dialog-editor-col">
              <div class="color-dialog-section__title">
                {{ t("database.color-dialog.section-light") }}
              </div>
              <ColorFieldEditor
                v-for="field in NODE_FIELDS"
                :key="'light-' + field.key"
                :label="t(field.labelKey)"
                v-model="draft.light[field.key]"
                @reset="delete draft.light[field.key]"
              />
            </div>
            <div class="color-dialog-editor-col">
              <div class="color-dialog-section__title">
                {{ t("database.color-dialog.section-dark") }}
              </div>
              <ColorFieldEditor
                v-for="field in NODE_FIELDS"
                :key="'dark-' + field.key"
                :label="t(field.labelKey)"
                v-model="draft.dark[field.key]"
                @reset="delete draft.dark[field.key]"
              />
            </div>
          </div>
        </div>

        <!-- 预览区 -->
        <div class="color-dialog-section">
          <div class="color-dialog-section__title">
            {{ t("database.color-dialog.section-preview") }}
          </div>
          <DataNodeColorPreview
            :scheme="draft"
            :title="currentTitle"
            :sub-title="currentSubTitle"
          />
        </div>
      </VCardText>
      <VCardActions class="justify-end ga-2">
        <VBtn variant="text" @click="onResetAll">
          {{ t("database.color-dialog.reset-all") }}
        </VBtn>
        <VSpacer />
        <VBtn variant="text" @click="onCancel">
          {{ t("database.color-dialog.cancel") }}
        </VBtn>
        <VBtn color="primary" variant="flat" @click="onSave">
          {{ t("database.color-dialog.save") }}
        </VBtn>
      </VCardActions>
    </VCard>
  </VDialog>
</template>

<style lang="scss" scoped>
.color-dialog-section {
  margin-bottom: 1.5rem;
}

.color-dialog-section__title {
  font-size: 0.875rem;
  font-weight: 500;
  margin-bottom: 0.5rem;
  opacity: 0.8;
}

.color-dialog-swatches {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5rem;
}

.color-dialog-editor-grid {
  display: flex;
  gap: 1.5rem;
}

.color-dialog-editor-col {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}
</style>
