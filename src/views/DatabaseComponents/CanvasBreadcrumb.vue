<!--
  数据库顶部面包屑组件。

  组件自行加载画布列表、组装层级链并上报数据异常。
  在 DatabaseView 顶部浮条中展示从画布宇宙到当前画布的层级导航路径。
  位于画布宇宙时仅显示静态"画布宇宙"文本。
  中间层级过多时折叠为省略号菜单，根画布与上一级画布固定显示。
-->
<script setup lang="ts">
import { ref, computed, watch, type Ref } from "vue";
import { useRouter, type RouteLocationRaw } from "vue-router";
import { get } from "lodash";
import { t } from "@/i18n";
import { userDatabaseCanvasList } from "@/api";
import { snackbarErrorCode, snackbarText } from "@/composables/use-snackbar";
import type { Canvas } from "@/api-types";
import { buildCanvasChain, collapseChain, type CanvasChainResult } from "./canvas-breadcrumb";
import { setCanvasNavIntent } from "./canvas-route-transition";

const props = defineProps<{
  canvasId?: string;
}>();

const router = useRouter();

const chainResult: Ref<CanvasChainResult> = ref({ status: "not-found" });

/**
   * 加载画布层级链：有 canvasId 时调 API 构建链，无 canvasId 时重置状态。
   * 输入：canvasId 当前画布 ID，undefined 表示位于画布宇宙。
   */
async function loadChain(canvasId: string | undefined) {
  if (!canvasId) {
    chainResult.value = { status: "not-found" };
    return;
  }
  const targetId = canvasId;
  try {
    const canvases = await userDatabaseCanvasList(false);
    if (props.canvasId !== targetId) return;
    chainResult.value = buildCanvasChain(canvases, targetId);
    if (chainResult.value.status === "cycle") {
      snackbarText(t("database.canvas.cycle-detected"));
    }
  } catch (e) {
    snackbarErrorCode(e);
  }
}

watch(() => props.canvasId, (id) => { void loadChain(id); }, { immediate: true });

type Crumb = {
  title: string;
  to?: RouteLocationRaw;
  disabled?: boolean;
  ellipsis?: boolean;
  hidden?: Canvas[];
};

function displayName(c: Canvas): string {
  if (c.parent_id === null) {
    return t("database.canvas.root-canvas");
  }
  return c.name;
}

/**
 * 跳转到指定父级画布（省略号菜单项）。
 * 输入：canvasId 目标画布 ID。
 */
function navigate(canvasId: string) {
  setCanvasNavIntent("drill-out");
  router.push({ name: "canvas", params: { canvasId } });
}

/**
 * 面包屑项点击处理：跳往父级画布的项记录浮出意图，供路由切换动画定向。
 * 输入：item 被点击的面包屑项。
 */
function onCrumbClick(item: Crumb) {
  if (get(item.to, "name") === "canvas") {
    setCanvasNavIntent("drill-out");
  }
}

const crumbs = computed<Crumb[]>(() => {
  if (!props.canvasId) {
    return [{ title: t("database.canvas-universe.title"), disabled: true }];
  }
  const items: Crumb[] = [
    { title: t("database.canvas-universe.title"), to: { name: "canvas-universe" } },
  ];
  if (chainResult.value.status === "ok") {
    const collapsed = collapseChain(chainResult.value.chain);
    if (collapsed.collapsed) {
      items.push({
        title: displayName(collapsed.root),
        to: { name: "canvas", params: { canvasId: collapsed.root.id } },
      });
      items.push({ title: "\u2026", ellipsis: true, hidden: collapsed.hidden });
      items.push({
        title: displayName(collapsed.parent),
        to: { name: "canvas", params: { canvasId: collapsed.parent.id } },
      });
      items.push({ title: displayName(collapsed.current), disabled: true });
    } else {
      for (const c of collapsed.visible.slice(0, -1)) {
        items.push({
          title: displayName(c),
          to: { name: "canvas", params: { canvasId: c.id } },
        });
      }
      const last = collapsed.visible[collapsed.visible.length - 1];
      items.push({ title: displayName(last), disabled: true });
    }
  }
  return items;
});
</script>

<template>
  <VBreadcrumbs :items="crumbs" density="compact" class="canvas-breadcrumb">
    <template #divider>
      <VIcon icon="mdi-chevron-right" size="16" />
    </template>
    <template #item="{ item }: { item: Crumb }">
      <VMenu v-if="item.ellipsis" location="bottom">
        <template #activator="{ props: menuProps }">
          <VBreadcrumbsItem v-bind="menuProps" :title="item.title" style="cursor: pointer" />
        </template>
        <VList density="compact">
          <VListItem
            v-for="c in item.hidden"
            :key="c.id"
            :title="displayName(c)"
            @click="navigate(c.id)"
          />
        </VList>
      </VMenu>
      <VBreadcrumbsItem
        v-else
        :title="item.title"
        :to="item.to"
        :disabled="item.disabled"
        max-width="160"
        @click="onCrumbClick(item)"
      />
    </template>
  </VBreadcrumbs>
</template>

<style lang="scss" scoped>
.canvas-breadcrumb {
  margin: 0;
  padding: 0.25rem 0.5rem;
}
</style>
