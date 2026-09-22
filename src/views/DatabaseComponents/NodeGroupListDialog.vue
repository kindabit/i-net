<!--
  节点分组列表对话框。

  以「画布套节点」形式展示一组节点（全部书签节点或具有某个标签的节点列表）：
  按节点所属画布分组，组内保持后端返回顺序，组间按画布名称升序排列。
  每个节点提供编辑与跳转两个操作：编辑使用内嵌的编辑节点对话框，保存后重新加载整个列表
  （节点可能因取消书签或移除标签而不再属于本列表），同时 emit nodeUpdated 事件供画布视图
  同步本地节点数据；取消编辑则保持列表不变；
  跳转导航到节点所在画布并定位节点，随后关闭本对话框。
  通过 defineExpose 暴露 open()；对话框从可见变为不可见时 emit closed 事件，
  供调用方（如标签云面板）在列表展示期间发生编辑后刷新自身数据。
-->
<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useRouter } from "vue-router";
import { t } from "@/i18n";
import {
  userDatabaseNodeListBookmarked,
  userDatabaseNodeTagListNodes,
} from "@/api";
import type { NodeSearchResponse } from "@/api-types";
import { snackbarErrorCode } from "@/composables/use-snackbar";
import EditNodeDialog from "./EditNodeDialog.vue";

/** 列表数据来源：全部书签节点，或具有指定标签的节点 */
type GroupSource = "bookmarked" | { tagName: string };

/** 打开参数 */
interface OpenOptions {
  /** 对话框标题 */
  title: string;
  /** 数据来源 */
  source: GroupSource;
}

/** 按画布分组的节点列表 */
interface NodeGroup {
  /** 画布 id */
  canvasId: string;
  /** 画布名称（根画布为本地化文案） */
  canvasName: string;
  /** 组内节点（保持后端返回顺序） */
  nodes: NodeSearchResponse[];
}

const emit = defineEmits<{
  closed: [];
  /** 列表内编辑节点保存成功时触发，携带节点 id 与最新值，供画布视图同步本地节点数据 */
  nodeUpdated: [{ id: string; title: string; subtitle: string; bookmarked: boolean }];
}>();

const router = useRouter();

const visible = ref(false);
const title = ref("");
const source = ref<GroupSource>("bookmarked");
const nodes = ref<NodeSearchResponse[]>([]);
const loading = ref(false);
const editNodeDialogRef = ref<InstanceType<typeof EditNodeDialog>>();

/** 按所属画布分组：组间按画布名称升序排列，组内保持后端返回顺序 */
const groups = computed<NodeGroup[]>(() => {
  const map = new Map<string, NodeGroup>();
  for (const node of nodes.value) {
    let group = map.get(node.canvas_id);
    if (!group) {
      group = {
        canvasId: node.canvas_id,
        canvasName: node.canvas_is_root ? t("database.canvas.root-canvas") : node.canvas_name,
        nodes: [],
      };
      map.set(node.canvas_id, group);
    }
    group.nodes.push(node);
  }
  return [...map.values()].sort((a, b) => a.canvasName.localeCompare(b.canvasName));
});

/** 空态文案：按数据来源区分书签列表与标签列表 */
const emptyText = computed(() =>
  source.value === "bookmarked"
    ? t("database.node-group.empty-bookmarked")
    : t("database.node-group.empty-tag"),
);

/**
 * 打开对话框并加载节点列表。
 * @param options 对话框标题与数据来源
 * @returns 无返回值
 */
function open(options: OpenOptions): void {
  visible.value = true;
  title.value = options.title;
  source.value = options.source;
  nodes.value = [];
  load();
}

/**
 * 按当前数据来源加载节点列表；加载失败时展示错误并清空列表。
 * 无输入参数，无返回值。
 */
async function load(): Promise<void> {
  loading.value = true;
  try {
    nodes.value =
      source.value === "bookmarked"
        ? await userDatabaseNodeListBookmarked()
        : await userDatabaseNodeTagListNodes(source.value.tagName);
  } catch (e) {
    snackbarErrorCode(e);
    nodes.value = [];
  } finally {
    loading.value = false;
  }
}

/**
 * 编辑列表中的节点：打开编辑对话框，用户保存后重新加载整个列表，取消编辑则不刷新。
 * @param node 列表项
 * @returns 无返回值
 */
