/**
 * CustomEdge 的贝塞尔曲线几何纯计算。
 *
 * 边的形状为三次贝塞尔曲线：端点 p0/p1 为源/目标连接桩坐标，
 * 控制点沿各自 handle 轴向偏移（偏移距离为水平投影 |dx| 的一半，即 curvature=0.5）。
 * 本模块只包含纯函数，坐标单位为 flow 坐标；响应式与 DOM 测量由组件层负责。
 */

/** 二维向量 */
export interface Vec2 {
  x: number;
  y: number;
}

/** 三次贝塞尔曲线的四个几何点：起点、两个控制点、终点 */
export interface BezierPoints {
  p0: Vec2;
  cp1: Vec2;
  cp2: Vec2;
  p1: Vec2;
}

/** 线性插值 */
function lerp(a: Vec2, b: Vec2, t: number): Vec2 {
  return { x: a.x + (b.x - a.x) * t, y: a.y + (b.y - a.y) * t };
}

/**
 * 根据 handle 方位计算单个控制点：从 pos 出发沿 handle 轴向偏移 distance。
 * 入参 handlePosition 取值为 top/right/bottom/left。
 */
function computeControlPoint(pos: Vec2, handlePosition: string, distance: number): Vec2 {
  const cp = { x: pos.x, y: pos.y };
  switch (handlePosition) {
    case "right":
      cp.x += distance;
      break;
    case "left":
      cp.x -= distance;
      break;
    case "bottom":
      cp.y += distance;
      break;
    case "top":
      cp.y -= distance;
      break;
  }
  return cp;
}

/**
 * 计算贝塞尔曲线的控制点。
 *
 * 入参为源/目标连接桩的 flow 坐标与各自 handle 方位（top/right/bottom/left）。
 * 控制点偏移距离为水平投影 |dx| × curvature（curvature=0.5）。
 * 注意：垂直边（dx=0）时偏移距离为 0，控制点与端点重合，曲线退化为直线——
 * 这是既定的外观行为，调用方在计算箭头朝向等派生量时需处理该退化场景（见 arrowDirection）。
 */
export function computeControlPoints(
  sx: number,
  sy: number,
  tx: number,
  ty: number,
  sourcePosition: string,
  targetPosition: string,
): BezierPoints {
  const curvature = 0.5;
  const dx = tx - sx;

  const p0: Vec2 = { x: sx, y: sy };
  const p1: Vec2 = { x: tx, y: ty };

  const cp1 = computeControlPoint(p0, sourcePosition, Math.abs(dx) * curvature);
  const cp2 = computeControlPoint(p1, targetPosition, Math.abs(dx) * curvature);

  return { p0, cp1, cp2, p1 };
}

/**
 * 用 De Casteljau 算法在任意 t 处拆分三次贝塞尔曲线。
 * 返回拆分得到的前后两段曲线（各四个点）以及拆分点坐标。
 */
export function splitBezierAtT(
  points: BezierPoints,
  t: number,
): { first: [Vec2, Vec2, Vec2, Vec2]; second: [Vec2, Vec2, Vec2, Vec2]; point: Vec2 } {
  const { p0, cp1, cp2, p1 } = points;

  // Level 1
  const q0 = lerp(p0, cp1, t);
  const q1 = lerp(cp1, cp2, t);
  const q2 = lerp(cp2, p1, t);

  // Level 2
  const r0 = lerp(q0, q1, t);
  const r1 = lerp(q1, q2, t);

  // Level 3
  const s = lerp(r0, r1, t);

  return {
    first: [p0, q0, r0, s],
    second: [s, r1, q2, p1],
    point: s,
  };
}

/** 将四个几何点转换为 SVG 三次贝塞尔路径字符串 */
export function toPathString(p0: Vec2, cp1: Vec2, cp2: Vec2, p3: Vec2): string {
  return `M ${p0.x},${p0.y} C ${cp1.x},${cp1.y} ${cp2.x},${cp2.y} ${p3.x},${p3.y}`;
}

