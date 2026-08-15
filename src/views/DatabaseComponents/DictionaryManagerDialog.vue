<!--
  字典管理对话框。

  提供字典条目的树形编辑界面，支持增删改移。
  打开时从全局字典状态深拷贝本地编辑副本，关闭时若存在未保存修改则拦截确认。
  保存时将校验后的副本交还全局字典状态模块，由其统一写库并刷新全局状态。
-->
<script setup lang="ts">
import { ref } from "vue";
import { t } from "@/i18n";
import { snackbarErrorCode, snackbarText } from "@/composables/use-snackbar";
import ConfirmDialog from "@/components/ConfirmDialog.vue";
import {
  cloneDictionaryTree,
  saveDictionaryTree,
  type DictionaryTreeNode,
} from "@/dictionary";

const dialog = ref(false);
const forest = ref<DictionaryTreeNode[]>([]);
const saving = ref(false);
const confirmingClose = ref(false);
const confirmDialogRef = ref<InstanceType<typeof ConfirmDialog>>();
let snapshot = "";

/**
 * VTreeview item-title 取值：返回节点显示文本。
 * @param node 字典树节点（VTreeview 回调传入的原始 item）
 * @returns 节点值
 */
function nodeTitle(node: unknown): string {
  return (node as DictionaryTreeNode).entry.value;
}

/**
 * VTreeview item-value 取值：返回节点唯一标识。
 * @param node 字典树节点（VTreeview 回调传入的原始 item）
 * @returns 节点 id
 */
function nodeValue(node: unknown): string {
  return (node as DictionaryTreeNode).entry.id;
}

/** 比较当前 forest 与快照是否不同。 */
function isDirty(): boolean {
  return JSON.stringify(forest.value) !== snapshot;
}

/**
 * DFS 查找指定 id 节点所在父数组及下标。
 * @param id 目标节点 entry.id
 * @param nodes 搜索的节点数组，默认从 forest 根开始
 * @returns 找到返回 { parentArr, index }，未找到返回 null
 */
function findNodeInfo(
  id: string,
  nodes: DictionaryTreeNode[] = forest.value,
): { parentArr: DictionaryTreeNode[]; index: number } | null {
  for (let i = 0; i < nodes.length; i++) {
    if (nodes[i].entry.id === id) return { parentArr: nodes, index: i };
    const found = findNodeInfo(id, nodes[i].children);
    if (found) return found;
  }
  return null;
}

function open() {
  dialog.value = true;
  forest.value = cloneDictionaryTree();
  snapshot = JSON.stringify(forest.value);
}

function moveUp(id: string) {
  const info = findNodeInfo(id);
  if (!info || info.index === 0) return;
  const arr = info.parentArr;
  [arr[info.index], arr[info.index - 1]] = [
    arr[info.index - 1],
    arr[info.index],
  ];
}

function moveDown(id: string) {
  const info = findNodeInfo(id);
  if (!info || info.index >= info.parentArr.length - 1) return;
  const arr = info.parentArr;
  [arr[info.index], arr[info.index + 1]] = [
    arr[info.index + 1],
    arr[info.index],
  ];
}

function addChild(id: string) {
  const info = findNodeInfo(id);
  if (!info) return;
  info.parentArr[info.index].children.push({
    entry: {
      id: crypto.randomUUID(),
      parent_id: null,
      value: "",
      order: 0,
    },
    children: [],
  });
}

function addRoot() {
  forest.value.push({
    entry: {
      id: crypto.randomUUID(),
      parent_id: null,
      value: "",
      order: 0,
    },
    children: [],
  });
}

async function deleteEntry(id: string) {
  const info = findNodeInfo(id);
  if (!info) return;
  const node = info.parentArr[info.index];
  const hasChildren = node.children.length > 0;
  const confirmed = await confirmDialogRef.value?.open({
    title: t("database.dictionary.delete-entry"),
    text: hasChildren
      ? t("database.dictionary.delete-entry-text-with-children", {
          value: node.entry.value,
        })
      : t("database.dictionary.delete-entry-text", {
          value: node.entry.value,
        }),
    confirmColor: "error",
  });
  if (!confirmed) return;
  info.parentArr.splice(info.index, 1);
}

/** DFS 遍历 forest 检查是否有空 value。 */
function hasEmptyValue(nodes: DictionaryTreeNode[]): boolean {
  for (const node of nodes) {
    if (node.entry.value.trim() === "") return true;
    if (hasEmptyValue(node.children)) return true;
  }
  return false;
}

