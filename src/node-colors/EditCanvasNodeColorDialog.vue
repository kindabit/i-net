<!--
  画布节点专属颜色编辑对话框。

  通过 defineExpose 的 open() 以 Promise 形式获取保存结果。
  草稿为可选属性对象（键缺失即默认值），保存时序列化；提供预设、历史、自定义编辑和实时预览。
  与数据节点编辑器概念独立，故意不合并。
-->
<script setup lang="ts">
import { ref, watch } from "vue";
import { t } from "@/i18n";
import {
  deserializeCanvasColor,
  serializeCanvasColor,
  collectCanvasColorList,
  type CanvasNodeColorScheme,
  type CanvasNodeColorProperties,
  type CanvasNodeHistoryColor,
} from "./index";
import { CANVAS_NODE_COLOR_PRESETS } from "./color-presets";
import { userDatabaseCanvasSetColor } from "@/api";
import { snackbarErrorCode } from "@/composables/use-snackbar";
import ColorPairSwatch from "./ColorPairSwatch.vue";
import ColorFieldEditor from "./ColorFieldEditor.vue";
import CanvasNodeColorPreview from "./CanvasNodeColorPreview.vue";

/** 字段定义项类型 */
type FieldDef = { key: keyof CanvasNodeColorProperties; labelKey: string };

/** 画布节点字段定义表（6 项：背景 / 未选中边框 / 选中边框 / 标题 / 图标 / 工具按钮） */
const CANVAS_FIELDS: FieldDef[] = [
  { key: "background", labelKey: "database.color-dialog.field-background" },
  { key: "borderUnselected", labelKey: "database.color-dialog.field-border" },
  { key: "borderSelected", labelKey: "database.color-dialog.field-selected-border" },
  { key: "title", labelKey: "database.color-dialog.field-title" },
  { key: "icon", labelKey: "database.color-dialog.field-icon" },
  { key: "action", labelKey: "database.color-dialog.field-action" },
];

/** 对话框显示状态 */
const dialog = ref(false);
/** 当前编辑的画布 id */
const currentCanvasId = ref("");
/** 预览用名称 */
const currentName = ref("");
/** 草稿：可选属性对象（键缺失即默认值） */
const draft = ref<CanvasNodeColorScheme>({ light: {}, dark: {} });
/** 历史颜色组合列表 */
const history = ref<CanvasNodeHistoryColor[]>([]);
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
 * 加载画布节点历史颜色组合列表。
 */
async function loadHistory(): Promise<void> {
  history.value = [];
  try {
    history.value = await collectCanvasColorList();
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
    await userDatabaseCanvasSetColor(currentCanvasId.value, color);
    settle(color);
    dialog.value = false;
  } catch (e) {
    snackbarErrorCode(e);
  }
}

/**
 * 打开对话框。
 * 先结算上一个未关闭的 Promise，再记录参数、反序列化当前颜色为草稿，打开对话框并异步加载历史。
 * @param canvasId 画布 id
 * @param name 预览用名称
 * @param currentColor 实体 color 字段原值
 * @returns 保存成功 resolve 新序列化串；恢复默认成功 resolve ""；取消/关闭 resolve null
 */
function open(canvasId: string, name: string, currentColor: string): Promise<string | null> {
  settle(null);
  currentCanvasId.value = canvasId;
  currentName.value = name;
  draft.value = deserializeCanvasColor(currentColor);
  dialog.value = true;
  void loadHistory();
  return new Promise((resolve) => {
    resolveOpen = resolve;
  });
}

/** 应用预设到草稿 */
function applyPreset(index: number): void {
  draft.value = structuredClone(CANVAS_NODE_COLOR_PRESETS[index].scheme);
}

/** 应用历史记录到草稿 */
function applyHistory(entry: CanvasNodeHistoryColor): void {
  draft.value = structuredClone(entry.scheme);
}

/** 保存草稿 */
function onSave(): void {
  void persist(serializeCanvasColor(draft.value));
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
        {{ t("database.color-dialog.title-canvas") }}：{{ currentName }}
      </VCardTitle>
      <VCardText>
        <!-- 预设区 -->
        <div class="color-dialog-section">
          <div class="color-dialog-section__title">
            {{ t("database.color-dialog.section-presets") }}
          </div>
          <div class="color-dialog-swatches">
            <ColorPairSwatch
              v-for="(preset, index) in CANVAS_NODE_COLOR_PRESETS"
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
                v-for="field in CANVAS_FIELDS"
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
                v-for="field in CANVAS_FIELDS"
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
          <CanvasNodeColorPreview
            :scheme="draft"
            :title="currentName"
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
