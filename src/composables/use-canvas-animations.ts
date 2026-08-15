/**
 * 画布动画。
 * 基于 Web Animations API 的 fire-and-forget 动画：播完效果自动消失，无需清理状态。
 * 同一元素的新动画自动 replace 旧动画，调用方无需取消。
 */
import { nextTick } from "vue";

/** 边淡入/淡出时长（ms） */
export const EDGE_FADE_DURATION = 300;
/** 节点入场时长（ms） */
export const NODE_ENTER_DURATION = 350;
/** 节点飞向回收站时长（ms） */
export const NODE_FLY_DURATION = 400;

/** 按节点 id 查询 vue-flow 节点 DOM */
function nodeEl(nodeId: string): HTMLElement | null {
  return document.querySelector<HTMLElement>(`.vue-flow__node[data-id="${nodeId}"]`);
}

/** 按边 id 查询 vue-flow 边 DOM */
function edgeEl(edgeId: string): HTMLElement | null {
  return document.querySelector<HTMLElement>(`.vue-flow__edge[data-id="${edgeId}"]`);
}

/**
 * 节点入场动画：从透明缩小状态缩放淡入。等待 nextTick 确保 DOM 已渲染。
 * resolve 时机为动画播完或被取消时（catch 已内部静默）；
 * 缩放动画会干扰 vue-flow 的 handle 位置测量，调用方需要 await 后再校正测量。
 */
export function fadeInNode(nodeId: string): Promise<void> {
  return nextTick().then(() => {
    const el = nodeEl(nodeId);
    if (!el) return;
    return el
      .animate(
        [{ opacity: 0, scale: 0.3 }, { opacity: 1, scale: 1 }],
        { duration: NODE_ENTER_DURATION, easing: "ease" },
      )
      .finished.then(() => {})
      .catch(() => {});
  });
}

/**
 * 边入场动画：对新加入的边播放 0→1 淡入。等待 nextTick 确保 DOM 已渲染。
 */
export function fadeInEdges(edgeIds: string[]): void {
  nextTick(() => {
    for (const id of edgeIds) {
      edgeEl(id)?.animate(
        [{ opacity: 0 }, { opacity: 1 }],
        { duration: EDGE_FADE_DURATION, easing: "ease" },
      );
    }
  });
}

/** ghost 残影的 overlay 图层（懒创建单例） */
let ghostOverlay: SVGSVGElement | null = null;

/** 获取/创建覆盖全屏的 SVG 残影图层 */
function getGhostOverlay(): SVGSVGElement {
  if (ghostOverlay && document.body.contains(ghostOverlay)) return ghostOverlay;
  ghostOverlay = document.createElementNS("http://www.w3.org/2000/svg", "svg");
  ghostOverlay.style.position = "fixed";
  ghostOverlay.style.inset = "0";
  ghostOverlay.style.width = "100%";
  ghostOverlay.style.height = "100%";
  ghostOverlay.style.pointerEvents = "none";
  ghostOverlay.style.zIndex = "9998";
  document.body.appendChild(ghostOverlay);
  return ghostOverlay;
}

/**
 * 边离场残影：克隆边 DOM 到全屏 overlay，播放淡出后自删。
 * 调用方应立即从 edges 数据中移除本边——残影与数据无关，fire-and-forget。
 * 通过 getScreenCTM 复制坐标变换矩阵，残影与原边屏幕位置精确重合。
 */
export function ghostOutEdge(edgeId: string): void {
  const edge = edgeEl(edgeId);
  if (!edge) return;
  const ctm = (edge as unknown as SVGGElement).getScreenCTM();
  if (!ctm) return;

  const clone = edge.cloneNode(true) as unknown as SVGGElement;
  clone.querySelectorAll("path").forEach((p) => {
    p.removeAttribute("marker-end");
    p.removeAttribute("marker-start");
  });
  clone.setAttribute(
    "transform",
    `matrix(${ctm.a} ${ctm.b} ${ctm.c} ${ctm.d} ${ctm.e} ${ctm.f})`,
  );

  getGhostOverlay().appendChild(clone);
  clone
    .animate([{ opacity: 1 }, { opacity: 0 }], {
      duration: EDGE_FADE_DURATION,
      easing: "ease",
    })
    .finished.then(() => clone.remove())
    .catch(() => {});
}

/**
 * 节点飞向回收站的克隆体动画：克隆节点 DOM，从原位置飞到目标元素位置，
 * 边移动边缩小淡出，播完移除克隆体（自包含 fire-and-forget）。
 * @param nodeId vue-flow 节点 id
 * @param targetEl 动画终点元素（回收站菜单按钮的 DOM）
 * @param cardSelector 节点卡片的选择器，默认为 ".data-node-card"
 * @param actionsSelector 操作按钮排的选择器，默认为 ".data-node-actions"
 */
export function flyToRecycleBin(
  nodeId: string,
  targetEl: HTMLElement,
  cardSelector = ".data-node-card",
  actionsSelector = ".data-node-actions",
): void {
  const cardEl = document.querySelector<HTMLElement>(
    `.vue-flow__node[data-id="${nodeId}"] ${cardSelector}`,
  );
  if (!cardEl) return;

  const rect = cardEl.getBoundingClientRect();
  const clone = cardEl.cloneNode(true) as HTMLElement;
  // 克隆体不携带 hover 按钮排与连接桩
  clone.querySelector(actionsSelector)?.remove();
  clone.querySelectorAll(".vue-flow__handle").forEach((h) => h.remove());

  clone.style.position = "fixed";
  clone.style.left = `${rect.left}px`;
  clone.style.top = `${rect.top}px`;
  clone.style.width = `${rect.width}px`;
  clone.style.height = `${rect.height}px`;
  clone.style.margin = "0";
  clone.style.zIndex = "9999";
  clone.style.pointerEvents = "none";

  document.body.appendChild(clone);

  const targetRect = targetEl.getBoundingClientRect();
  const dx = targetRect.left + targetRect.width / 2 - (rect.left + rect.width / 2);
  const dy = targetRect.top + targetRect.height / 2 - (rect.top + rect.height / 2);

  // 单关键帧：从当前状态动画到目标状态
  clone
    .animate(
      { transform: `translate(${dx}px, ${dy}px) scale(0)`, opacity: "0" },
      { duration: NODE_FLY_DURATION, easing: "cubic-bezier(0.4, 0, 0.2, 1)" },
    )
    .finished.then(() => clone.remove())
    .catch(() => {});
}
