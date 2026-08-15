/**
 * 前端应用入口。
 *
 * 初始化流程：创建 Vue 实例 -> 加载偏好 -> i18n ->
 * vuetify（内置主题随其创建注册，自定义主题与偏好主题在挂载前注入、应用）->
 * router -> 挂载根组件。
 */
import "unfonts.css";
import "@mdi/font/css/materialdesignicons.css";
import "vuetify/styles";
import "@vue-flow/core/dist/style.css";
import "@vue-flow/core/dist/theme-default.css";
import "@vue-flow/controls/dist/style.css";
import "@/styles/vue-flow.scss";
import "@/styles/frosted-glass.scss";

import { createApp } from "vue";

import App from "./App.vue";
import { mountI18n } from "@/i18n";
import { vuetify } from "@/vuetify";
import { initThemes } from "@/themes";
import router from "@/router";

/**
 * 初始化并挂载 Vue 应用。
 * @returns 无返回值
 */
async function initApp() {
  const app = createApp(App);

  await mountI18n(app);

  app.use(vuetify);
  await initThemes();

  app.use(router);

  app.mount("#app");
}

void initApp();
