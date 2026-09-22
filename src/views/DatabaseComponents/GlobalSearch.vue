<!--
  全局搜索组件。

  收起态为搜索图标按钮，点击后展开为输入框；输入关键词防抖搜索所有画布中的节点，
  下拉展示结果（结果项左侧为标题与副标题，右侧为节点标签），支持键盘导航与鼠标点击，
  选中后跳转到对应画布并居中目标节点。
-->
<script setup lang="ts">
import { ref, nextTick, onUnmounted } from "vue";
import { useRoute, useRouter } from "vue-router";
import debounce from "lodash/debounce";
import { t } from "@/i18n";
import { userDatabaseNodeSearch } from "@/api";
import type { NodeSearchResponse } from "@/api-types";
import { snackbarErrorCode } from "@/composables/use-snackbar";

const route = useRoute();
const router = useRouter();

/** 是否处于展开状态 */
const expanded = defineModel<boolean>("expanded", { default: false });

/** 输入关键词 */
const keyword = ref("");
/** 搜索结果列表 */
const results = ref<NodeSearchResponse[]>([]);
/** 下拉框是否可见 */
const dropdownOpen = ref(false);
/** 当前高亮项索引 */
const highlightedIndex = ref(0);
/** 输入框模板引用 */
const inputRef = ref<{ focus: () => void } | undefined>();
/** 下拉框模板引用 */
const dropdownRef = ref<{ $el: HTMLElement } | undefined>();

/** 请求序号，用于竞态丢弃过期结果 */
let requestSeq = 0;

/** 单个搜索结果最多显示的标签数量；超出时追加一个省略号 chip */
const MAX_VISIBLE_TAGS = 3;

/** 当前结果集对应的查询关键词列表（小写），用于把命中的标签排在前面 */
const searchedKeywords = ref<string[]>([]);

/**
 * 执行搜索请求（防抖后调用）。
 * 输入：无。
 * 返回：无返回值。
 */
async function doSearch() {
  const seq = ++requestSeq;
  const q = keyword.value.trim();
  if (q === "") {
    results.value = [];
    searchedKeywords.value = [];
    dropdownOpen.value = false;
    return;
  }
  try {
    const data = await userDatabaseNodeSearch(q);
    if (seq !== requestSeq) return;
    results.value = data;
    searchedKeywords.value = splitKeywords(q);
    highlightedIndex.value = 0;
    // 无论是否有结果都打开下拉：无结果时展示 no-results 占位，向用户反馈搜索已完成
    dropdownOpen.value = true;
  } catch (e) {
    if (seq !== requestSeq) return;
    snackbarErrorCode(e);
  }
}

const debouncedSearch = debounce(doSearch, 300);

/**
 * 图标按钮点击处理：收起态展开并聚焦输入框，展开态仅聚焦输入框。
 * 输入：无。
 * 返回：无返回值。
 */
async function onIconClick() {
  if (!expanded.value) {
    expanded.value = true;
    await nextTick();
  }
  inputRef.value?.focus();
}

/**
 * 输入事件处理：空关键词时清空结果并取消防抖，否则触发防抖搜索。
 * 输入：无。
 * 返回：无返回值。
 */
function onInput() {
  const q = keyword.value.trim();
  if (q === "") {
    debouncedSearch.cancel();
    results.value = [];
    searchedKeywords.value = [];
    dropdownOpen.value = false;
    return;
  }
  debouncedSearch();
}

/**
 * 将当前高亮项滚动到可视区域。
 * 输入：无。
 * 返回：无返回值。
 */
async function scrollToHighlighted() {
  await nextTick();
  const dropdownEl = dropdownRef.value?.$el;
  if (!dropdownEl) return;
  const highlightedEl = dropdownEl.querySelector(".search-highlighted") as HTMLElement | null;
  highlightedEl?.scrollIntoView({ block: "nearest" });
}

/**
 * 返回搜索结果所在画布的展示名称：根画布显示本地化文案，其余画布显示原名。
 * 输入：item 搜索结果项。
 * 返回：画布展示名称。
 */
function canvasDisplayName(item: NodeSearchResponse): string {
  return item.canvas_is_root ? t("database.canvas.root-canvas") : item.canvas_name;
}

