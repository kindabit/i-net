/**
 * 罗盘锚定分层布局算法（纯函数模块，不依赖 Vue / vue-flow）。
 *
 * 适用于有向无环图：节点按最长路径分层，父节点必然先于子节点放置。
 * 子节点理想位 = 各入边锚点的均值，锚点 = 父节点位置 + 锚向 × ringSpacing：
 * 锚向优先取 sourcePort 方向（父节点选择的出口侧是最强信号），
 * 其次取 targetPort 的反向；无端口时取父节点的流向方向；
 * 孤立根节点的无端口子代绕父节点均布（过密时自动扩大均布半径）。
 * 同父同锚向的兄弟节点沿锚向的垂直方向等距对称排开（列或行）；
 * 不同子树偶然相撞产生的残余重叠由固定轮次的确定性碰撞松弛消除。
 * 布局结果横平竖直、父子恒距、兄弟成列成行、多父取锚点重心。
 * 互不连通的分量各自完成布局后按大小降序网格平铺；没有任何边连接的孤立
 * 节点（以及防御性识别出的环相关节点）单独在主区域右侧排成网格。
 *
 * 算法完全确定性：同一输入必然产生同一输出（所有排序平局均按节点 id 决胜，
 * 所有浮点求和均先排序以消除输入顺序带来的舍入差异）。
 * 输出为节点中心坐标，调用方需自行换算为节点左上角坐标及 snap 网格对齐。
 *
 * 【历史命名】模块名与导出名保留了初版"径向同心环"时期的 radial 字样，
 * 现算法与同心环无关；为保持调用方（use-auto-layout.ts）稳定未更名。
 *
 * 【维护指引（供后续接手者）】
 * - 两条不变量不可破坏：① 父节点先于子节点放置（最长路径分层保证，
 *   布局每一步都假设 positions 中能取到父节点坐标）；② 完全确定性
 *   （所有遍历按 id 排序、所有浮点求和先排序、松弛轮次固定）。
 *   修改任何遍历/求和逻辑前先检查是否触碰这两条，确定性有测试兜底。
 * - layoutComponent 的放置管线按序执行：根节点成行 → 均布登记 → 理想位 →
 *   兄弟展开 → 端口轴向间距约束 →（全部层完成后）碰撞松弛。
 *   后续层的锚点基于前序步骤修正后的坐标计算，新增步骤时注意插入位置
 *   对下游锚点的连锁影响。
 * - 已知边界（如需进一步优化可从这些点入手）：
 *   ① 边跨节点仅在布局侧缓解，完全消除需改 CustomEdge.vue 为正交路由；
 *   ② 间距约束先于碰撞松弛执行，拥挤场景松弛可能轻微侵蚀间距，
 *      严格保证需约束-松弛交替求解；
 *   ③ 病态输入（同一节点左右/上下矛盾端口）按 left/top 覆盖
 *      right/bottom 取舍；④ 无端口边的方向推断（流向/均布）仅是兜底，
 *      真实数据（CanvasView/CanvasUniverseView）每条边都带端口。
 */

/** 布局输入：一个节点的 id 与渲染尺寸。 */
export interface RadialLayoutNode {
  /** 节点 id，同一输入内必须唯一。 */
  id: string;
  /** 节点渲染宽度（px）。 */
  width: number;
  /** 节点渲染高度（px）。 */
  height: number;
}

/** 端口方向：边在节点上的进出方位（画布坐标系，+x 向右、+y 向下）。 */
export type RadialPortDirection = "top" | "bottom" | "left" | "right";

/** 布局输入：一条有向边。方向决定层级关系（source 为父节点，先于 target 放置）。 */
export interface RadialLayoutEdge {
  /** 源节点 id。 */
  source: string;
  /** 目标节点 id。 */
  target: string;
  /** 可选：边离开源节点的端口方向（如 right 表示从源节点右侧引出）。 */
  sourcePort?: RadialPortDirection;
  /** 可选：边进入目标节点的端口方向（如 left 表示从目标节点左侧进入）。 */
  targetPort?: RadialPortDirection;
}

/** 布局参数。 */
export interface RadialLayoutConfig {
  /** 父子节点的基础间距（中心距，px）。 */
  ringSpacing: number;
  /** 相邻节点的最小间距（px），用于兄弟排距、根行距与碰撞边距。 */
  nodeMargin: number;
  /** 不同连通分量包围圆之间的间距（px）。 */
  componentGap: number;
  /** 孤立节点网格的单元格边长（px）。 */
  isolatedCell: number;
}

