<!--
  全局 Snackbar 消息队列组件。

  监听 use-snackbar 中的 snackbarMessages，按队列方式显示消息提示。
-->
<script setup lang="ts">
import { snackbarMessages } from "@/composables/use-snackbar";
</script>

<template>
  <VSnackbarQueue
    v-model="snackbarMessages"
    location="bottom"
    :total-visible="7"
    transition="fade-transition"
  />
</template>
