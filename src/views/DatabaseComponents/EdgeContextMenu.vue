<!--
  边右键菜单组件。

  在画布中右键点击边时显示，提供编辑和删除边的操作。
  不接收 props，通过 defineExpose 暴露 open 方法，通过 defineEmits 发出 edit 事件。
  CanvasView 只需在 @edge-context-menu 中调用 open() 并监听 edit 事件即可。

  删除边复用 use-edge-delete 的断连确认流程（先以未确认姿态调用，后端提示会断开
  影子节点在子画布内的关联节点则确认后重试）；本组件负责确认前收起菜单、删除成功后
  把边从本地边集移除。
-->
<script setup lang="ts">
import { ref, onMounted, onUnmounted } from "vue";
import type { Edge as VFEdge } from "@vue-flow/core";
import { t } from "@/i18n";
import { deleteEdgeWithDisconnectConfirm } from "@/composables/use-edge-delete";
import ConfirmDialog from "@/components/ConfirmDialog.vue";

const emit = defineEmits<{ edit: [id: string] }>();

const visible = ref(false);
const edgeId = ref<string | null>(null);
const position = ref({ x: 0, y: 0 });
let edges: { value: VFEdge[] } | null = null;

/** 确认对话框实例引用 */
const confirmDialogRef = ref<InstanceType<typeof ConfirmDialog> | null>(null);

function close() {
  visible.value = false;
  edgeId.value = null;
}

function onEsc(e: KeyboardEvent) {
  if (e.key === "Escape" && visible.value) {
    close();
  }
}

onMounted(() => {
  document.addEventListener("keydown", onEsc);
});

onUnmounted(() => {
  document.removeEventListener("keydown", onEsc);
});

/** 发出 edit 事件并关闭菜单 */
function onEdit(): void {
  if (!edgeId.value) return;
  emit('edit', edgeId.value);
  close();
}

/**
 * 从本地 edges 引用中过滤掉指定 id 的边。
 * @param id 要移除的边 id
 */
function removeEdgeLocally(id: string) {
  if (!edges) return;
  edges.value = edges.value.filter((e) => e.id !== id);
}

/**
 * 删除边：断连确认与重试由共用流程处理，本组件在请求确认前收起菜单，
 * 删除成功后把边从本地边集移除并关闭菜单。
 */
async function onDelete() {
  const id = edgeId.value;
  if (!id) return;
  // 断连确认前先收起菜单，让确认对话框浮在画布之上而不被菜单遮挡
  const deleted = await deleteEdgeWithDisconnectConfirm(id, async (nodes) => {
    close();
    const confirmed = await confirmDialogRef.value?.open({
      title: t("database.canvas.delete-edge-disconnect-title"),
      text: t("database.canvas.delete-edge-disconnect-text", {
        nodes: nodes.join(t("database.canvas.delete-edge-disconnect-separator")),
      }),
      confirmText: t("database.canvas.delete-edge"),
      confirmColor: "error",
    });
    return !!confirmed;
  });
  if (!deleted) return;
  removeEdgeLocally(id);
  close();
}

defineExpose({
  open(
    id: string,
    pos: { x: number; y: number },
    edgesRef: { value: VFEdge[] },
  ) {
    edgeId.value = id;
    position.value = pos;
    edges = edgesRef;
    visible.value = true;
  },
});
</script>

<template>
  <Teleport to="body">
    <div v-if="visible" class="edge-context-menu-backdrop" @click="close" />
    <div
      v-if="visible"
      class="edge-context-menu"
      :style="{ left: `${position.x}px`, top: `${position.y}px` }"
    >
      <VList density="compact">
        <VListItem
          :title="t('database.canvas.edit-edge')"
          @click="onEdit"
        />
        <VListItem
          :title="t('database.canvas.delete-edge')"
          @click="onDelete"
        />
      </VList>
    </div>
  </Teleport>
  <ConfirmDialog ref="confirmDialogRef" />
</template>

<style lang="scss" scoped>
.edge-context-menu-backdrop {
  position: fixed;
  inset: 0;
  z-index: 999;
}

.edge-context-menu {
  position: fixed;
  z-index: 1000;
  min-width: 7.5rem;
  background-color: rgb(var(--v-theme-surface));
  border-radius: 0.25rem;
  box-shadow: 0 0.25rem 1rem rgba(0, 0, 0, 0.2);
  padding: 0.25rem 0;
}
</style>
