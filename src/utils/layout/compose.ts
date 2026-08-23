/**
 * Pass 7 · 全局合成：分量平铺、孤立节点网格与整体归一。
 *
 * 互不连通的分量各自完成布局后按节点数降序（平局按最小 id）网格平铺；
 * 没有任何边连接的孤立节点（以及防御性识别出的环相关节点）单独在主区域
 * 右侧排成网格；最后整体平移使含节点尺寸的包围盒中心位于原点。
 */

import type { ComponentLayout } from "./component-layout";
import type {
  RadialLayoutConfig,
  RadialLayoutNode,
  RadialLayoutPoint,
} from "./types";
import { compareId } from "./utils";

/**
 * 合成最终布局。
 *
 * @param components 各连通分量的局部布局结果。
 * @param isolatedIds 孤立节点 id 列表（含环降级节点，顺序任意，内部会排序）。
 * @param nodeById 节点 id → 节点输入。
 * @param config 布局参数。
 * @returns 节点 id → 中心坐标（全局坐标，包围盒中心位于原点）。
 */
export function composeLayout(
  components: ComponentLayout[],
  isolatedIds: string[],
  nodeById: Map<string, RadialLayoutNode>,
  config: RadialLayoutConfig,
): Map<string, RadialLayoutPoint> {
  const result = new Map<string, RadialLayoutPoint>();

  // 分量平铺：按节点数降序（平局按最小 id），贪心装入近似正方形的多行。
  components.sort(
    (a, b) =>
      b.nodeIds.length - a.nodeIds.length ||
      compareId(
        a.nodeIds.reduce((min, id) => (id < min ? id : min)),
        b.nodeIds.reduce((min, id) => (id < min ? id : min)),
      ),
  );
  const componentOrigins = new Map<ComponentLayout, RadialLayoutPoint>();
  let componentsMaxX = 0;
  if (components.length > 0) {
    const totalArea = components.reduce((sum, component) => {
      const footprint = 2 * component.boundingRadius + config.componentGap;
      return sum + footprint * footprint;
    }, 0);
    const targetRowWidth = Math.max(
      Math.sqrt(totalArea),
      2 * components[0].boundingRadius,
    );
    let cursorX = 0;
    let cursorY = 0;
    let rowHeight = 0;
    for (const component of components) {
      const diameter = 2 * component.boundingRadius;
      if (cursorX > 0 && cursorX + diameter > targetRowWidth) {
        cursorY += rowHeight + config.componentGap;
        cursorX = 0;
        rowHeight = 0;
      }
      componentOrigins.set(component, {
        cx: cursorX + component.boundingRadius,
        cy: cursorY + component.boundingRadius,
      });
      cursorX += diameter + config.componentGap;
      rowHeight = Math.max(rowHeight, diameter);
      componentsMaxX = Math.max(componentsMaxX, cursorX - config.componentGap);
    }
  }
  for (const component of components) {
    const origin = componentOrigins.get(component)!;
    for (const [id, point] of component.positions) {
      result.set(id, { cx: point.cx + origin.cx, cy: point.cy + origin.cy });
    }
  }

  // 孤立节点网格：按 id 排序，置于分量区域右侧（无分量时从原点起排）。
  if (isolatedIds.length > 0) {
    isolatedIds.sort(compareId);
    const columns = Math.ceil(Math.sqrt(isolatedIds.length));
    const originX =
      components.length > 0 ? componentsMaxX + config.componentGap : 0;
    isolatedIds.forEach((id, index) => {
      const column = index % columns;
      const row = Math.floor(index / columns);
      result.set(id, {
        cx: originX + column * config.isolatedCell + config.isolatedCell / 2,
        cy: row * config.isolatedCell + config.isolatedCell / 2,
      });
    });
  }

  // 整体归一：平移使含节点尺寸的包围盒中心位于原点。
  let minX = Infinity;
  let minY = Infinity;
  let maxX = -Infinity;
  let maxY = -Infinity;
  for (const [id, point] of result) {
    const node = nodeById.get(id)!;
    minX = Math.min(minX, point.cx - node.width / 2);
    maxX = Math.max(maxX, point.cx + node.width / 2);
    minY = Math.min(minY, point.cy - node.height / 2);
    maxY = Math.max(maxY, point.cy + node.height / 2);
  }
  const shiftX = (minX + maxX) / 2;
  const shiftY = (minY + maxY) / 2;
  for (const point of result.values()) {
    point.cx -= shiftX;
    point.cy -= shiftY;
  }
  return result;
}
