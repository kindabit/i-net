/**
 * 前端路由定义。
 *
 * 提供首页（home）和数据库页（database）两个平级路由。
 * 数据库页内嵌画布宇宙等子路由。
 */
import { createRouter, createWebHistory } from "vue-router";
import Home from "@/views/Home.vue";
import CanvasUniverseView from "@/views/CanvasUniverseView.vue";
import CanvasView from "@/views/CanvasView.vue";
const router = createRouter({
  history: createWebHistory(),
  routes: [
    {
      path: "/",
      name: "home",
      component: Home,
    },
    {
      path: "/database",
      component: () => import("@/views/DatabaseView.vue"),
      children: [
        {
          path: "",
          redirect: { name: "canvas-universe" },
        },
        {
          path: "canvas-universe",
          name: "canvas-universe",
          // 静态导入：异步组件在并发过渡下入场动画不生效
          component: CanvasUniverseView,
        },
        {
          path: "canvas/:canvasId",
          name: "canvas",
          // 静态导入：异步组件在并发过渡下入场动画不生效
          component: CanvasView,
        },
      ],
    },
  ],
});

export default router;
