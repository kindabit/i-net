<!--
  边右键菜单组件。

  在画布中右键点击边时显示，提供编辑和删除边的操作。
  不接收 props，通过 defineExpose 暴露 open 方法，通过 defineEmits 发出 edit 事件。
  CanvasView 只需在 @edge-context-menu 中调用 open() 并监听 edit 事件即可。

  删除边流程（影子节点联动）：
  1. 先以未确认姿态调用后端；若删除会断开影子节点在子画布内的关联节点，后端返回
     EdgeDeleteDisconnectsNodes 错误（data.nodes 为受影响节点标题列表）；
  2. 此时关闭右键菜单并弹出确认对话框，用户确认后再以 confirmed=true 重试；
  3. 确认后调用失败或其它错误均交给 snackbarErrorCode 统一展示。
-->
<script setup lang="ts">
import { ref, onMounted, onUnmounted } from "vue";
import type { Edge as VFEdge } from "@vue-flow/core";
import { t } from "@/i18n";
import { userDatabaseEdgeDelete } from "@/api";
import { snackbarErrorCode } from "@/composables/use-snackbar";
import { isErrorCode } from "@/error-code";
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
 * 删除边：先以未确认姿态调用，若后端提示会断开影子节点的关联节点，
 * 则关闭右键菜单并弹出确认对话框，用户确认后再以 confirmed=true 重试。
 */
async function onDelete() {
  const id = edgeId.value;
  if (!id) return;
  try {
    await userDatabaseEdgeDelete(id, false);
    removeEdgeLocally(id);
    close();
    return;
  } catch (e) {
    if (isErrorCode(e, "EdgeDeleteDisconnectsNodes")) {
      close();
      const rawNodes = e.data?.nodes;
      const nodes: string[] = Array.isArray(rawNodes)
        ? rawNodes.map(String)
        : [];
      const separator = t("database.canvas.delete-edge-disconnect-separator");
      const confirmed = await confirmDialogRef.value?.open({
        title: t("database.canvas.delete-edge-disconnect-title"),
        text: t("database.canvas.delete-edge-disconnect-text", {
          nodes: nodes.join(separator),
        }),
        confirmText: t("database.canvas.delete-edge"),
        confirmColor: "error",
      });
      if (!confirmed) return;
      try {
        await userDatabaseEdgeDelete(id, true);
        removeEdgeLocally(id);
      } catch (e2) {
        snackbarErrorCode(e2);
      }
      return;
    }
    snackbarErrorCode(e);
  }
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
