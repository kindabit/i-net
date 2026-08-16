import { defineConfig, loadEnv } from "vite";
import vue from "@vitejs/plugin-vue";
import vueI18n from "@intlify/unplugin-vue-i18n/vite";
import ViteFonts from 'unplugin-fonts/vite';
import { fileViewerRenderers } from "@file-viewer/vite-plugin";
import vitePluginConditionalCompile from "vite-plugin-conditional-compile";
import { fileURLToPath, URL } from "node:url";

const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig(async ({ mode }) => {
  // vite.config.ts 在 vite 启动之前由 Node 直接 require 执行，
  // 不会自动加载 .env 文件——必须手动 loadEnv 注入。
  // mode 由 `pnpm dev` / `pnpm build` / `pnpm dev --mode X` 决定，
  // 对应 .env.{mode} 文件（如 .env.development / .env.production）。
  const env = loadEnv(mode, process.cwd(), "");

  return {
    resolve: {
      alias: {
        "@": fileURLToPath(new URL("./src", import.meta.url)),
      },
    },
    plugins: [
      // 条件编译插件：源码中用 // #if [DEBUG] / <!-- #if [DEBUG] --> 包裹的代码块，
      // 在 DEBUG=false 时从产物中完全剔除（注释指令 + 内含代码一起移除）。
      // 必须放在 vue() 之前：插件自身声明了 enforce: "pre"，但显式靠前更稳。
      vitePluginConditionalCompile({
        // loadEnv 返回字符串 "true"/"false"，babel 严格判 === true 会失败。
        // 显式转 boolean 是必需步骤，不能省略。
        // 注意：插件自身在 configResolved 阶段也会调 loadEnv 并合并 ctx.env，
        // 但其合并顺序是 { ...loadEnv, ...userOptions.env }，外部传值覆盖。
        // 因此这里必须显式传正确的 boolean，否则系统环境变量缺失时 DEBUG 永远为 false。
        env: {
          DEBUG: env.DEBUG === "true",
        },
      }),
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
  };
});