async function save() {
  if (hasEmptyValue(forest.value)) {
    snackbarText(t("database.dictionary.value-required"), "warning");
    return;
  }

  saving.value = true;
  try {
    await saveDictionaryTree(forest.value);
    snackbarText(t("database.dictionary.saved"), "success");
    dialog.value = false;
  } catch (e) {
    snackbarErrorCode(e);
  } finally {
    saving.value = false;
  }
}

/**
 * 请求关闭对话框：无未保存修改直接关闭，否则先弹确认，确认后才关闭。
 * 对话框为 persistent，仅取消按钮经此入口关闭。
 * @returns 无返回值
 */
async function requestClose(): Promise<void> {
  if (confirmingClose.value) return;
  if (!isDirty()) {
    dialog.value = false;
    return;
  }
  confirmingClose.value = true;
  try {
    const confirmed = await confirmDialogRef.value?.open({
      title: t("database.canvas.unsaved-changes-title"),
      text: t("database.canvas.unsaved-changes-text"),
      confirmColor: "error",
    });
    if (confirmed) {
      dialog.value = false;
    }
  } finally {
    confirmingClose.value = false;
  }
}

defineExpose({ open });
</script>

<template>
  <VDialog v-model="dialog" max-width="560" persistent>
    <VCard>
      <VCardTitle>{{ t("database.dictionary.manager-title") }}</VCardTitle>
      <VCardText class="dict-card-text">
        <VTreeview
          v-if="forest.length > 0"
          :items="forest"
          :item-title="nodeTitle"
          :item-value="nodeValue"
          item-children="children"
          density="compact"
          class="dict-tree"
        >
          <template #toggle="{ props: toggleProps, internalItem }">
            <VBtn
              v-if="(internalItem.raw as DictionaryTreeNode).children.length > 0"
              v-bind="toggleProps"
            />
            <div v-else class="v-treeview-item__level" />
          </template>
          <template #title="{ internalItem }">
            <VTextField
              :model-value="(internalItem.raw as DictionaryTreeNode).entry.value"
              variant="underlined"
              density="compact"
              hide-details
              class="dict-value-input"
              @update:model-value="(v: string) => ((internalItem.raw as DictionaryTreeNode).entry.value = v)"
            />
          </template>
          <template #append="{ internalItem }">
            <VBtn
              icon="mdi-arrow-up"
              variant="text"
              density="compact"
              size="small"
              :disabled="(findNodeInfo((internalItem.raw as DictionaryTreeNode).entry.id)?.index ?? 0) === 0"
              @click="moveUp((internalItem.raw as DictionaryTreeNode).entry.id)"
            />
            <VBtn
              icon="mdi-arrow-down"
              variant="text"
              density="compact"
              size="small"
              :disabled="(() => {
                const info = findNodeInfo((internalItem.raw as DictionaryTreeNode).entry.id);
                return !info ? true : info.index >= info.parentArr.length - 1;
              })()"
              @click="moveDown((internalItem.raw as DictionaryTreeNode).entry.id)"
            />
            <VBtn
              icon="mdi-plus"
              variant="text"
              density="compact"
              size="small"
              :title="t('database.dictionary.add-child')"
              @click="addChild((internalItem.raw as DictionaryTreeNode).entry.id)"
            />
            <VBtn
              icon="mdi-delete-outline"
              variant="text"
              density="compact"
              size="small"
              color="error"
              :title="t('database.dictionary.delete-entry')"
              @click="deleteEntry((internalItem.raw as DictionaryTreeNode).entry.id)"
            />
          </template>
        </VTreeview>
        <div
          v-if="forest.length === 0"
          class="dict-empty-hint"
        >
          {{ t("database.dictionary.empty-hint") }}
        </div>
      </VCardText>
      <VCardActions>
        <VBtn
          variant="text"
          prepend-icon="mdi-plus"
          class="mr-auto"
          @click="addRoot"
        >
          {{ t("database.dictionary.add-root") }}
        </VBtn>
        <VBtn variant="text" @click="requestClose">
          {{ t("common.cancel") }}
        </VBtn>
        <VBtn
          color="primary"
          :disabled="!isDirty()"
          :loading="saving"
          @click="save"
        >
          {{ t("common.save") }}
        </VBtn>
      </VCardActions>
    </VCard>
    <ConfirmDialog ref="confirmDialogRef" />
  </VDialog>
</template>

<style lang="scss" scoped>
.dict-card-text {
  max-height: 60vh;
  overflow-y: auto;
}

.dict-empty-hint {
  text-align: center;
  opacity: 0.6;
  color: rgb(var(--v-theme-on-surface));
  padding-top: 2rem;
  padding-bottom: 2rem;
}

.dict-value-input {
  flex: 1;
}

.mr-auto {
  margin-right: auto;
}
</style>