/** 布局输出：节点中心坐标。 */
export interface RadialLayoutPoint {
  cx: number;
  cy: number;
}

/** 默认布局参数，适用于普通画布（数据节点固定尺寸，见 node-size.ts）与画布宇宙。 */
export const DEFAULT_RADIAL_LAYOUT_CONFIG: RadialLayoutConfig = {
  ringSpacing: 300,
  nodeMargin: 40,
  componentGap: 200,
  isolatedCell: 240,
};

/** 并查集：按边（无视方向）把节点聚合为连通分量。 */
class UnionFind {
  private readonly parent = new Map<string, string>();

  /**
   * 查找节点所在集合的根（带路径压缩）。
   *
   * @param id 节点 id。
   * @returns 根节点 id；未加入任何集合的节点以自身为根。
   */
  find(id: string): string {
    const parent = this.parent.get(id);
    if (parent === undefined || parent === id) {
      return id;
    }
    const root = this.find(parent);
    this.parent.set(id, root);
    return root;
  }

  /**
   * 合并两个节点所在的集合。
   *
   * @param a 节点 id。
   * @param b 节点 id。
   */
  union(a: string, b: string): void {
    this.parent.set(this.find(a), this.find(b));
  }
}

/** 字符串 id 的确定性比较器（按 UTF-16 码元序）。 */
function compareId(a: string, b: string): number {
  return a < b ? -1 : a > b ? 1 : 0;
}

/** 端口方向 → 方向角（弧度，画布坐标系：+x 向右、+y 向下）。 */
const PORT_ANGLE: Record<RadialPortDirection, number> = {
  right: 0,
  bottom: Math.PI / 2,
  left: Math.PI,
  top: -Math.PI / 2,
};

/** 一条入边在布局中需要的信息：父节点 id 与可选端口方向。 */
interface IncomingEdge {
  /** 父节点 id。 */
  parent: string;
  /** 可选：边离开父节点的端口方向。 */
  sourcePort?: RadialPortDirection;
  /** 可选：边进入本节点的端口方向。 */
  targetPort?: RadialPortDirection;
}

/** 锚向解析结果：方向角 + 锚点距离。 */
interface AnchorOffset {
  /** 锚向角（弧度，归一化到 [0, 2π)）。 */
  angle: number;
  /** 锚点与父节点的中心距（px）。 */
  distance: number;
}

/**
 * 解析入边的端口方向（sourcePort 优先，其次 targetPort 反向）。
 *
 * @param edge 入边信息（父节点 id 与可选端口方向）。
 * @returns 端口方向；无端口时返回 undefined。
 */
function portDirectionOf(edge: IncomingEdge): RadialPortDirection | undefined {
  if (edge.sourcePort !== undefined) {
    return edge.sourcePort;
  }
  switch (edge.targetPort) {
    case "top":
      return "bottom";
    case "bottom":
      return "top";
    case "left":
      return "right";
    case "right":
      return "left";
    default:
      return undefined;
  }
}

/**
 * 将角度归一化到 [0, 2π)。
 *
 * @param angle 任意角度（弧度）。
 * @returns 归一化后的角度（弧度）。
 */
function normalizeAngle(angle: number): number {
  const fullCircle = 2 * Math.PI;
  return ((angle % fullCircle) + fullCircle) % fullCircle;
}

/**
 * 计算节点的流向方向：自身位置 − 各父节点均值位置的方向。
 *
 * @param id 节点 id。
 * @param positions 已放置节点 id → 局部中心坐标。
 * @param incoming 节点 id → 入边信息列表。
 * @returns 流向方向角（弧度）；无父节点、父节点未放置或向量抵消时返回 undefined。
 */
function flowDirectionOf(
  id: string,
  positions: Map<string, RadialLayoutPoint>,
  incoming: Map<string, IncomingEdge[]>,
): number | undefined {
  const edges = incoming.get(id)!;
  const pos = positions.get(id);
  if (edges.length === 0 || pos === undefined) {
    return undefined;
  }
  // 排序后求和：消除入边输入顺序带来的浮点舍入差异，保证完全确定性。
  const parentPoints = edges
    .map((edge) => positions.get(edge.parent))
    .filter((point): point is RadialLayoutPoint => point !== undefined)
    .sort((a, b) => a.cx - b.cx || a.cy - b.cy);
  if (parentPoints.length === 0) {
    return undefined;
  }
  let sumX = 0;
  let sumY = 0;
  for (const point of parentPoints) {
    sumX += point.cx;
    sumY += point.cy;
  }
  const dx = pos.cx - sumX / parentPoints.length;
  const dy = pos.cy - sumY / parentPoints.length;
  if (dx === 0 && dy === 0) {
    return undefined;
  }
  return Math.atan2(dy, dx);
}