/**
 * 曲线在中点（t=0.5）处的切向量 B′(0.5)（未归一化，模长即中点参数速度）。
 *
 * 三次贝塞尔的导数 B′(t) = 3(1−t)²(cp1−p0) + 6(1−t)t(cp2−cp1) + 3t²(p1−cp2)，
 * 代入 t=0.5 化简得 3/4·[(p1−p0) + (cp2−cp1)]。
 * 返回零向量当且仅当 p0 与 p1 重合（节点完全重叠的病理场景）。
 */
export function midpointTangent(points: BezierPoints): Vec2 {
  const { p0, cp1, cp2, p1 } = points;
  return {
    x: 0.75 * (p1.x - p0.x + (cp2.x - cp1.x)),
    y: 0.75 * (p1.y - p0.y + (cp2.y - cp1.y)),
  };
}

/**
 * 曲线在中点（t=0.5）处的参数速度 |B′(0.5)|（flow 坐标/单位参数）。
 *
 * 用途：把像素缺口换算为参数间隔 δ = halfGap / speed，使缺口弧长 ≈ 2δ·speed 与边的朝向无关。
 * 不能用控制多边形长度（|cp1−p0|+|cp2−cp1|+|p1−cp2|）替代：它与中点速度的比值随朝向在
 * 0.75（水平边）到 1.5（垂直边）之间连续变化，会导致垂直边的缺口被放大约 1.5 倍。
 */
export function midpointSpeed(points: BezierPoints): number {
  const tangent = midpointTangent(points);
  return Math.hypot(tangent.x, tangent.y);
}

/**
 * 轴对齐矩形沿给定方向过中心点的弦半长：min(halfWidth/|ux|, halfHeight/|uy|)。
 *
 * 入参 halfWidth/halfHeight 为矩形半宽/半高（flow 坐标），direction 为方向向量（无需归一化）。
 * 用途：标签缺口只需覆盖标签矩形被曲线穿过的部分——水平边穿过宽度方向、垂直边穿过高度方向、
 * 对角边取两者的过渡；方向的某个分量接近 0 时对应轴不限制弦长（对应项为无穷大）。
 * direction 为零向量（曲线完全退化）时弦长无意义，按水平方向处理，返回 halfWidth。
 * 返回：弦半长（flow 坐标）。
 */
export function rectChordHalfLength(halfWidth: number, halfHeight: number, direction: Vec2): number {
  const len = Math.hypot(direction.x, direction.y);
  if (len === 0) return halfWidth;
  const ux = Math.abs(direction.x) / len;
  const uy = Math.abs(direction.y) / len;
  const tx = ux > 0 ? halfWidth / ux : Infinity;
  const ty = uy > 0 ? halfHeight / uy : Infinity;
  return Math.min(tx, ty);
}

/**
 * 箭头朝向单位向量，取曲线末端切向（cp2→p1）。
 *
 * 入参 targetPosition 为目标 handle 方位（top/right/bottom/left）。
 * 退化场景处理：控制点与终点重合（如垂直边 dx=0 时控制距离为 0）时切向为零向量，
 * 此时按目标 handle 轴向确定朝向——按控制点的构造，非退化时 cp2 恒位于 p1 的 handle 轴线上，
 * cp2→p1 的方向本来就恒等于 handle 轴向，故该兜底与正常情形连续一致，
 * 节点被拖动经过 dx=0 时箭头朝向不会跳变。
 */
export function arrowDirection(points: BezierPoints, targetPosition: string): Vec2 {
  const { cp2, p1 } = points;
  const dx = p1.x - cp2.x;
  const dy = p1.y - cp2.y;
  const len = Math.hypot(dx, dy);
  if (len > 0) return { x: dx / len, y: dy / len };
  switch (targetPosition) {
    case "top":
      return { x: 0, y: 1 };
    case "bottom":
      return { x: 0, y: -1 };
    case "right":
      return { x: -1, y: 0 };
    default:
      // "left" 及其它取值：与组件层 targetPosition 的默认值 "left" 保持一致
      return { x: 1, y: 0 };
  }
}