async function onEdit(node: NodeSearchResponse): Promise<void> {
  const result = await editNodeDialogRef.value?.open({
    id: node.id,
    title: node.title,
    subtitle: node.subtitle,
    bookmarked: node.bookmarked,
  });
  if (!result) return;
  emit("nodeUpdated", {
    id: node.id,
    title: result.title,
    subtitle: result.subtitle,
    bookmarked: result.bookmarked,
  });
  await load();
}

/**
 * 跳转到节点的画布并居中定位该节点，随后关闭对话框。
 * @param node 列表项
 * @returns 无返回值
 */
async function onJump(node: NodeSearchResponse): Promise<void> {
  await router.push({
    name: "canvas",
    params: { canvasId: node.canvas_id },
    query: { nodeId: node.id },
  });
  visible.value = false;
}

// 对话框从可见变为不可见时通知调用方（关闭按钮、遮罩、跳转等任何关闭路径）
watch(visible, (value) => {
  if (!value) emit("closed");
});

defineExpose({ open });
</script>

<template>
  <VDialog v-model="visible" width="40rem" max-width="90vw">
    <VCard>
      <VCardTitle class="node-group-title-row">
        <span class="node-group-title-text">{{ title }}</span>
        <VBtn
          icon="mdi-close"
          size="x-small"
          variant="text"
          density="comfortable"
          @click="visible = false"
        />
      </VCardTitle>
      <VCardText class="node-group-body">
        <div v-if="loading" class="d-flex justify-center py-8">
          <VProgressCircular indeterminate color="primary" />
        </div>
        <div v-else-if="groups.length === 0" class="node-group-empty">
          {{ emptyText }}
        </div>
        <div v-else class="node-group-list">
          <div v-for="group in groups" :key="group.canvasId" class="node-group">
            <div class="node-group-canvas-name">{{ group.canvasName }}</div>
            <div class="node-group-grid">
              <div v-for="node in group.nodes" :key="node.id" class="node-group-item">
                <div class="node-group-item-text">
                  <div class="node-group-item-title">{{ node.title }}</div>
                  <div class="node-group-item-subtitle">{{ node.subtitle }}</div>
                </div>
                <div class="node-group-item-actions">
                  <VBtn
                    icon="mdi-pencil-outline"
                    size="x-small"
                    variant="text"
                    density="comfortable"
                    :title="t('database.canvas.edit-node')"
                    @click="onEdit(node)"
                  />
                  <VBtn
                    icon="mdi-open-in-new"
                    size="x-small"
                    variant="text"
                    density="comfortable"
                    :title="t('database.node-group.go-to-node')"
                    @click="onJump(node)"
                  />
                </div>
              </div>
            </div>
          </div>
        </div>
      </VCardText>
    </VCard>
    <EditNodeDialog ref="editNodeDialogRef" />
  </VDialog>
</template>

<style lang="scss" scoped>
.node-group-title-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.5rem;
}

.node-group-title-text {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.node-group-body {
  max-height: 60vh;
  overflow-y: auto;
}

.node-group-empty {
  padding: 2rem 0;
  text-align: center;
  font-size: 0.875rem;
  opacity: 0.6;
}

.node-group-list {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.node-group {
  padding: 0.75rem;
  border: 1px solid rgba(var(--v-theme-on-surface), 0.12);
  border-radius: 0.5rem;
}

.node-group-canvas-name {
  margin-bottom: 0.5rem;
  font-size: 0.875rem;
  font-weight: 700;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.node-group-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(12rem, 1fr));
  gap: 0.5rem;
}

.node-group-item {
  display: flex;
  align-items: center;
  gap: 0.25rem;
  min-width: 0;
  padding: 0.375rem 0.5rem;
  border: 1px solid rgba(var(--v-theme-on-surface), 0.12);
  border-radius: 0.375rem;
}

.node-group-item-text {
  flex: 1;
  min-width: 0;
}

.node-group-item-title {
  font-size: 0.875rem;
  font-weight: 700;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.node-group-item-subtitle {
  min-height: 1em;
  font-size: 0.75rem;
  opacity: 0.6;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.node-group-item-actions {
  display: flex;
  flex-shrink: 0;
  gap: 0.125rem;
}
</style>