/**
 * 计算单条入边的锚向与锚距。
 *
 * 锚向优先级：sourcePort 方向 → targetPort 反向 → 父节点流向方向 →
 * 无端口子代组均布（角度 = 2π×组内序号/组大小，组过密时锚距突破
 * ringSpacing 扩大至 span/(2·sin(π/组大小)) 保证弦长不重叠；
 * 均布方向与父节点有端口子代的方向重合时整体偏移半格 π/组大小）。
 *
 * @param edge 入边信息（父节点 id 与可选端口方向）。
 * @param childId 本节点 id（用于定位均布组内序号）。
 * @param positions 已放置节点 id → 局部中心坐标（父节点必然已放置）。
 * @param incoming 节点 id → 入边信息列表。
 * @param spreadGroups 父节点 id → 本层无端口子代 id 列表（已按 id 排序）。
 * @param portedAnglesByParent 父节点 id → 其有端口子边的锚向角列表。
 * @param span 节点间距（最大节点尺寸 + nodeMargin，px）。
 * @param config 布局参数。
 * @returns 锚向角（弧度，[0, 2π)）与锚距（px）。
 */
function anchorOffsetOf(
  edge: IncomingEdge,
  childId: string,
  positions: Map<string, RadialLayoutPoint>,
  incoming: Map<string, IncomingEdge[]>,
  spreadGroups: Map<string, string[]>,
  portedAnglesByParent: Map<string, number[]>,
  span: number,
  config: RadialLayoutConfig,
): AnchorOffset {
  const portDirection = portDirectionOf(edge);
  if (portDirection !== undefined) {
    return { angle: PORT_ANGLE[portDirection], distance: config.ringSpacing };
  }
  const flow = flowDirectionOf(edge.parent, positions, incoming);
  if (flow !== undefined) {
    return { angle: normalizeAngle(flow), distance: config.ringSpacing };
  }
  // 孤立根节点的无端口子代：绕父节点均布。
  const group = spreadGroups.get(edge.parent) ?? [childId];
  const count = group.length;
  const index = group.indexOf(childId);
  const fullCircle = 2 * Math.PI;
  // 均布方向与父节点有端口子代的方向重合时，整体偏移半格。
  let phase = 0;
  const ported = portedAnglesByParent.get(edge.parent) ?? [];
  for (const portedAngle of ported) {
    for (let i = 0; i < count; i++) {
      const candidate = (i * fullCircle) / count;
      if (Math.abs(normalizeAngle(candidate - portedAngle)) < 1e-9) {
        phase = Math.PI / count;
      }
    }
  }
  // 锚距按弦长（而非弧长）保证相邻子代不重叠：2r·sin(π/n) ≥ span。
  const distance =
    count >= 2
      ? Math.max(config.ringSpacing, span / (2 * Math.sin(Math.PI / count)))
      : config.ringSpacing;
  return {
    angle: normalizeAngle(phase + (Math.max(index, 0) * fullCircle) / count),
    distance,
  };
}

/** 碰撞松弛迭代轮次上限（固定上限保证完全确定性；通常远未用满即收敛）。 */
const COLLISION_ITERATION_LIMIT = 50;

/**
 * 确定性碰撞松弛：按 id 序遍历节点对，AABB 重叠（含边距）时沿重叠较小的轴
 * 对称推开（中心重合时按遍历序定向）；提前收敛即停止。
 *
 * @param nodeIds 分量内节点 id 列表。
 * @param positions 节点 id → 局部中心坐标（就地修改）。
 * @param nodeById 节点 id → 节点输入。
 * @param margin 节点间最小边距（px）。
 * @returns 无返回值。
 */
