/**
 * 组装画布层级链的纯函数模块，供 CanvasBreadcrumb 使用。
 *
 * buildCanvasChain 从当前画布沿 parent_id 上溯至根画布，返回有序链。
 * collapseChain 折叠过长的链：根画布与上一级画布固定保留，
 * 仅将两者之间的中间节点放入隐藏列表。
 */
import type { Canvas } from "@/api-types";

export type CanvasChainResult =
  | { status: "ok"; chain: Canvas[] }
  | { status: "not-found" }
  | { status: "cycle" };

type CollapsedChain =
  | { collapsed: false; visible: Canvas[] }
  | {
      collapsed: true;
      root: Canvas;
      parent: Canvas;
      current: Canvas;
      hidden: Canvas[];
    };

export function buildCanvasChain(
  canvases: Canvas[],
  canvasId: string,
): CanvasChainResult {
  const map = new Map<string, Canvas>();
  for (const c of canvases) {
    map.set(c.id, c);
  }
  if (!map.has(canvasId)) {
    return { status: "not-found" };
  }
  const chain: Canvas[] = [];
  const visited = new Set<string>();
  let id: string | null = canvasId;
  while (id !== null) {
    if (visited.has(id)) {
      return { status: "cycle" };
    }
    visited.add(id);
    const c: Canvas = map.get(id)!;
    chain.unshift(c);
    id = c.parent_id;
  }
  return { status: "ok", chain };
}

/**
 * 折叠过长的层级链，供面包屑渲染。
 * 输入：chain 由 buildCanvasChain 返回的有序链（根画布在首、当前画布在尾）。
 * 返回：链长 ≤ 3 时不折叠（全部可见）；链长 ≥ 4 时折叠，根画布与上一级
 * 画布（chain 尾二位）固定保留，两者之间的中间节点按原有顺序放入 hidden。
 */
export function collapseChain(chain: Canvas[]): CollapsedChain {
  if (chain.length <= 3) {
    return { collapsed: false, visible: chain };
  }
  const root = chain[0];
  const parent = chain[chain.length - 2];
  const current = chain[chain.length - 1];
  const hidden = chain.slice(1, -2);
  return { collapsed: true, root, parent, current, hidden };
}
