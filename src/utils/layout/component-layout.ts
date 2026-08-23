/**
 * Pass 5 · 分量布局：对一个无环连通分量执行罗盘锚定分层布局。
 *
 * 第 0 层（入度为 0 的根）：按 id 序水平排成一行并以分量原点居中。
 * 第 k 层（k≥1）按序执行四遍：
 *   ① 均布登记——登记需要均布方向的无端口子代；
 *   ② 理想位——节点理想位 = 全部入边锚点的均值（锚向锚距规则见
 *      anchor.ts），并记录单入边节点的锚向供兄弟分组；
 *   ③ 兄弟展开——同父同锚向的单入边兄弟沿锚向垂直方向等距对称排开；
 *   ④ 端口轴向间距约束——按端口方向强制每条有端口入边的父子最小间距
 *      （多父均值可能侵蚀端口承诺，如菱形带捷径场景）。
 * 全部层放置完成后做确定性碰撞松弛（collision.ts）消除残余重叠。
 *
 * 后续层的锚点基于前序步骤修正后的坐标计算，新增步骤时注意插入位置
 * 对下游锚点的连锁影响。
 */

import { anchorOffsetOf, flowDirectionOf, portDirectionOf } from "./anchor";
import { resolveCollisions } from "./collision";
import {
  PORT_ANGLE,
  type IncomingEdge,
  type RadialLayoutConfig,
  type RadialLayoutNode,
  type RadialLayoutPoint,
} from "./types";
import { compareId, normalizeAngle } from "./utils";

/** 单个连通分量的布局结果（局部坐标，分量中心为原点）。 */
export interface ComponentLayout {
  /** 分量内节点 id 列表。 */
  nodeIds: string[];
  /** 节点 id → 局部中心坐标。 */
  positions: Map<string, RadialLayoutPoint>;
  /** 包围圆半径（含节点自身尺寸）。 */
  boundingRadius: number;
}

/**
 * 对一个无环连通分量执行罗盘锚定布局。
 *
 * @param nodeIds 分量内节点 id 列表（必须全部成功分层）。
 * @param layer 节点 id → 层号（最长路径分层结果）。
 * @param incoming 节点 id → 入边信息列表（父节点 id 与可选端口方向）。
 * @param nodeById 节点 id → 节点输入。
 * @param config 布局参数。
 * @returns 分量局部布局结果。
 */
