<!--
  标签云悬浮面板。

  悬浮在画布左上角悬浮菜单右侧，展示当前用户数据库中的全部活跃标签，
  标签字号按使用该标签的节点数量分档显示（标签云形态）；点击标签打开节点分组列表对话框，
  对话框关闭后重新加载标签，以反映编辑期间可能发生的标签变更。
  通过 defineExpose 暴露 open / close / toggle / visible 方法。
-->
<script setup lang="ts">
import { computed, ref } from "vue";
import { t } from "@/i18n";
import { userDatabaseNodeTagList } from "@/api";
import type { NodeTagVO } from "@/api-types";
import { snackbarErrorCode } from "@/composables/use-snackbar";
import NodeGroupListDialog from "./NodeGroupListDialog.vue";

/** 标签字号的 5 个档位（按使用节点数量与最大数量之比分档） */
const TAG_FONT_SIZES = ["0.75rem", "0.875rem", "1rem", "1.1875rem", "1.375rem"];

const visible = ref(false);
const tags = ref<NodeTagVO[]>([]);
const groupListDialogRef = ref<InstanceType<typeof NodeGroupListDialog>>();

const emit = defineEmits<{
  /** 标签节点列表对话框内编辑保存时透传的事件，供画布视图同步本地节点数据 */
  nodeUpdated: [{ id: string; title: string; subtitle: string; bookmarked: boolean }];
}>();

/** 标签列表中最大的使用节点数量（空列表时为 0） */
const maxCount = computed(() =>
  tags.value.reduce((max, tag) => Math.max(max, tag.node_count), 0),
);

/**
 * 按使用节点数量在最大数量中的比例把标签映射到 5 个字号档位。
 * @param count 标签的使用节点数量
 * @returns 该标签使用的字号（rem 值）
 */
function tagFontSize(count: number): string {
  if (maxCount.value <= 0) return TAG_FONT_SIZES[0];
  const index = Math.min(
    TAG_FONT_SIZES.length - 1,
    Math.floor((count / maxCount.value) * TAG_FONT_SIZES.length),
  );
  return TAG_FONT_SIZES[index];
}

/**
 * 从后端加载全部活跃标签并更新面板数据。
 * 无输入参数，无返回值；加载失败时通过全局 snackbar 展示错误。
 */
async function loadTags() {
  try {
    tags.value = await userDatabaseNodeTagList();
  } catch (e) {
    snackbarErrorCode(e);
  }
}

/**
 * 打开标签云面板并加载标签列表。
 * 无输入参数，无返回值。
 */
function open() {
  visible.value = true;
  loadTags();
}

/**
 * 关闭标签云面板。
 * 无输入参数，无返回值。
 */
function close() {
  visible.value = false;
}

/**
 * 切换标签云面板的显示状态；展开时重新加载标签列表。
 * 无输入参数，无返回值。
 */
function toggle() {
  visible.value = !visible.value;
  if (visible.value) {
    loadTags();
  }
}

/**
 * 点击标签：打开节点分组列表对话框，展示使用该标签的节点。
 * @param tag 被点击的标签
 * @returns 无返回值
 */
function onTagClick(tag: NodeTagVO) {
  groupListDialogRef.value?.open({
    title: t("database.tag-cloud.dialog-title", { tag: tag.name }),
    source: { tagName: tag.name },
  });
}

defineExpose({
  open,
  close,
  toggle,
  visible,
});
</script>

<template>
  <Transition name="tag-cloud-panel">
    <div
      v-if="visible"
      v-click-outside="close"
      class="tag-cloud-panel frosted-glass"
    >
      <div class="tag-cloud-header">
        <span class="tag-cloud-title">
          {{ t("database.canvas.tag-cloud") }}
        </span>
        <VBtn
          icon="mdi-close"
          size="x-small"
          variant="text"
          density="comfortable"
          @click="close"
        />
      </div>
      <div class="tag-cloud-body">
        <div v-if="tags.length === 0" class="tag-cloud-empty">
          {{ t("database.tag-cloud.empty") }}
        </div>
        <div v-else class="tag-cloud-list">
          <button
            v-for="tag in tags"
            :key="tag.name"
            type="button"
            class="tag-cloud-tag"
            :style="{ fontSize: tagFontSize(tag.node_count) }"
            @click="onTagClick(tag)"
          >
            {{ tag.name }}
          </button>
        </div>
      </div>
    </div>
  </Transition>
  <NodeGroupListDialog ref="groupListDialogRef" @closed="loadTags" @node-updated="(p) => emit('nodeUpdated', p)" />
</template>

<style lang="scss" scoped>
.tag-cloud-panel {
  position: absolute;
  top: 0.75rem;
  left: 4.25rem;
  z-index: 10;
  min-width: 12rem;
  max-width: 24rem;
  max-height: 60vh;
  display: flex;
  flex-direction: column;
  border-radius: 0.5rem;
  overflow: hidden;
}

.tag-cloud-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0.5rem 0.75rem;
  flex-shrink: 0;
}

.tag-cloud-title {
  font-size: 0.875rem;
  font-weight: 500;
  white-space: nowrap;
}

.tag-cloud-body {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 0.5rem 0.75rem;
}

.tag-cloud-empty {
  padding: 1rem 0;
  text-align: center;
  font-size: 0.8rem;
  opacity: 0.5;
}

.tag-cloud-list {
  display: flex;
  flex-wrap: wrap;
  align-items: baseline;
  gap: 0.5rem;
}

.tag-cloud-tag {
  padding: 0;
  border: none;
  background: none;
  color: rgb(var(--v-theme-primary));
  font-family: inherit;
  line-height: 1.2;
  cursor: pointer;

  &:hover {
    text-decoration: underline;
  }
}

/* 面板展开/收起动画 */
.tag-cloud-panel-enter-active,
.tag-cloud-panel-leave-active {
  transition: opacity 0.2s ease, transform 0.2s ease;
  overflow: hidden;
}

.tag-cloud-panel-enter-from,
.tag-cloud-panel-leave-to {
  opacity: 0;
  transform: translateX(-0.5rem);
}
</style>
