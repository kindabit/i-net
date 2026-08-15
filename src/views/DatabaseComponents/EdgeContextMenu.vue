<!--
  边右键菜单组件。

  在画布中右键点击边时显示，提供编辑和删除边的操作。
  不接收 props，通过 defineExpose 暴露 open 方法，通过 defineEmits 发出 edit 事件。
  CanvasView 只需在 @edge-context-menu 中调用 open() 并监听 edit 事件即可。
-->
<script setup lang="ts">
import { ref, onMounted, onUnmounted } from "vue";
import type { Edge as VFEdge } from "@vue-flow/core";
import { t } from "@/i18n";
import { userDatabaseEdgeDelete } from "@/api";
import { snackbarErrorCode } from "@/composables/use-snackbar";

const emit = defineEmits<{ edit: [id: string] }>();

const visible = ref(false);
const edgeId = ref<string | null>(null);
const position = ref({ x: 0, y: 0 });
let edges: { value: VFEdge[] } | null = null;

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

async function onDelete() {
  const id = edgeId.value;
  if (!id) return;
  try {
    await userDatabaseEdgeDelete(id);
    if (edges) {
      edges.value = edges.value.filter((e) => e.id !== id);
    }
    close();
  } catch (e) {
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
