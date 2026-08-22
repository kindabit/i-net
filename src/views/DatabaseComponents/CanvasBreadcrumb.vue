<!--
  数据库顶部面包屑组件。

  组件自行加载画布列表、组装层级链并上报数据异常。
  在 DatabaseView 顶部浮条中展示从画布宇宙到当前画布的层级导航路径。
  位于画布宇宙时仅显示静态"画布宇宙"文本。
  中间层级过多时折叠为省略号菜单，根画布与上一级画布固定显示。
  在跨画布迁移（按住 Alt 拖拽）时根据节点集合法性在可跳转祖先片段上显示"允许/禁止"落点高亮。
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
import nodeMoveAndRelocate from "@/composables/use-node-move-and-relocate";

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
  /** 可跳转祖先画布片段对应的画布 id，供节点迁移目标查询使用 */
  canvasId?: string;
  /** 可跳转祖先画布片段对应的画布显示名，供节点迁移目标查询使用 */
  canvasName?: string;
};

/** 面包屑根元素引用，用于 queryBreadcrumbAtPosition 的命中范围判定 */
const breadcrumbRef = ref<{ $el: HTMLElement } | null>(null);

function displayName(c: Canvas): string {
  if (c.parent_id === null) {
    return t("database.canvas.root-canvas");
  }
  return c.name;
}

/**
 * 构造可跳转的祖先画布片段：附带 canvasId/canvasName，使其可作为节点迁移目标。
 * 输入：c 目标画布。
 * 返回：面包屑项。
 */
function canvasCrumb(c: Canvas): Crumb {
  return {
    title: displayName(c),
    to: { name: "canvas", params: { canvasId: c.id } },
    canvasId: c.id,
    canvasName: displayName(c),
  };
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

/** 迁移落点面包屑片段对应的画布 id；非 relocate 模式或目标不是面包屑片段时为 null */
const dropTargetCanvasId = computed(() => {
  if (nodeMoveAndRelocate.mode.value !== "relocate") return null;
  const target = nodeMoveAndRelocate.relocatingTarget.value;
  return target?.type === "breadcrumb-segment" ? target.canvasId : null;
});

/** 当前拖拽节点集是否可合法迁移（决定落点高亮为"允许"还是"禁止"） */
const dropRelocateAllowed = computed(
  () => nodeMoveAndRelocate.nodeSetRelocatingLegality.value === "legal",
);

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
      items.push(canvasCrumb(collapsed.root));
      items.push({ title: "…", ellipsis: true, hidden: collapsed.hidden });
      items.push(canvasCrumb(collapsed.parent));
      items.push({ title: displayName(collapsed.current), disabled: true });
    } else {
      for (const c of collapsed.visible.slice(0, -1)) {
        items.push(canvasCrumb(c));
      }
      const last = collapsed.visible[collapsed.visible.length - 1];
      items.push({ title: displayName(last), disabled: true });
    }
  }
  return items;
});

/**
 * 查询指定屏幕坐标位置下的面包屑片段对应的迁移目标画布。
 *
 * 仅"可跳转的祖先画布片段"（渲染时根元素带 data-canvas-id）是合法的迁移目标；
 * 画布宇宙片段、省略号片段、当前画布片段以及面包屑以外的位置均返回 null。
 * 省略号菜单项经 teleport 渲染在面包屑 DOM 之外，同样不会命中。
 * @param position 屏幕坐标（clientX / clientY）
 * @returns 目标画布的 id 与名称，无合法命中时返回 null
 */
function queryBreadcrumbAtPosition(position: { x: number, y: number }): { canvasId: string, canvasName: string } | null {
  const root = breadcrumbRef.value?.$el;
  if (!root) return null;
  const hit = document.elementFromPoint(position.x, position.y);
  if (!(hit instanceof Element) || !root.contains(hit)) return null;
  const itemEl = hit.closest("[data-canvas-id]");
  if (!(itemEl instanceof HTMLElement) || !root.contains(itemEl)) return null;
  const canvasId = itemEl.dataset.canvasId;
  const canvasName = itemEl.dataset.canvasName;
  if (!canvasId || !canvasName) return null;
  return { canvasId, canvasName };
}
nodeMoveAndRelocate.setQueryBreadcrumbAtPosition(queryBreadcrumbAtPosition);
</script>

<template>
  <VBreadcrumbs ref="breadcrumbRef" :items="crumbs" density="compact" class="canvas-breadcrumb">
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
        :data-canvas-id="item.canvasId"
        :data-canvas-name="item.canvasName"
        :class="{
          'canvas-breadcrumb-item--drop-allow': item.canvasId === dropTargetCanvasId && dropRelocateAllowed,
          'canvas-breadcrumb-item--drop-forbid': item.canvasId === dropTargetCanvasId && !dropRelocateAllowed,
        }"
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

.canvas-breadcrumb-item--drop-allow {
  background-color: rgba(var(--v-theme-success), 0.18);
  border-radius: 0.25rem;
}

.canvas-breadcrumb-item--drop-forbid {
  background-color: rgba(var(--v-theme-error), 0.18);
  border-radius: 0.25rem;
}
</style>
