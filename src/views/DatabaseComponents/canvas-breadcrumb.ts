/**
 * 组装画布层级链的纯函数模块，供 CanvasBreadcrumb 使用。
 *
 * buildCanvasChain 从当前画布沿 parent_id 上溯至根画布，返回有序链。
 * collapseChain 折叠过长的链，只保留当前画布并将中间节点放入隐藏列表。
 */
import type { Canvas } from "@/api-types";

export type CanvasChainResult =
  | { status: "ok"; chain: Canvas[] }
  | { status: "not-found" }
  | { status: "cycle" };

type CollapsedChain =
  | { collapsed: false; visible: Canvas[] }
  | { collapsed: true; current: Canvas; hidden: Canvas[] };

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

export function collapseChain(chain: Canvas[]): CollapsedChain {
  if (chain.length < 3) {
    return { collapsed: false, visible: chain };
  }
  const current = chain[chain.length - 1];
  const hidden = chain.slice(0, -1);
  return { collapsed: true, current, hidden };
}
