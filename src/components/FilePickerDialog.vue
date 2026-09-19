<!--
  通用文件选择对话框（纯 Web 渲染的文件系统浏览器）。

  通过 defineExpose 的 open() 以 Promise 形式获取用户选择：
  open 模式选择已有文件，save 模式输入目标文件名；
  确认返回目标文件的完整路径，取消或关闭返回 null。
  目录内容经后端 file_system_* 接口读取，不使用任何系统原生文件选择能力。
-->
<script lang="ts">
/** 打开文件选择对话框的选项 */
export interface FilePickerOptions {
  /** 模式：open 选择已有文件；save 输入目标文件名 */
  mode: "open" | "save";
  /** 对话框标题（由调用方传入本地化文本） */
  title: string;
  /** save 模式的文件名初值；open 模式忽略 */
  defaultFileName?: string;
  /** 需要可选的扩展名（小写、不带点）；为空/省略表示不过滤 */
  extensions?: readonly string[];
  /** 确认按钮文案；省略用 common.confirm */
  confirmText?: string;
}
</script>

<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { t } from "@/i18n";
import {
  fileSystemListDirectory,
  fileSystemPathExists,
  fileSystemRoots,
} from "@/api";
import type { DirectoryEntryVO, DirectoryListingVO } from "@/api-types";
import { snackbarErrorCode, snackbarText } from "@/composables/use-snackbar";
import ConfirmDialog from "@/components/ConfirmDialog.vue";
import { joinPath, matchesExtensions, validateFileName } from "@/utils/file-picker";

/** 列表行模型：条目与是否置灰不可选中的展示状态 */
interface FilePickerRow {
  /** 目录条目 */
  entry: DirectoryEntryVO;
  /** 是否置灰不可选中（显示全部文件开关打开后，扩展名不匹配的文件） */
  disabled: boolean;
}

/** 列表行高（像素；对应 CSS 中的 2.5rem，供 VVirtualScroll 以像素单位使用） */
const ROW_HEIGHT_PX = 2.5 * 16;

/** 对话框显示状态 */
const dialog = ref(false);
/** 当前展示的选项 */
const options = ref<FilePickerOptions>({ mode: "open", title: "" });
/** 等待用户选择的 Promise resolve（仅生效一次） */
let resolveOpen: ((value: string | null) => void) | null = null;

/** 当前目录清单；首次加载失败时保持 null */
const listing = ref<DirectoryListingVO | null>(null);
/** 根路径列表（打开对话框时加载） */
const roots = ref<string[]>([]);
/** 路径输入框内容 */
const pathInput = ref("");
/** 当前选中条目的完整路径 */
const selectedPath = ref<string | null>(null);
/** 是否显示扩展名不匹配的文件 */
const showAllFiles = ref(false);
/** save 模式的文件名输入内容 */
const fileName = ref("");
/** save 模式文件名输入的错误状态（用户修正输入时自动解除） */
const fileNameError = ref(false);
/** 目录读取的请求序号，用于丢弃过期响应 */
let loadToken = 0;
/** save 确认流程进行中（防止重复触发覆盖确认） */
const saving = ref(false);
/** 虚拟列表的重建键：每次成功加载目录后自增，复位虚拟滚动状态 */
const listKey = ref(0);
const confirmDialogRef = ref<InstanceType<typeof ConfirmDialog>>();

/** 扩展名过滤是否生效 */
const filterActive = computed(() => (options.value.extensions?.length ?? 0) > 0);

/** 列表展示的行（目录恒显示；文件按扩展名过滤与“显示全部文件”开关决定） */
const rows = computed<FilePickerRow[]>(() => {
  const entries = listing.value?.entries ?? [];
  const extensions = options.value.extensions ?? [];
  const result: FilePickerRow[] = [];
  for (const entry of entries) {
    if (
      entry.is_directory ||
      extensions.length === 0 ||
      matchesExtensions(entry.name, extensions)
    ) {
      result.push({ entry, disabled: false });
    } else if (showAllFiles.value) {
      result.push({ entry, disabled: true });
    }
  }
  return result;
});

/** 当前选中的文件条目；未选中或选中的是目录时为 null */
const selectedFile = computed<DirectoryEntryVO | null>(() => {
  const row = rows.value.find((item) => item.entry.path === selectedPath.value);
  if (!row || row.entry.is_directory) return null;
  return row.entry;
});

/** 确认按钮是否禁用（目录未加载完成，或 open 模式未选中文件时禁用） */
const confirmDisabled = computed(
  () =>
    listing.value === null ||
    (options.value.mode === "open" && selectedFile.value === null),
);

