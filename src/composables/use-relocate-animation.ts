/**
 * 跨画布迁移的消失动画。
 * 属于节点移动和迁移系统：迁移成功后，被迁走的节点与内部边在源画布播放
 * 失焦淡出（opacity + blur，视觉效果仿画布路由切换的 blur 过渡），
 * 播完由调用方从数据中移除。
 */

/** 失焦淡出时长（ms） */
const BLUR_OUT_DURATION = 120;
/** 失焦淡出终点模糊半径 */
const BLUR_OUT_RADIUS = "blur(12px)";

/** 按节点 id 查询 vue-flow 节点 DOM */
function nodeEl(nodeId: string): HTMLElement | null {
  return document.querySelector<HTMLElement>(`.vue-flow__node[data-id="${nodeId}"]`);
}

/** 按边 id 查询 vue-flow 边 DOM */
function edgeEl(edgeId: string): HTMLElement | null {
  return document.querySelector<HTMLElement>(`.vue-flow__edge[data-id="${edgeId}"]`);
}

/**
 * 对被迁移的节点与内部边播放失焦淡出动画，全部播完或被取消后 resolve。
 * 遵循 prefers-reduced-motion：用户要求减少动态效果时立即 resolve（不播动画）。
 * 调用方应在 resolve 后再从数据中移除这些节点和边。
 * @param nodeIds 被迁移的节点 id 列表
 * @param edgeIds 随迁的内部边 id 列表
 * @returns 动画全部结束（或跳过）后 resolve 的 Promise
 */
export function blurOutRelocated(nodeIds: string[], edgeIds: string[]): Promise<void> {
  if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) {
    return Promise.resolve();
  }
  const blurOut = (el: HTMLElement): Promise<void> =>
    el.animate(
      [{ opacity: 1, filter: "blur(0px)" }, { opacity: 0, filter: BLUR_OUT_RADIUS }],
      { duration: BLUR_OUT_DURATION, easing: "cubic-bezier(0.4, 0, 0.2, 1)" },
    ).finished.then(() => {}).catch(() => {});
  const animations: Promise<void>[] = [];
  for (const id of nodeIds) {
    const el = nodeEl(id);
    if (el) animations.push(blurOut(el));
  }
  for (const id of edgeIds) {
    const el = edgeEl(id);
    if (el) animations.push(blurOut(el));
  }
  return Promise.all(animations).then(() => {});
}