export function layoutComponent(
  nodeIds: string[],
  layer: Map<string, number>,
  incoming: Map<string, IncomingEdge[]>,
  nodeById: Map<string, RadialLayoutNode>,
  config: RadialLayoutConfig,
): ComponentLayout {
  const byLayer = new Map<number, string[]>();
  for (const id of nodeIds) {
    const nodeLayer = layer.get(id)!;
    const list = byLayer.get(nodeLayer) ?? [];
    list.push(id);
    byLayer.set(nodeLayer, list);
  }
  const maxLayer = Math.max(...byLayer.keys());
  const positions = new Map<string, RadialLayoutPoint>();

  // 节点间距统一按分量内最大节点尺寸计算，保证任何节点组合都不重叠。
  const span =
    Math.max(
      ...nodeIds.map((id) => {
        const node = nodeById.get(id)!;
        return Math.max(node.width, node.height);
      }),
    ) + config.nodeMargin;

  // 父节点 → 其有端口子边的锚向角列表（用于无端口均布方向的避让）。
  const portedAnglesByParent = new Map<string, number[]>();
  for (const id of nodeIds) {
    for (const edge of incoming.get(id)!) {
      let angle: number | undefined;
      if (edge.sourcePort !== undefined) {
        angle = PORT_ANGLE[edge.sourcePort];
      } else if (edge.targetPort !== undefined) {
        angle = normalizeAngle(PORT_ANGLE[edge.targetPort] + Math.PI);
      }
      if (angle !== undefined) {
        const list = portedAnglesByParent.get(edge.parent) ?? [];
        list.push(angle);
        portedAnglesByParent.set(edge.parent, list);
      }
    }
  }

  // 第 0 层：根节点按 id 序水平成行，整行以分量原点居中。
  const roots = [...(byLayer.get(0) ?? [])].sort(compareId);
  roots.forEach((id, index) => {
    positions.set(id, {
      cx: (index - (roots.length - 1) / 2) * span,
      cy: 0,
    });
  });

  for (let currentLayer = 1; currentLayer <= maxLayer; currentLayer++) {
    const ids = byLayer.get(currentLayer);
    // 最长路径分层在连通无环分量内层号必然连续，此分支仅作防御。
    if (!ids) {
      continue;
    }
    const sorted = [...ids].sort(compareId);

    // 第一遍：登记需要均布方向的无端口子代（父节点无流向可用的孤立根场景）。
    const spreadGroups = new Map<string, string[]>();
    for (const id of sorted) {
      for (const edge of incoming.get(id)!) {
        if (edge.sourcePort !== undefined || edge.targetPort !== undefined) {
          continue;
        }
        if (flowDirectionOf(edge.parent, positions, incoming) !== undefined) {
          continue;
        }
        const group = spreadGroups.get(edge.parent) ?? [];
        group.push(id);
        spreadGroups.set(edge.parent, group);
      }
    }

    // 第二遍：计算理想位（全部入边锚点的均值，锚点排序后求和保证确定性），
    // 并记录单入边节点的锚向（供兄弟分组使用）。
    const idealOf = new Map<string, RadialLayoutPoint>();
    const anchorAngleById = new Map<string, number>();
    for (const id of sorted) {
      const edges = incoming.get(id)!;
      const anchors = edges.map((edge) => {
        const offset = anchorOffsetOf(
          edge,
          id,
          positions,
          incoming,
          spreadGroups,
          portedAnglesByParent,
          span,
          config,
        );
        const parentPos = positions.get(edge.parent)!;
        return {
          angle: offset.angle,
          cx: parentPos.cx + Math.cos(offset.angle) * offset.distance,
          cy: parentPos.cy + Math.sin(offset.angle) * offset.distance,
        };
      });
      anchors.sort((a, b) => a.cx - b.cx || a.cy - b.cy);
      let sumX = 0;
      let sumY = 0;
      for (const anchor of anchors) {
        sumX += anchor.cx;
        sumY += anchor.cy;
      }
      idealOf.set(id, {
        cx: sumX / anchors.length,
        cy: sumY / anchors.length,
      });
      if (edges.length === 1) {
        anchorAngleById.set(id, anchors[0].angle);
      }
    }

    // 第三遍：同父同锚向的单入边兄弟沿锚向垂直方向等距对称排开。
    const groupsByAnchor = new Map<string, string[]>();
    for (const id of sorted) {
      const angle = anchorAngleById.get(id);
      if (angle === undefined) {
        continue;
      }
      const key = `${incoming.get(id)![0].parent} ${angle}`;
      const group = groupsByAnchor.get(key) ?? [];
      group.push(id);
      groupsByAnchor.set(key, group);
    }
    for (const id of sorted) {
      const ideal = idealOf.get(id)!;
      const angle = anchorAngleById.get(id);
      if (angle === undefined) {
        positions.set(id, ideal);
        continue;
      }
      // angle 已登记即必然存在对应分组，此分支仅作防御。
      const group = groupsByAnchor.get(`${incoming.get(id)![0].parent} ${angle}`);
      if (group === undefined || group.length === 1) {
        positions.set(id, ideal);
        continue;
      }
      const index = group.indexOf(id);
      const offset = (index - (group.length - 1) / 2) * span;
      positions.set(id, {
        cx: ideal.cx + Math.cos(angle + Math.PI / 2) * offset,
        cy: ideal.cy + Math.sin(angle + Math.PI / 2) * offset,
      });
    }

    // 第四遍：端口方向最小间距约束。多父节点的锚点均值可能违背某条入边的
    // 端口方向承诺（如菱形带捷径场景，捷径父代的锚点把多父节点拽回近侧），
    // 这里按端口轴向强制子节点与父节点保持 ringSpacing 间距。
    // 约束为"≥"型且父节点坐标已最终确定，每层单趟即可收敛；
    // 同一节点轴向约束矛盾时（病态输入），left/top 覆盖 right/bottom，保证确定性。
    for (const id of sorted) {
      const point = positions.get(id)!;
      let minCx = -Infinity;
      let maxCx = Infinity;
      let minCy = -Infinity;
      let maxCy = Infinity;
      for (const edge of incoming.get(id)!) {
        const direction = portDirectionOf(edge);
        if (direction === undefined) {
          continue;
        }
        const parentPos = positions.get(edge.parent)!;
        if (direction === "right") {
          minCx = Math.max(minCx, parentPos.cx + config.ringSpacing);
        } else if (direction === "left") {
          maxCx = Math.min(maxCx, parentPos.cx - config.ringSpacing);
        } else if (direction === "bottom") {
          minCy = Math.max(minCy, parentPos.cy + config.ringSpacing);
        } else {
          maxCy = Math.min(maxCy, parentPos.cy - config.ringSpacing);
        }
      }
      point.cx = Math.min(Math.max(point.cx, minCx), maxCx);
      point.cy = Math.min(Math.max(point.cy, minCy), maxCy);
    }
  }

  // 不同子树偶然相撞产生的残余重叠由确定性松弛消除。
  resolveCollisions(nodeIds, positions, nodeById, config.nodeMargin);

  let boundingRadius = 0;
  for (const id of nodeIds) {
    const point = positions.get(id)!;
    const node = nodeById.get(id)!;
    boundingRadius = Math.max(
      boundingRadius,
      Math.hypot(point.cx, point.cy) + Math.max(node.width, node.height) / 2,
    );
  }
  return { nodeIds, positions, boundingRadius };
}