function resolveCollisions(
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

/** 单个连通分量的布局结果（局部坐标，分量中心为原点）。 */
interface ComponentLayout {
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
 * 第 0 层（入度为 0 的根）：按 id 序水平排成一行并以分量原点居中。
 * 第 k 层（k≥1）：节点理想位 = 全部入边锚点的均值（锚向锚距规则见
 * anchorOffsetOf）；同父同锚向的单入边兄弟沿锚向垂直方向等距对称排开；
 * 随后按端口方向强制每条有端口入边的父子最小间距（详见循环内第四遍注释）；
 * 全部层放置完成后做确定性碰撞松弛消除残余重叠。
 *
 * @param nodeIds 分量内节点 id 列表（必须全部成功分层）。
 * @param layer 节点 id → 层号（最长路径分层结果）。
 * @param incoming 节点 id → 入边信息列表（父节点 id 与可选端口方向）。
 * @param nodeById 节点 id → 节点输入。
 * @param config 布局参数。
 * @returns 分量局部布局结果。
 */
function layoutComponent(
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

/**
 * 计算 DAG 的罗盘锚定分层布局。
 *
 * 流程：最长路径分层（Kahn 拓扑）→ 并查集划分连通分量 → 每个分量各自
 * 罗盘锚定布局 → 分量按大小降序（平局按最小 id）网格平铺 →
 * 孤立节点在主区域右侧排网格 → 整体平移使包围盒中心位于原点。
 * 防御：拓扑排序未覆盖的节点（理论上不可能的环及其下游）降级为孤立节点。
 *
 * @param nodes 节点列表，id 必须唯一。
 * @param edges 边列表；端点不存在、自环或重复的边会被忽略。
 * @param config 布局参数。
 * @returns 节点 id → 节点中心坐标，覆盖全部输入节点；空输入返回空 Map。
 */
export function computeRadialLayout(
  nodes: RadialLayoutNode[],
  edges: RadialLayoutEdge[],
  config: RadialLayoutConfig,
): Map<string, RadialLayoutPoint> {
  const result = new Map<string, RadialLayoutPoint>();
  if (nodes.length === 0) {
    return result;
  }
  const nodeById = new Map(nodes.map((node) => [node.id, node]));

  // 过滤无效边（端点缺失、自环）并去重，保证后续步骤的输入干净。
  const validEdges: RadialLayoutEdge[] = [];
  const seenEdgeKeys = new Set<string>();
  for (const edge of edges) {
    if (edge.source === edge.target) {
      continue;
    }
    if (!nodeById.has(edge.source) || !nodeById.has(edge.target)) {
      continue;
    }
    const key = `${edge.source}→${edge.target}`;
    if (seenEdgeKeys.has(key)) {
      continue;
    }
    seenEdgeKeys.add(key);
    validEdges.push(edge);
  }

  const outgoing = new Map<string, string[]>();
  const incoming = new Map<string, IncomingEdge[]>();
  const indegree = new Map<string, number>();
  for (const node of nodes) {
    outgoing.set(node.id, []);
    incoming.set(node.id, []);
    indegree.set(node.id, 0);
  }
  for (const edge of validEdges) {
    outgoing.get(edge.source)!.push(edge.target);
    incoming.get(edge.target)!.push({
      parent: edge.source,
      sourcePort: edge.sourcePort,
      targetPort: edge.targetPort,
    });
    indegree.set(edge.target, indegree.get(edge.target)! + 1);
  }

  // Kahn 拓扑 + 最长路径分层：layer[v] = max(layer[u]) + 1，保证父节点先于子节点放置。
  const layer = new Map<string, number>();
  const remainingIndegree = new Map(indegree);
  const queue = nodes
    .filter((node) => indegree.get(node.id) === 0)
    .map((node) => node.id)
    .sort(compareId);
  while (queue.length > 0) {
    const id = queue.shift()!;
    const currentLayer = layer.get(id) ?? 0;
    layer.set(id, currentLayer);
    for (const next of outgoing.get(id)!) {
      const candidate = currentLayer + 1;
      if (candidate > (layer.get(next) ?? 0)) {
        layer.set(next, candidate);
      }
      const degree = remainingIndegree.get(next)! - 1;
      remainingIndegree.set(next, degree);
      if (degree === 0) {
        queue.push(next);
      }
    }
  }

  // 并查集按边（无视方向）划分连通分量。
  const unionFind = new UnionFind();
  for (const edge of validEdges) {
    unionFind.union(edge.source, edge.target);
  }
  const componentIds = new Map<string, string[]>();
  for (const node of nodes) {
    const root = unionFind.find(node.id);
    const list = componentIds.get(root) ?? [];
    list.push(node.id);
    componentIds.set(root, list);
  }

  // 每个分量内：成功分层的节点参与布局（不足 2 个按孤立处理）；
  // 未分层节点（环相关，防御性场景）降级为孤立节点。
  const components: ComponentLayout[] = [];
  const isolatedIds: string[] = [];
  for (const ids of componentIds.values()) {
    const acyclic = ids.filter((id) => layer.has(id));
    isolatedIds.push(...ids.filter((id) => !layer.has(id)));
    if (acyclic.length >= 2) {
      components.push(
        layoutComponent(acyclic, layer, incoming, nodeById, config),
      );
    } else {
      isolatedIds.push(...acyclic);
    }
  }

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
