/**
 * Pass 6 · 碰撞松弛：消除不同子树偶然相撞产生的残余重叠。
 *
 * 确定性松弛：按 id 序遍历节点对，AABB 重叠（含边距）时沿重叠较小的轴
 * 对称推开（中心重合时按遍历序定向）；提前收敛即停止。
 * 轮次上限固定，保证完全确定性（布局管线的不变量②）。
 */

import type { RadialLayoutNode, RadialLayoutPoint } from "./types";
import { compareId } from "./utils";

/** 碰撞松弛迭代轮次上限（固定上限保证完全确定性；通常远未用满即收敛）。 */
const COLLISION_ITERATION_LIMIT = 50;

/**
 * 就地执行确定性碰撞松弛。
 *
 * @param nodeIds 分量内节点 id 列表。
 * @param positions 节点 id → 局部中心坐标（就地修改）。
 * @param nodeById 节点 id → 节点输入。
 * @param margin 节点间最小边距（px）。
 */
export function resolveCollisions(
  nodeIds: string[],
  positions: Map<string, RadialLayoutPoint>,
  nodeById: Map<string, RadialLayoutNode>,
  margin: number,
): void {
  const sorted = [...nodeIds].sort(compareId);
  for (let round = 0; round < COLLISION_ITERATION_LIMIT; round++) {
    let moved = false;
    for (let i = 0; i < sorted.length; i++) {
      for (let j = i + 1; j < sorted.length; j++) {
        const pointA = positions.get(sorted[i])!;
        const pointB = positions.get(sorted[j])!;
        const nodeA = nodeById.get(sorted[i])!;
        const nodeB = nodeById.get(sorted[j])!;
        const overlapX =
          (nodeA.width + nodeB.width) / 2 +
          margin -
          Math.abs(pointA.cx - pointB.cx);
        const overlapY =
          (nodeA.height + nodeB.height) / 2 +
          margin -
          Math.abs(pointA.cy - pointB.cy);
        if (overlapX <= 0 || overlapY <= 0) {
          continue;
        }
        moved = true;
        // 沿重叠较小的轴对称推开，位移最小化。
        if (overlapX <= overlapY) {
          const shift = overlapX / 2;
          if (pointA.cx <= pointB.cx) {
            pointA.cx -= shift;
            pointB.cx += shift;
          } else {
            pointA.cx += shift;
            pointB.cx -= shift;
          }
        } else {
          const shift = overlapY / 2;
          if (pointA.cy <= pointB.cy) {
            pointA.cy -= shift;
            pointB.cy += shift;
          } else {
            pointA.cy += shift;
            pointB.cy -= shift;
          }
        }
      }
    }
    if (!moved) {
      return;
    }
  }
}
