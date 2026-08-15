<!--
  通用附件预览组件（基于 @file-viewer/vue3-full）。

  将附件明文字节封装为 File 对象，交由 file-viewer 渲染。
  覆盖图片、音频、视频、Office 文档、PDF 等类型。
  通过 File 对象携带文件名，使 file-viewer 能正确提取扩展名并选择对应渲染器。
-->
<script setup lang="ts">
import { computed, shallowRef } from "vue";
import { FileViewer } from "@file-viewer/vue3-full";
import { mimeOf } from "./attachment-types";

const props = defineProps<{
  /** 附件明文字节 */
  bytes: Uint8Array;
  /** 文件名（用于推断 MIME 类型，并传递给 file-viewer 以提取扩展名） */
  fileName: string;
  /** 附件 ID（预留，供未来扩展如注释、书签等场景使用） */
  attachmentId: string;
}>();

/** file-viewer 实例引用；类型由库内部定义，使用 any 避免类型不兼容 */
// eslint-disable-next-line @typescript-eslint/no-explicit-any
const viewerRef = shallowRef<any>(null);

/**
 * 计算预览文件对象：根据文件名推断 MIME，构造 File 对象。
 * 若扩展名不受支持（理论上不会发生，因为路由层已过滤），返回 null。
 */
const fileObject = computed(() => {
  const mime = mimeOf(props.fileName);
  if (!mime) return null;
  return new File([props.bytes], props.fileName, { type: mime });
});

/** 销毁查看器实例 */
function destroy(): void {
  viewerRef.value?.destroy?.();
}

defineExpose({ destroy });
</script>

<template>
  <div v-if="fileObject" class="omni-viewer">
    <file-viewer
      ref="viewerRef"
      :file="fileObject"
      :options="{
        styleIsolation: 'shadow',
        theme: 'system',
      }"
    />
  </div>
  <div v-else class="omni-error">
    无法生成预览文件
  </div>
</template>

<style lang="scss" scoped>
.omni-viewer {
  width: 100%;
  height: 100%;
  min-height: 0;
  display: flex;
  flex-direction: column;
}

.omni-error {
  display: flex;
  justify-content: center;
  align-items: center;
  height: 100%;
  opacity: 0.75;
}
</style>