/**
 * 打开对话框并等待用户选择。
 * @param opts 对话框选项
 * @returns 确认返回目标文件的完整路径，取消或关闭返回 null
 */
function open(opts: FilePickerOptions): Promise<string | null> {
  // 重复打开时先按取消结算上一次等待，避免 Promise 泄漏
  settle(null);
  options.value = opts;
  listing.value = null;
  roots.value = [];
  pathInput.value = "";
  selectedPath.value = null;
  showAllFiles.value = false;
  fileName.value = opts.defaultFileName ?? "";
  fileNameError.value = false;
  saving.value = false;
  listKey.value += 1;
  dialog.value = true;
  void loadRoots();
  void loadDirectory(null);
  return new Promise((resolve) => {
    resolveOpen = resolve;
  });
}

/**
 * 结算等待中的 Promise。
 * @param value 用户选择或 null
 */
function settle(value: string | null) {
  resolveOpen?.(value);
  resolveOpen = null;
}

/** 加载根路径列表；失败仅提示，不影响其它导航方式 */
async function loadRoots(): Promise<void> {
  try {
    roots.value = await fileSystemRoots();
  } catch (e) {
    roots.value = [];
    snackbarErrorCode(e);
  }
}

/**
 * 读取并切换到指定目录；失败提示并保持当前目录不变。
 * @param path 目标目录路径；null 表示用户主目录
 */
async function loadDirectory(path: string | null): Promise<void> {
  const token = ++loadToken;
  try {
    const result = await fileSystemListDirectory(path);
    if (token !== loadToken) return;
    listing.value = result;
    pathInput.value = result.path;
    selectedPath.value = null;
    listKey.value += 1;
  } catch (e) {
    if (token !== loadToken) return;
    snackbarErrorCode(e);
    // 恢复路径输入框显示当前目录；首次加载失败时清空
    pathInput.value = listing.value?.path ?? "";
  }
}

/** 导航到用户主目录 */
function goHome(): void {
  void loadDirectory(null);
}

/** 导航到当前目录的上级目录 */
function goUp(): void {
  const parent = listing.value?.parent;
  if (parent === null || parent === undefined) return;
  void loadDirectory(parent);
}

/**
 * 导航到指定根路径。
 * @param root 根路径
 */
function goToRoot(root: string): void {
  void loadDirectory(root);
}

/** 路径输入框回车：按输入内容直接请求后端导航 */
function onPathEnter(): void {
  void loadDirectory(pathInput.value);
}

/**
 * 单击行：选中该条目；置灰行不可选中。
 * @param row 目标行
 */
function onRowClick(row: FilePickerRow): void {
  if (row.disabled) return;
  selectedPath.value = row.entry.path;
}

/**
 * 双击行：目录进入下一级；open 模式的普通文件立即确认返回。
 * @param row 目标行
 */
function onRowDblClick(row: FilePickerRow): void {
  if (row.disabled) return;
  if (row.entry.is_directory) {
    void loadDirectory(row.entry.path);
    return;
  }
  if (options.value.mode === "open") {
    selectedPath.value = row.entry.path;
    settle(row.entry.path);
    dialog.value = false;
  }
}

/** 确认当前选择 */
function onConfirm(): void {
  if (options.value.mode === "open") {
    const file = selectedFile.value;
    // 确认按钮在未选中文件时处于禁用状态，正常交互下 file 不为 null
    if (file === null) return;
    settle(file.path);
    dialog.value = false;
    return;
  }
  void confirmSave();
}

/** save 模式确认：校验文件名合法且非空，目标已存在时二次确认覆盖，成功后返回完整路径 */
async function confirmSave(): Promise<void> {
  if (saving.value || listing.value === null) return;
  const name = fileName.value.trim();
  if (name === "") {
    fileNameError.value = true;
    snackbarText(t("file-picker.file-name-required"), "warning");
    return;
  }
  const nameError = validateFileName(name);
  if (nameError !== null) {
    fileNameError.value = true;
    snackbarText(
      t(
        nameError === "reserved-name"
          ? "file-picker.file-name-reserved"
          : "file-picker.file-name-invalid",
      ),
      "warning",
    );
    return;
  }
  const fullPath = joinPath(listing.value?.path ?? "", name);
  saving.value = true;
  try {
    if (await fileSystemPathExists(fullPath)) {
      const overwrite = await confirmDialogRef.value?.open({
        title: t("file-picker.overwrite-title"),
        text: t("file-picker.overwrite-text", { name }),
        confirmColor: "warning",
      });
      if (!overwrite) return;
    }
    settle(fullPath);
    dialog.value = false;
  } catch (e) {
    snackbarErrorCode(e);
  } finally {
    saving.value = false;
  }
}

