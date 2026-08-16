<!--
  视口调试叠层。

  在画布页面（ CanvasView / CanvasUniverseView ）左下角以一行小字实时显示
  vue-flow 视口的平移坐标与缩放等级，仅用于开发期调试，无交互。
  文字色取自当前主题的 on-surface 半透明变体，主题切换时自动跟随。
-->
<script setup lang="ts">
/** vue-flow 视口快照。父组件从 useVueFlow().viewport 传入，仅读取不修改。 */
defineProps<{ viewport: { x: number; y: number; zoom: number } }>();
</script>

<template>
  <div class="viewport-debug">x: {{ Math.round(viewport.x) }} y: {{ Math.round(viewport.y) }} zoom: {{ viewport.zoom.toFixed(2) }}</div>
</template>

<style lang="scss" scoped>
.viewport-debug {
  position: absolute;
  bottom: 0.0;
  left: 0.0;
  z-index: 10;
  font-size: 0.5rem;
  color: rgba(var(--v-theme-on-surface), 0.5);
  pointer-events: none;
}
</style>