<!--
  数据库布局页。

  打开用户数据库后显示的页面框架。
    右下角提供"日志"按钮（打开操作日志对话框）与"保存并退出"按钮（保存数据库 → 关闭数据库 → 回主页）。
  窗口关闭（X 按钮）弹出确认对话框，确认后退出应用。
  监听 Ctrl/Cmd+S 快捷键，仅保存数据库，不退出当前页面。
  顶部居中悬浮 Topbar（全局搜索 + 画布层级面包屑）。
  内部通过 <RouterView /> 渲染子页面（如画布宇宙）。
-->
<script setup lang="ts">
import { onMounted, onUnmounted, ref, useTemplateRef, watch, computed } from "vue";
import { useRoute, useRouter } from "vue-router";
import { isString } from "lodash";
import { t } from "@/i18n";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type { CloseRequestedEvent } from "@tauri-apps/api/window";
import {
  userDatabaseLifecycleClose,
  userDatabaseLifecycleSave,
  userDatabaseRegistrySet,
  userDatabaseExport,
} from "@/api";
import { LAST_SCENE_KEY } from "./Home.vue";
import LogDialog from "./DatabaseComponents/LogDialog.vue";
import ExportDialog from "./DatabaseComponents/ExportDialog.vue";
import { snackbarErrorCode, snackbarText } from "@/composables/use-snackbar";
import { loadDictionary, clearDictionary } from "@/dictionary";
import {
  consumeCanvasNavIntent,
  resolveRouteTransition,
} from "./DatabaseComponents/canvas-route-transition";
import Topbar from "./DatabaseComponents/Topbar.vue";

const router = useRouter();
const route = useRoute();

const saving = ref(false);
const closing = ref(false);
const showCloseConfirm = ref(false);
const logDialogRef = useTemplateRef<InstanceType<typeof LogDialog>>("logDialogRef");
const exportDialogRef = useTemplateRef<InstanceType<typeof ExportDialog>>("exportDialogRef");

let unlistenClose: (() => void) | undefined;

// 子路由变化时记录当前场景（画布宇宙记为空值），供下次打开数据库时恢复
watch(
  () => route.name,
  (name) => {
    const canvasId = name === "canvas" ? (route.params.canvasId as string) : "";
    void userDatabaseRegistrySet(LAST_SCENE_KEY, canvasId).catch(snackbarErrorCode);
  },
  { immediate: true },
);

// 当前画布 ID：仅画布路由下派生，画布宇宙下为 undefined
const canvasId = computed(() =>
  route.name === "canvas" ? (route.params.canvasId as string) : undefined,
);

// 子路由切换时解析过渡动画名：宇宙↔画布由路由名对比判定，画布↔画布由导航意图判定
const transitionName = ref("");
let prevRouteName: string | null = null;
watch(
  () => route.fullPath,
  () => {
    const toName = isString(route.name) ? route.name : null;
    transitionName.value = resolveRouteTransition(
      prevRouteName,
      toName,
      consumeCanvasNavIntent(),
    );
    prevRouteName = toName;
  },
  { immediate: true },
);

onMounted(async () => {
  loadDictionary().catch(snackbarErrorCode);
  unlistenClose = await getCurrentWindow().onCloseRequested(
    async (event: CloseRequestedEvent) => {
      event.preventDefault();
      showCloseConfirm.value = true;
    },
  );
  document.addEventListener("keydown", onKeydown);
});

onUnmounted(() => {
  unlistenClose?.();
  clearDictionary();
  document.removeEventListener("keydown", onKeydown);
});

/**
 * 全局快捷键监听：Ctrl/Cmd+S 触发数据库保存（仅保存，不退出）。
 * @param e 键盘事件
 */
function onKeydown(e: KeyboardEvent) {
  if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "s") {
    e.preventDefault();
    void handleSave();
  }
}

/** 保存数据库：保存期间忽略重复触发；成功显示提示，失败展示错误 */
async function handleSave() {
  if (saving.value) return;
  saving.value = true;
  try {
    await userDatabaseLifecycleSave();
    snackbarText(t("database.saved"), "success");
  } catch (error) {
    snackbarErrorCode(error);
  } finally {
    saving.value = false;
  }
}

/** 保存并退出：保存成功后关闭数据库回主页；保存失败不关闭，防止数据丢失 */
async function handleSaveAndExit() {
  saving.value = true;
  try {
    await userDatabaseLifecycleSave();
    await userDatabaseLifecycleClose();
    await router.push("/");
  } catch (error) {
    snackbarErrorCode(error);
  } finally {
    saving.value = false;
  }
}

/** 导出数据库：弹出导出对话框收集模式，调用后端导出；取消静默返回，成功提示，失败展示错误 */
async function handleExport() {
  const mode = await exportDialogRef.value?.open();
  if (!mode) return;
  try {
    const exported = await userDatabaseExport(mode);
    if (!exported) return;
    snackbarText(t("database.export.exported"), "success");
  } catch (error) {
    snackbarErrorCode(error);
  }
}

/** 确认框中"保存并关闭"的处理：保存成功后执行关闭；保存失败不关闭 */
async function handleConfirmSaveAndClose() {
  saving.value = true;
  try {
    await userDatabaseLifecycleSave();
    await doClose();
  } catch (error) {
    snackbarErrorCode(error);
  } finally {
    saving.value = false;
  }
}

