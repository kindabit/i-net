<!--
  节点坐标调试叠层。

  在节点卡片左上角以 L 形排列显示 x / y 坐标：x 紧贴上边缘外侧（左对齐），
  y 紧贴左边缘外侧（顶对齐），与 vue-flow 节点坐标（左上角原点）语义一致。
  文字色取自当前主题的 on-surface 半透明变体，主题切换时自动跟随；
  pointer-events: none 确保不阻挡拖拽、hover、双击等节点交互。
-->
<script setup lang="ts">
/** 节点左上角坐标（vue-flow position，px）。父组件从 NodeProps.position 传入。 */
defineProps<{ x: number; y: number }>();
</script>

<template>
  <span class="node-debug-x">{{ Math.round(x) }}</span>
  <span class="node-debug-y">{{ Math.round(y) }}</span>
</template>

<style lang="scss" scoped>
.node-debug-x,
.node-debug-y {
  position: absolute;
  font-size: 0.6875rem;
  color: rgba(var(--v-theme-on-surface), 0.5);
  pointer-events: none;
  white-space: nowrap;
}

.node-debug-x {
  bottom: 100%;
  left: 0;
  margin-bottom: 0.125rem;
}

.node-debug-y {
  right: 100%;
  top: 0;
  margin-right: 0.125rem;
  padding-right: 0.25rem;
}
</style>