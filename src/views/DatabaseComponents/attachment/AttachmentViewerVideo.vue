<!--
  附件视频查看器。

  将附件明文字节包装为内存 Blob URL 并以视频播放器渲染，全程不落盘；
  组件卸载时回收 Blob URL。
-->
<script setup lang="ts">
import { onUnmounted } from "vue";
import { mimeOf } from "./attachment-types";

const props = defineProps<{
  /** 附件明文内容 */
  bytes: Uint8Array;
  /** 附件文件名（用于推断 MIME 类型） */
  fileName: string;
}>();

/** 附件字节对应的内存 Blob URL */
const blobUrl = createBlobUrl(props.bytes, props.fileName);

/**
 * 将附件字节包装为内存 Blob URL。
 * @param bytes 附件明文内容
 * @param fileName 附件文件名
 * @returns Blob URL
 */
function createBlobUrl(bytes: Uint8Array, fileName: string): string {
  const mime = mimeOf(fileName);
  const blob = new Blob([bytes], { type: mime ?? undefined });
  return URL.createObjectURL(blob);
}

onUnmounted(() => {
  URL.revokeObjectURL(blobUrl);
});
</script>

<template>
  <video class="viewer-video" :src="blobUrl" controls />
</template>

<style lang="scss" scoped>
.viewer-video {
  // 占满固定高度的查看器容器，视频内容等比缩放（不足部分黑边填充）
  width: 100%;
  height: 100%;
  object-fit: contain;
}
</style>