async function doClose() {
  closing.value = true;
  try {
    await userDatabaseLifecycleClose();
    await getCurrentWindow().destroy();
  } catch (error) {
    snackbarErrorCode(error);
    closing.value = false;
    showCloseConfirm.value = false;
  }
}
</script>

<template>
  <div class="database-viewport">
    <Topbar :canvas-id="canvasId" />

    <RouterView v-slot="{ Component }">
      <Transition :name="transitionName" appear>
        <component :is="Component" :key="route.fullPath" />
      </Transition>
    </RouterView>

      <div class="bottom-actions">
      <div class="frosted-btns frosted-glass">
        <VBtn variant="text" @click="handleExport">
          <VIcon icon="mdi-file-export-outline" class="mr-1" />
          {{ t("database.export.button") }}
        </VBtn>
        <VBtn variant="text" @click="logDialogRef?.open()">
          <VIcon icon="mdi-history" class="mr-1" />
          {{ t("log.dialog-title") }}
        </VBtn>
        <VBtn
          variant="text"
          :loading="saving"
          @click="handleSaveAndExit"
        >
          <VIcon icon="mdi-content-save-outline" class="mr-1" />
          {{ t("database.save-and-exit") }}
        </VBtn>
      </div>
    </div>

    <VDialog v-model="showCloseConfirm" max-width="400" persistent>
      <VCard>
        <VCardTitle>{{ t("database.close-confirm.title") }}</VCardTitle>
        <VCardText>{{ t("database.close-confirm.text") }}</VCardText>
        <VCardActions>
          <VSpacer />
          <VBtn variant="text" @click="showCloseConfirm = false">
            {{ t("common.cancel") }}
          </VBtn>
          <VBtn variant="text" color="error" @click="doClose">
            {{ t("database.close-confirm.close-without-saving") }}
          </VBtn>
          <VBtn
            variant="flat"
            color="primary"
            :loading="saving"
            @click="handleConfirmSaveAndClose"
          >
            {{ t("database.close-confirm.save-and-close") }}
          </VBtn>
        </VCardActions>
      </VCard>
    </VDialog>

    <LogDialog ref="logDialogRef" />
    <ExportDialog ref="exportDialogRef" />
  </div>
</template>

<style lang="scss" scoped>
.database-viewport {
  position: relative;
  width: 100%;
  height: 100%;
  overflow: hidden;
}

.bottom-actions {
  position: absolute;
  bottom: 1rem;
  right: 1rem;
  z-index: 10;
}

.frosted-btns {
  display: flex;
  gap: 0.5rem;
  padding: 0.25rem;
  border-radius: 0.5rem;
}

// 路由切换过渡：drill-in 钻入 / drill-out 浮出 / drill-swap 淡换
// 用 filter: blur() 而不是 transform: scale() 实现"焦点拉近/失焦"的入场离场效果：
// filter 是 painted 属性、不进入 layout 流水线，不会改变子元素 getBoundingClientRect，
// 避免 vue-flow 在 transition 起始帧读错 handle.bounds（详见 CanvasView 边渲染偏移问题）。
.drill-in-enter-active,
.drill-in-leave-active,
.drill-out-enter-active,
.drill-out-leave-active,
.drill-swap-enter-active,
.drill-swap-leave-active {
  transition:
    opacity 220ms cubic-bezier(0.4, 0, 0.2, 1),
    filter 220ms cubic-bezier(0.4, 0, 0.2, 1);
}

// 并发过渡期间，离场视图脱离文档流，避免与入场视图互相挤压
.drill-in-leave-active,
.drill-out-leave-active,
.drill-swap-leave-active {
  position: absolute;
  inset: 0;
}

// 入场视图在过渡期间先以不透明背景呈现，
// 避免数据加载完成前元素近乎全透明而导致入场动画不可见
.drill-in-enter-active,
.drill-out-enter-active,
.drill-swap-enter-active {
  background-color: rgb(var(--v-theme-background));
}

// 钻入：新视图在上层从模糊到清晰淡入，旧视图失焦淡出
.drill-in-enter-active {
  position: relative;
  z-index: 2;
}
.drill-in-enter-from {
  opacity: 0;
  filter: blur(12px);
}
.drill-in-leave-to {
  opacity: 0;
  filter: blur(12px);
}

// 浮出：旧视图在上层失焦淡出，逐渐露出下层的新视图
.drill-out-leave-active {
  z-index: 2;
}
.drill-out-leave-to {
  opacity: 0;
  filter: blur(12px);
}
.drill-out-enter-from {
  opacity: 0;
  filter: blur(12px);
}

// 淡换：仅淡入淡出
.drill-swap-enter-active {
  position: relative;
  z-index: 2;
}
.drill-swap-enter-from,
.drill-swap-leave-to {
  opacity: 0;
}

@media (prefers-reduced-motion: reduce) {
  .drill-in-enter-active,
  .drill-in-leave-active,
  .drill-out-enter-active,
  .drill-out-leave-active,
  .drill-swap-enter-active,
  .drill-swap-leave-active {
    transition: none;
  }
}
</style>