/**
 * 把原始查询串拆分为关键词列表，与后端拆分规则保持一致：
 * 以空白字符以及 `,` `，` `、` `。` `.` `;` `；` 为分隔符，每段非空才保留。
 * @param query 原始查询串。
 * @returns 小写化的关键词列表。
 */
function splitKeywords(query: string): string[] {
  return query
    .split(/[\s,，、。.;；]+/u)
    .map((part) => part.trim().toLowerCase())
    .filter((part) => part !== "");
}

/**
 * 组合搜索结果的副标题行文本：节点副标题与画布展示名以间隔点连接，无节点副标题时仅显示画布名。
 * @param item 搜索结果项。
 * @returns 副标题行文本。
 */
function subtitleText(item: NodeSearchResponse): string {
  return item.subtitle ? `${item.subtitle} · ${canvasDisplayName(item)}` : canvasDisplayName(item);
}

/**
 * 计算搜索结果项的标签展示顺序：命中任一查询关键词的标签排在前面，其余标签在后，
 * 两组内部均保持后端返回的标签名称升序；无关键词时等同于全部按字典序排列。
 * @param item 搜索结果项。
 * @returns 排序后的标签列表。
 */
function orderedTags(item: NodeSearchResponse): string[] {
  const keywords = searchedKeywords.value;
  if (keywords.length === 0) return item.tags;
  const matched: string[] = [];
  const unmatched: string[] = [];
  for (const tag of item.tags) {
    const lower = tag.toLowerCase();
    if (keywords.some((keyword) => lower.includes(keyword))) {
      matched.push(tag);
    } else {
      unmatched.push(tag);
    }
  }
  return [...matched, ...unmatched];
}

/**
 * 返回搜索结果项实际展示的标签列表：取展示顺序中的前 MAX_VISIBLE_TAGS 个。
 * @param item 搜索结果项。
 * @returns 展示用标签列表。
 */
function visibleTags(item: NodeSearchResponse): string[] {
  return orderedTags(item).slice(0, MAX_VISIBLE_TAGS);
}

/**
 * 键盘导航处理：ArrowDown/ArrowUp 循环高亮，Enter 选择高亮项，Escape 关闭下拉。
 * 输入：e 键盘事件。
 * 返回：无返回值。
 */
function onKeydown(e: KeyboardEvent) {
  if (!dropdownOpen.value) return;
  switch (e.key) {
    case "ArrowDown":
      e.preventDefault();
      if (results.value.length > 0) {
        highlightedIndex.value = (highlightedIndex.value + 1) % results.value.length;
        void scrollToHighlighted();
      }
      break;
    case "ArrowUp":
      e.preventDefault();
      if (results.value.length > 0) {
        highlightedIndex.value = (highlightedIndex.value - 1 + results.value.length) % results.value.length;
        void scrollToHighlighted();
      }
      break;
    case "Enter":
      e.preventDefault();
      if (results.value[highlightedIndex.value]) {
        select(results.value[highlightedIndex.value]);
      }
      break;
    case "Escape":
      e.preventDefault();
      dropdownOpen.value = false;
      break;
  }
}

/**
 * 选中搜索结果项：跳转到对应画布并居中目标节点，清空搜索状态但不收起输入框。
 * 输入：item 搜索结果项。
 * 返回：无返回值。
 */
function select(item: NodeSearchResponse) {
  keyword.value = "";
  results.value = [];
  searchedKeywords.value = [];
  dropdownOpen.value = false;
  void router.push({
    name: "canvas",
    params: { canvasId: item.canvas_id },
    // 清除可能残留的 edgeId：edgeId 与 nodeId 是两个互斥的定位参数，搜索定位以 nodeId 为准
    query: { ...route.query, edgeId: undefined, nodeId: item.id },
  });
}

/** 收起搜索：清空关键词、结果与下拉状态，折叠输入框 */
function collapse() {
  keyword.value = "";
  results.value = [];
  searchedKeywords.value = [];
  dropdownOpen.value = false;
  expanded.value = false;
}

onUnmounted(() => {
  debouncedSearch.cancel();
});
</script>

