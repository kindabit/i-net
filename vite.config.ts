import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
import vueI18n from "@intlify/unplugin-vue-i18n/vite";
import ViteFonts from 'unplugin-fonts/vite';
import { fileViewerRenderers } from "@file-viewer/vite-plugin";
import { fileURLToPath, URL } from "node:url";

const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig(async () => ({
  resolve: {
    alias: {
      "@": fileURLToPath(new URL("./src", import.meta.url)),
    },
  },
  plugins: [
    vue(),
    vueI18n({
      // 自动收集合并 src/i18n/<模块>/<locale>.json，文件名即 locale 键
      include: [fileURLToPath(new URL("./src/i18n/**/*.json", import.meta.url))],
    }),
    ViteFonts({
      fontsource: {
        families: [
          {
            name: 'Roboto',
            weights: [100, 300, 400, 500, 700, 900],
            styles: ['normal', 'italic'],
          },
        ],
      },
    }),
    fileViewerRenderers({ copyAssets: true }),
  ],

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 1.5 @silurus/ooxml 的 WASM 引用路径会被 Vite 7 依赖优化器破坏，需排除
  optimizeDeps: {
    exclude: ['@silurus/ooxml'],
  },
  // 2. Worker 使用 ES 模块，避免 iife 与代码分割冲突
  worker: {
    format: 'es'
  },
  // 3. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // 3. tell Vite to ignore watching `src-tauri`
      // 4. onlyoffice 套件/构建产物均在 packages/ 与 public/ 下，源码不引用，无需监视（数千文件）
      ignored: ["**/src-tauri/**", "**/packages/**", "**/public/**"],
    },
  },
}));
