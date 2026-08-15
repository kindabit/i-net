<!--
  数据库页面顶部栏。

  居中悬浮于 DatabaseView 顶部，全局搜索与画布层级面包屑共处同一毛玻璃背景，
  两者之间以竖直分隔线相隔；搜索展开时面包屑与分隔线以宽度收缩动画让位，避免内容推挤错位。
-->
<script setup lang="ts">
import { ref } from "vue";
import GlobalSearch from "./GlobalSearch.vue";
import CanvasBreadcrumb from "./CanvasBreadcrumb.vue";

defineProps<{
  /** 当前画布 id，画布宇宙页面为 undefined */
  canvasId?: string;
}>();

/** 全局搜索是否处于展开状态（展开时面包屑让位隐藏） */
const searchExpanded = ref(false);
</script>

<template>
  <div class="topbar frosted-glass">
    <GlobalSearch v-model:expanded="searchExpanded" />
    <div
      class="breadcrumb-grid"
      :class="{ 'breadcrumb-grid--hidden': searchExpanded }"
    >
      <div class="breadcrumb-clip">
        <div class="breadcrumb-content">
          <VDivider vertical class="topbar-divider" />
          <CanvasBreadcrumb :canvas-id="canvasId" />
        </div>
      </div>
    </div>
  </div>
</template>

<style lang="scss" scoped>
.topbar {
  position: absolute;
  top: 0.75rem;
  left: 50%;
  transform: translateX(-50%);
  z-index: 10;
  display: flex;
  align-items: center;
  max-width: calc(100% - 8.25rem);
  padding: 0.25rem;
  border-radius: 0.5rem;
}

.breadcrumb-grid {
  display: grid;
  grid-template-columns: 1fr;
  transition: grid-template-columns 0.25s ease, opacity 0.2s ease;
  opacity: 1;
}

.breadcrumb-grid--hidden {
  grid-template-columns: 0fr;
  opacity: 0;
}

.breadcrumb-clip {
  overflow: hidden;
  min-width: 0;
}

/* 面包屑内容与分隔线保持自然宽度，收缩期间仅被裁剪、不被压缩重排 */
.breadcrumb-content {
  display: flex;
  align-items: center;
  width: max-content;
  max-width: 40rem;
}

/* 竖直分隔线：显式高度并强制居中——Vuetify 默认 align-self: stretch 在固定高度下会退化为顶部对齐 */
.topbar-divider {
  flex: 0 0 auto;
  align-self: center;
  height: 1.5rem;
  margin: 0 0.5rem;
}
</style>
