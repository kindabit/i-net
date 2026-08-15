/**
 * 画布路由切换动画的方向解析模块，供 DatabaseView 使用。
 *
 * 宇宙 ↔ 画布的切换方向由路由名对比判定（唯一事实来源）；
 * 画布 ↔ 画布的切换方向无法从路由参数得知，由导航调用点在跳转前
 * 通过 setCanvasNavIntent 记录钻取意图，DatabaseView 在路由变化后消费。
 */

export type CanvasNavIntent = "drill-in" | "drill-out";

let pendingIntent: CanvasNavIntent | null = null;

/**
 * 在发起画布间导航前记录钻取方向意图。
 * 输入：intent 钻取方向（drill-in 进入子画布 / drill-out 返回父画布）。
 */
export function setCanvasNavIntent(intent: CanvasNavIntent): void {
  pendingIntent = intent;
}

/**
 * 取出并清除待处理的导航意图。
 * 返回：待处理的钻取方向；无意图时返回 null。
 */
export function consumeCanvasNavIntent(): CanvasNavIntent | null {
  const intent = pendingIntent;
  pendingIntent = null;
  return intent;
}

/**
 * 根据前后路由名与导航意图解析过渡动画名。
 * 输入：fromName 跳转前路由名（null 表示 DatabaseView 首次挂载，配合 appear 播放
 *       入场动画），toName 跳转后路由名，intent 画布间导航意图。
 * 返回：Transition 的 name（drill-in / drill-out / drill-swap）；
 *       空字符串表示不播放过渡（如数据库布局外的路由切换）。
 */
export function resolveRouteTransition(
  fromName: string | null,
  toName: string | null,
  intent: CanvasNavIntent | null,
): string {
  if (fromName === "canvas-universe" && toName === "canvas") {
    return "drill-in";
  }
  if (fromName === "canvas" && toName === "canvas-universe") {
    return "drill-out";
  }
  if (fromName === "canvas" && toName === "canvas") {
    return intent ?? "drill-swap";
  }
  // 首次挂载：从主页进入画布或画布宇宙均播放钻入动画
  if (fromName === null && (toName === "canvas" || toName === "canvas-universe")) {
    return "drill-in";
  }
  return "";
}
