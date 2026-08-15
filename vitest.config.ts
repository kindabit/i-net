import { defineConfig } from "vitest/config";
import { fileURLToPath, URL } from "node:url";

// vitest 独立配置：不加载 vite.config.ts 中的 vue/i18n/字体等构建插件，
// 只保留路径别名，保证纯逻辑单测在 node 环境下快速运行。
export default defineConfig({
  resolve: {
    alias: {
      "@": fileURLToPath(new URL("./src", import.meta.url)),
    },
  },
  test: {
    environment: "node",
    include: ["src/**/*.test.ts"],
  },
});
