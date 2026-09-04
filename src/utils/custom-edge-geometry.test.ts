import { describe, expect, it } from "vitest";

import {
  arrowDirection,
  computeControlPoints,
  midpointSpeed,
  midpointTangent,
  rectChordHalfLength,
  splitBezierAtT,
  toPathString,
  type BezierPoints,
} from "./custom-edge-geometry";

describe("computeControlPoints", () => {
  it("水平边：控制点沿 handle 轴向偏移水平投影的一半", () => {
    // 测试意图：锁定既定曲线形状（curvature=0.5、偏移距离 |dx|/2），防止后续修改意外改变所有边的外观。
    const points = computeControlPoints(0, 0, 200, 0, "right", "left");
    expect(points.p0).toEqual({ x: 0, y: 0 });
    expect(points.cp1).toEqual({ x: 100, y: 0 });
    expect(points.cp2).toEqual({ x: 100, y: 0 });
    expect(points.p1).toEqual({ x: 200, y: 0 });
  });

  it("垂直边：控制距离为 0，控制点与端点重合（曲线退化为直线）", () => {
    // 测试意图：锁定垂直边退化为直线的既定外观；箭头朝向等派生量必须容忍该退化（见 arrowDirection 用例）。
    const points = computeControlPoints(0, 0, 0, 180, "bottom", "top");
    expect(points.cp1).toEqual(points.p0);
    expect(points.cp2).toEqual(points.p1);
  });
});

describe("midpointSpeed", () => {
  it("垂直边：中点速度为直线长度的 1.5 倍", () => {
    // 测试意图：量化缺口换算失真的根源——退化垂直边 |B′(0.5)| = 1.5L；若用控制多边形长度（=L）换算会把标签缺口放大约 1.5 倍。
    const points = computeControlPoints(0, 0, 0, 180, "bottom", "top");
    expect(midpointSpeed(points)).toBeCloseTo(270);
  });

  it("水平边：中点速度为直线长度的 0.75 倍", () => {
    // 测试意图：与垂直边对照——两种朝向下中点速度不同，直接用速度换算缺口才能使缺口大小与朝向无关。
    const points = computeControlPoints(0, 0, 200, 0, "right", "left");
    expect(midpointSpeed(points)).toBeCloseTo(150);
  });

  it("两端点重合：返回 0（组件层 0.08 兜底的前提）", () => {
    // 测试意图：失败/边界路径——节点完全重叠的病理场景下不能产生除零。
    const points = computeControlPoints(50, 50, 50, 50, "right", "left");
    expect(midpointSpeed(points)).toBe(0);
  });
});

describe("midpointTangent", () => {
  it("垂直边：切向垂直，模长为直线长度的 1.5 倍", () => {
    // 测试意图：切向同时供应缺口弦长的方向与参数速度——垂直边方向必须为 (0,1) 方向，垂直边缺口才能按标签高度计算。
    const tangent = midpointTangent(computeControlPoints(0, 0, 0, 180, "bottom", "top"));
    expect(tangent.x).toBeCloseTo(0);
    expect(tangent.y).toBeCloseTo(270);
  });

  it("水平边：切向水平，模长为直线长度的 0.75 倍", () => {
    // 测试意图：与垂直边对照——水平边方向为 (1,0) 方向，缺口按标签宽度计算（与旧行为一致）。
    const tangent = midpointTangent(computeControlPoints(0, 0, 200, 0, "right", "left"));
    expect(tangent.x).toBeCloseTo(150);
    expect(tangent.y).toBeCloseTo(0);
  });

  it("两端点重合：返回零向量（弦长按水平兜底的前提）", () => {
    // 测试意图：失败/边界路径——曲线完全退化时切向为零向量，调用方不得对其归一化。
    const tangent = midpointTangent(computeControlPoints(50, 50, 50, 50, "right", "left"));
    expect(tangent).toEqual({ x: 0, y: 0 });
  });
});