// 任何途径的关闭（取消按钮或其它程序性关闭）都按取消结算
watch(dialog, (value) => {
  if (!value) settle(null);
});

defineExpose({ open });
</script>

<template>
  <VDialog v-model="dialog" persistent max-width="48rem">
    <VCard>
      <VCardTitle>{{ options.title }}</VCardTitle>
      <VCardText class="d-flex flex-column ga-3">
        <div class="d-flex align-center ga-2">
          <VTextField
            v-model="pathInput"
            class="file-picker-path flex-grow-1"
            :label="t('file-picker.path-label')"
            density="comfortable"
            variant="outlined"
            hide-details
            single-line
            @keydown.enter="onPathEnter"
          />
          <VBtn
            icon="mdi-home"
            variant="text"
            :title="t('file-picker.home')"
            @click="goHome"
          />
          <VBtn
            icon="mdi-arrow-up"
            variant="text"
            :title="t('file-picker.up')"
            :disabled="listing?.parent == null"
            @click="goUp"
          />
          <VBtn
            v-if="roots.length === 1"
            icon="mdi-harddisk"
            variant="text"
            :title="t('file-picker.roots')"
            @click="goToRoot(roots[0])"
          />
          <VMenu v-else>
            <template #activator="{ props: menuProps }">
              <VBtn
                v-bind="menuProps"
                icon="mdi-harddisk"
                variant="text"
                :title="t('file-picker.roots')"
                :disabled="roots.length === 0"
              />
            </template>
            <VList density="compact">
              <VListItem
                v-for="root in roots"
                :key="root"
                :title="root"
                @click="goToRoot(root)"
              />
            </VList>
          </VMenu>
          <VSwitch
            v-if="filterActive"
            v-model="showAllFiles"
            class="file-picker-show-all flex-grow-0 flex-shrink-0"
            :label="t('file-picker.show-all-files')"
            density="compact"
            hide-details
          />
        </div>

        <div v-if="rows.length === 0" class="file-picker-empty">
          {{ t("file-picker.empty") }}
        </div>
        <VVirtualScroll
          v-else
          :key="listKey"
          class="file-picker-scroll"
          :items="rows"
          height="24rem"
          :item-height="ROW_HEIGHT_PX"
        >
          <template #default="{ item }">
            <div
              class="file-picker-row"
              :class="{
                'file-picker-row--selected':
                  item.entry.path === selectedPath,
                'file-picker-row--disabled': item.disabled,
              }"
              @click="onRowClick(item)"
              @dblclick="onRowDblClick(item)"
            >
              <VIcon
                :icon="
                  item.entry.is_directory ? 'mdi-folder' : 'mdi-file-outline'
                "
                size="1.25rem"
              />
              <span class="file-picker-row-name">{{ item.entry.name }}</span>
            </div>
          </template>
        </VVirtualScroll>

        <VTextField
          v-if="options.mode === 'save'"
          v-model="fileName"
          :label="t('file-picker.file-name-label')"
          :error="fileNameError"
          density="comfortable"
          variant="outlined"
          hide-details="auto"
          single-line
          @update:model-value="fileNameError = false"
          @keydown.enter="onConfirm"
        />
      </VCardText>
      <VCardActions class="justify-end ga-2">
        <VBtn variant="text" @click="dialog = false">
          {{ t("common.cancel") }}
        </VBtn>
        <VBtn
          color="primary"
          variant="flat"
          :disabled="confirmDisabled"
          @click="onConfirm"
        >
          {{ options.confirmText ?? t("common.confirm") }}
        </VBtn>
      </VCardActions>
    </VCard>
  </VDialog>
  <ConfirmDialog ref="confirmDialogRef" />
</template>

<style lang="scss" scoped>
.file-picker-path {
  min-width: 0;
}

.file-picker-scroll {
  overflow-anchor: none;
}

.file-picker-empty {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 24rem;
  color: rgba(var(--v-theme-on-surface), 0.45);
}

.file-picker-row {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  height: 2.5rem;
  padding: 0 0.75rem;
  min-width: 0;
  white-space: nowrap;
  cursor: pointer;
  user-select: none;
}

.file-picker-row:hover {
  background: rgba(var(--v-theme-on-surface), 0.06);
}

.file-picker-row--selected {
  background: rgba(var(--v-theme-primary), 0.18);
}

.file-picker-row--disabled {
  opacity: 0.38;
  cursor: default;
}

.file-picker-row--disabled:hover {
  background: transparent;
}

.file-picker-row-name {
  overflow: hidden;
  text-overflow: ellipsis;
}
</style>