<template>
  <div class="global-search" :class="{ 'global-search--expanded': expanded }">
    <!-- 图标按钮：常驻。收起时点击展开；展开时点击聚焦输入框 -->
    <VBtn icon="mdi-magnify" size="small" variant="text" :title="t('database.search.open')" @click="onIconClick" />

    <!-- 输入框区域：grid 宽度动画 -->
    <div class="search-field-grid">
      <div class="search-field-clip">
        <div class="search-field-content">
          <VTextField
            ref="inputRef"
            v-model="keyword"
            :placeholder="t('database.search.placeholder')"
            density="compact"
            variant="plain"
            hide-details
            @keydown="onKeydown"
            @input="onInput"
          />
        </div>
      </div>
    </div>

    <!-- 关闭按钮区域：grid 宽度动画 -->
    <div class="search-close-grid">
      <div class="search-close-clip">
        <VBtn icon="mdi-close" size="small" variant="text" :title="t('database.search.close')" @click="collapse" />
      </div>
    </div>

    <!-- 结果下拉：absolute 定位于输入框下方 -->
    <Transition name="dropdown">
      <VCard v-show="dropdownOpen" ref="dropdownRef" class="search-dropdown" elevation="8" rounded="lg">
        <VList v-if="results.length > 0" density="compact">
          <VListItem
            v-for="(item, i) in results"
            :key="item.id"
            :active="i === highlightedIndex"
            :class="{ 'search-highlighted': i === highlightedIndex }"
            @mousedown.prevent="select(item)"
          >
            <!-- 结果项为左右两列：左列标题与副标题（左对齐），右列标签（垂直居中、右对齐） -->
            <div class="search-item-body">
              <div class="search-item-main">
                <VListItemTitle>{{ item.title }}</VListItemTitle>
                <VListItemSubtitle>{{ subtitleText(item) }}</VListItemSubtitle>
              </div>
              <!-- 标签列：单行右对齐；命中的标签靠前，最多 MAX_VISIBLE_TAGS 个，超出以省略号 chip 表示 -->
              <div v-if="item.tags.length > 0" class="search-item-tags">
                <VChip
                  v-for="tag in visibleTags(item)"
                  :key="tag"
                  size="x-small"
                  class="search-item-tag"
                >
                  {{ tag }}
                </VChip>
                <VChip v-if="item.tags.length > MAX_VISIBLE_TAGS" size="x-small" class="search-item-tag">…</VChip>
              </div>
            </div>
          </VListItem>
        </VList>
        <div v-else class="text-body-2 text-disabled pa-3 text-center">
          {{ t("database.search.no-results") }}
        </div>
      </VCard>
    </Transition>
  </div>
</template>

<style lang="scss" scoped>
.global-search {
  position: relative;
  display: flex;
  align-items: center;
}

.search-field-grid,
.search-close-grid {
  display: grid;
  grid-template-columns: 0fr;
  transition: grid-template-columns 0.25s ease;
}

.global-search--expanded .search-field-grid {
  grid-template-columns: 1fr;
}
.global-search--expanded .search-close-grid {
  grid-template-columns: 1fr;
}

.search-field-clip,
.search-close-clip {
  overflow: hidden;
  min-width: 0;
}

/* 内容宽度固定，动画期间只被裁剪、不被压缩重排 */
.search-field-content {
  width: 15rem;
}

/* 消除 Vuetify compact 密度为浮动标签预留的顶部留白（--v-input-padding-top），
   使输入内容与两侧图标按钮同高、垂直居中 */
.search-field-content :deep(.v-input) {
  --v-input-padding-top: 0px;
}

.search-dropdown {
  position: absolute;
  top: 100%;
  left: 0;
  right: 0;
  margin-top: 0.375rem;
  z-index: 200;
  max-height: 15rem;
  overflow-y: auto;
  transform-origin: top center;
}

/* 结果项主体：左列标题与副标题，右列标签 */
.search-item-body {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

/* 左列可收缩，标题与副标题超宽时省略号收尾 */
.search-item-main {
  flex: 1;
  min-width: 0;
}

/* 标签列：单行右对齐（与左对齐的标题、副标题形成对照），超宽时裁剪而非换行 */
.search-item-tags {
  display: flex;
  flex-shrink: 0;
  justify-content: flex-end;
  gap: 0.25rem;
  max-width: 55%;
  overflow: hidden;
  white-space: nowrap;
}

.search-item-tag {
  flex-shrink: 0;
}

.dropdown-enter-active,
.dropdown-leave-active {
  transition: opacity 0.2s ease, transform 0.2s ease;
}
.dropdown-enter-from,
.dropdown-leave-to {
  opacity: 0;
  transform: scaleY(0.8) translateY(-0.5rem);
}
</style>