describe("rectChordHalfLength", () => {
  it("水平方向：弦半长等于矩形半宽", () => {
    // 测试意图：水平边穿过标签的宽度方向——缺口与按标签宽度计算旧行为一致。
    expect(rectChordHalfLength(30, 10, { x: 150, y: 0 })).toBeCloseTo(30);
  });

  it("垂直方向：弦半长等于矩形半高", () => {
    // 测试意图：核心场景——垂直边只穿过标签的高度方向，缺口不再按宽度放大。
    expect(rectChordHalfLength(30, 10, { x: 0, y: 270 })).toBeCloseTo(10);
  });

  it("45 度方向穿过宽矩形：受高度限制", () => {
    // 测试意图：对角边的过渡行为——宽矩形（半宽 30、半高 10）在 45 度方向从上下边穿出，弦半长 = 半高·√2。
    expect(rectChordHalfLength(30, 10, { x: 1, y: 1 })).toBeCloseTo(10 * Math.SQRT2);
  });

  it("45 度方向穿过高矩形：受宽度限制", () => {
    // 测试意图：对角边的过渡行为——高矩形（半宽 10、半高 30）在 45 度方向从左右边穿出，弦半长 = 半宽·√2。
    expect(rectChordHalfLength(10, 30, { x: 1, y: 1 })).toBeCloseTo(10 * Math.SQRT2);
  });

  it("零向量方向：按水平处理返回半宽", () => {
    // 测试意图：失败/边界路径——曲线完全退化（p0==p1）时不得产生除零，弦长退化为半宽。
    expect(rectChordHalfLength(30, 10, { x: 0, y: 0 })).toBe(30);
  });
});

describe("splitBezierAtT", () => {
  it("t=0.5 拆分：拆分点为曲线中点，前后两段首尾相接且覆盖原端点", () => {
    // 测试意图：标签缺口依赖拆分正确性——前半段从 p0 出发、两段在拆分点相接、后半段到 p1 结束。
    const points = computeControlPoints(0, 0, 0, 180, "bottom", "top");
    const { first, second, point } = splitBezierAtT(points, 0.5);
    expect(point.x).toBeCloseTo(0);
    expect(point.y).toBeCloseTo(90);
    expect(first[0]).toEqual(points.p0);
    expect(first[3]).toEqual(point);
    expect(second[0]).toEqual(point);
    expect(second[3]).toEqual(points.p1);
  });
});

describe("arrowDirection", () => {
  it("垂直边（目标 top handle）：切向退化为零向量时按 handle 轴向返回 (0,1)", () => {
    // 测试意图：核心回归——修复前垂直边 dx=0 导致切向为零向量、箭头被整体丢弃；兜底朝向必须与 handle 轴向一致（向下进入节点）。
    const points = computeControlPoints(0, 0, 0, 180, "bottom", "top");
    const dir = arrowDirection(points, "top");
    expect(dir.x).toBeCloseTo(0);
    expect(dir.y).toBeCloseTo(1);
  });

  it("垂直边挂左右 handle：退化时按目标 handle 轴向返回水平朝向", () => {
    // 测试意图：退化兜底的另一朝向——目标为 left handle 时箭头向右 (+1,0) 进入节点。
    const points = computeControlPoints(0, 0, 0, 180, "right", "left");
    const dir = arrowDirection(points, "left");
    expect(dir.x).toBeCloseTo(1);
    expect(dir.y).toBeCloseTo(0);
  });

  it("非退化边：四个目标 handle 朝向均沿轴向指向节点", () => {
    // 测试意图：锁定正常路径行为——cp2 恒位于 p1 的 handle 轴线上，朝向恒为轴向；与退化兜底结果一致，节点拖动经过 dx=0 时朝向不跳变。
    const cases: Array<{ points: BezierPoints; target: string; expected: { x: number; y: number } }> = [
      { points: computeControlPoints(0, 0, 100, 180, "bottom", "top"), target: "top", expected: { x: 0, y: 1 } },
      { points: computeControlPoints(0, 180, 100, 0, "top", "bottom"), target: "bottom", expected: { x: 0, y: -1 } },
      { points: computeControlPoints(0, 0, 200, 50, "right", "left"), target: "left", expected: { x: 1, y: 0 } },
      { points: computeControlPoints(200, 0, 0, 50, "left", "right"), target: "right", expected: { x: -1, y: 0 } },
    ];
    for (const { points, target, expected } of cases) {
      const dir = arrowDirection(points, target);
      expect(dir.x).toBeCloseTo(expected.x);
      expect(dir.y).toBeCloseTo(expected.y);
    }
  });
});

describe("toPathString", () => {
  it("生成 M/C 形式的三次贝塞尔路径", () => {
    // 测试意图：冒烟——SVG path 格式契约（边的 path 元素直接消费该字符串）。
    expect(toPathString({ x: 0, y: 0 }, { x: 1, y: 0 }, { x: 2, y: 0 }, { x: 3, y: 0 })).toBe(
      "M 0,0 C 1,0 2,0 3,0",
    );
  });
});
