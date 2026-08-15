import { describe, expect, it } from "vitest";

import {
  computeRadialLayout,
  type RadialLayoutConfig,
  type RadialLayoutEdge,
  type RadialLayoutNode,
  type RadialLayoutPoint,
  type RadialPortDirection,
} from "./radial-layout";

/** 测试用固定布局参数：数值取整便于推算预期坐标。 */
const CONFIG: RadialLayoutConfig = {
  ringSpacing: 300,
  nodeMargin: 40,
  componentGap: 200,
  isolatedCell: 240,
};

/** 构造测试用节点（与普通画布的 160×80 固定尺寸一致）。 */
function node(id: string): RadialLayoutNode {
  return { id, width: 160, height: 80 };
}

/** 构造测试用边。 */
function edge(source: string, target: string): RadialLayoutEdge {
  return { source, target };
}

/** 构造带端口方向的测试用边。 */
function portedEdge(
  source: string,
  target: string,
  sourcePort?: RadialPortDirection,
  targetPort?: RadialPortDirection,
): RadialLayoutEdge {
  return { source, target, sourcePort, targetPort };
}

/** 计算两个节点中心坐标之间的欧氏距离。 */
function distance(a: RadialLayoutPoint, b: RadialLayoutPoint): number {
  return Math.hypot(a.cx - b.cx, a.cy - b.cy);
}

describe("computeRadialLayout", () => {
  it("空图返回空结果", () => {
    // 意图：空输入是合法边界，算法应正常返回而不报错。
    expect(computeRadialLayout([], [], CONFIG).size).toBe(0);
  });

  it("单节点位于原点", () => {
    // 意图：唯一节点没有边，属于孤立节点，归一化后应恰好位于包围盒中心（原点）。
    const result = computeRadialLayout([node("a")], [], CONFIG);
    expect(result.size).toBe(1);
    const point = result.get("a")!;
    expect(point.cx).toBeCloseTo(0, 6);
    expect(point.cy).toBeCloseTo(0, 6);
  });

  it("链式图：各层等距、共线、同向", () => {
    // 意图：A→B→C 应沿同一方向逐层向外展开，相邻父子间距等于 ringSpacing，
    // 验证无端口边沿父节点流向方向锚定（B 在 A 正东、C 继续向东）。
    const result = computeRadialLayout(
      [node("a"), node("b"), node("c")],
      [edge("a", "b"), edge("b", "c")],
      CONFIG,
    );
    const a = result.get("a")!;
    const b = result.get("b")!;
    const c = result.get("c")!;
    expect(distance(a, b)).toBeCloseTo(300, 6);
    expect(distance(b, c)).toBeCloseTo(300, 6);
    // 共线：向量 AB 与 BC 的叉积为 0；同向：点积为正。
    const abx = b.cx - a.cx;
    const aby = b.cy - a.cy;
    const bcx = c.cx - b.cx;
    const bcy = c.cy - b.cy;
    expect(abx * bcy - aby * bcx).toBeCloseTo(0, 6);
    expect(abx * bcx + aby * bcy).toBeGreaterThan(0);
  });

  it("星型图：无端口子代绕根节点均布成正方形", () => {
    // 意图：根节点 4 个无端口子代绕根均布（相邻角差 90°，即正方形），
    // 验证孤立根的无端口子代均布方向分配与恒距。
    const children = ["c1", "c2", "c3", "c4"];
    const result = computeRadialLayout(
      [node("root"), ...children.map(node)],
      children.map((id) => edge("root", id)),
      CONFIG,
    );
    const root = result.get("root")!;
    const angles = children
      .map((id) => {
        const point = result.get(id)!;
        expect(distance(root, point)).toBeCloseTo(300, 6);
        return Math.atan2(point.cy - root.cy, point.cx - root.cx);
      })
      .sort((a, b) => a - b);
    for (let index = 1; index < angles.length; index++) {
      expect(angles[index] - angles[index - 1]).toBeCloseTo(Math.PI / 2, 6);
    }
  });

  it("最长路径分层：跨层边使目标节点比中间层更靠外", () => {
    // 意图：a→b→c 且 a→c，最长路径分层使 a 为第 0 层、b 为第 1 层、c 为第 2 层；
    // a 在原点，b 在正东（a 的锚点 (300,0)），c 的理想位 = (b 的锚点 (600,0) + a 的锚点 (300,0))/2
    // = (450,0)；碰撞松弛后 b、c 沿 x 轴微推。断言：三者 cy 相等（共线）；
    // a.cx < b.cx < c.cx（跨层边 a→c 使 c 比 b 更靠外）。
    const result = computeRadialLayout(
      [node("a"), node("b"), node("c")],
      [edge("a", "b"), edge("b", "c"), edge("a", "c")],
      CONFIG,
    );
    const a = result.get("a")!;
    const b = result.get("b")!;
    const c = result.get("c")!;
    expect(a.cy).toBeCloseTo(b.cy, 6);
    expect(b.cy).toBeCloseTo(c.cy, 6);
    expect(a.cx).toBeLessThan(b.cx);
    expect(b.cx).toBeLessThan(c.cx);
  });

  it("多分量：分量内部正常布局，分量之间不重叠", () => {
    // 意图：两条互不相连的链各自完成布局后按网格平铺，
    // 验证分量划分与平铺逻辑，跨分量节点间距不得小于 componentGap。
    const result = computeRadialLayout(
      [node("a1"), node("a2"), node("b1"), node("b2")],
      [edge("a1", "a2"), edge("b1", "b2")],
      CONFIG,
    );
    expect(distance(result.get("a1")!, result.get("a2")!)).toBeCloseTo(300, 6);
    expect(distance(result.get("b1")!, result.get("b2")!)).toBeCloseTo(300, 6);
    for (const fromA of ["a1", "a2"]) {
      for (const fromB of ["b1", "b2"]) {
        expect(
          distance(result.get(fromA)!, result.get(fromB)!),
        ).toBeGreaterThanOrEqual(CONFIG.componentGap);
      }
    }
  });

  it("孤立节点：排成规则网格且不与连通分量重叠", () => {
    // 意图：4 个孤立节点应排成 2×2 网格（间距为 isolatedCell），
    // 且位于连通分量区域右侧，验证孤立节点的独立网格排列。
    const isolated = ["i1", "i2", "i3", "i4"];
    const result = computeRadialLayout(
      [node("a"), node("b"), ...isolated.map(node)],
      [edge("a", "b")],
      CONFIG,
    );
    const points = isolated.map((id) => result.get(id)!);
    const uniqueCx = [...new Set(points.map((p) => Math.round(p.cx)))].sort(
      (a, b) => a - b,
    );
    const uniqueCy = [...new Set(points.map((p) => Math.round(p.cy)))].sort(
      (a, b) => a - b,
    );
    expect(uniqueCx.length).toBe(2);
    expect(uniqueCy.length).toBe(2);
    expect(uniqueCx[1] - uniqueCx[0]).toBe(CONFIG.isolatedCell);
    expect(uniqueCy[1] - uniqueCy[0]).toBe(CONFIG.isolatedCell);
    for (const point of points) {
      for (const componentNode of ["a", "b"]) {
        expect(
          distance(point, result.get(componentNode)!),
        ).toBeGreaterThanOrEqual(CONFIG.componentGap);
      }
    }
  });

  it("确定性：同输入多次调用、输入乱序，输出完全一致", () => {
    // 意图：算法必须完全确定（用户明确拒绝力导向的随机风格），
    // 所有排序平局按 id 决胜，输入顺序不影响输出。
    const buildInput = () => ({
      nodes: [
        node("root"),
        node("c1"),
        node("c2"),
        node("c3"),
        node("g1"),
        node("iso"),
        node("x1"),
        node("x2"),
      ],
      edges: [
        edge("root", "c1"),
        edge("root", "c2"),
        edge("root", "c3"),
        edge("c1", "g1"),
        edge("x1", "x2"),
      ],
    });
    const first = computeRadialLayout(buildInput().nodes, buildInput().edges, CONFIG);
    const secondInput = buildInput();
    const second = computeRadialLayout(secondInput.nodes, secondInput.edges, CONFIG);
    const reversed = computeRadialLayout(
      [...buildInput().nodes].reverse(),
      buildInput().edges,
      CONFIG,
    );
    expect(Object.fromEntries(first)).toEqual(Object.fromEntries(second));
    expect(Object.fromEntries(first)).toEqual(Object.fromEntries(reversed));
  });

  it("环防御：成环节点降级为孤立节点，正常分量不受影响", () => {
    // 意图：后端保证 DAG，但算法需对环保持防御——拓扑排序无法覆盖的
    // 环节点应获得孤立网格位置而非导致死循环或缺失输出。
    const result = computeRadialLayout(
      [node("a"), node("b"), node("d"), node("e")],
      [edge("a", "b"), edge("b", "a"), edge("d", "e")],
      CONFIG,
    );
    expect(result.size).toBe(4);
    expect(distance(result.get("d")!, result.get("e")!)).toBeCloseTo(300, 6);
    // 环节点按孤立网格排列：2 个孤立节点同行，间距为 isolatedCell。
    const a = result.get("a")!;
    const b = result.get("b")!;
    expect(Math.abs(a.cx - b.cx)).toBeCloseTo(CONFIG.isolatedCell, 6);
    expect(a.cy).toBeCloseTo(b.cy, 6);
  });

  it("均布容量：无端口子代过密时自动扩大锚距", () => {
    // 意图：root + 30 个无端口子代，理想锚距按弦长公式 max(ringSpacing, span/(2·sin(π/n))) 计算，
    // 其中 span = 200、n = 30，理想锚距 ≈ 955.64 > ringSpacing；碰撞松弛使部分节点被推向外侧，
    // 故使用范围断言：每个子代距离 ≥ 理想值 − 1（不小于理想锚距），
    // 且极差 < 30px（松弛后仍近似等距）；
    // 任意两节点不重叠（|dx|≥199 或 |dy|≥119）。
    const children = Array.from({ length: 30 }, (_, index) => `c${index}`);
    const result = computeRadialLayout(
      [node("root"), ...children.map(node)],
      children.map((id) => edge("root", id)),
      CONFIG,
    );
    const root = result.get("root")!;
    const idealDistance = Math.max(300, 200 / (2 * Math.sin(Math.PI / 30)));
    const distances = children.map((id) => distance(root, result.get(id)!));
    for (const d of distances) {
      expect(d).toBeGreaterThanOrEqual(idealDistance - 1);
    }
    const maxDist = Math.max(...distances);
    const minDist = Math.min(...distances);
    expect(maxDist - minDist).toBeLessThan(30);
    for (let i = 0; i < children.length; i++) {
      for (let j = i + 1; j < children.length; j++) {
        const p1 = result.get(children[i])!;
        const p2 = result.get(children[j])!;
        const dx = Math.abs(p1.cx - p2.cx);
        const dy = Math.abs(p1.cy - p2.cy);
        expect(dx >= 199 || dy >= 119).toBe(true);
      }
    }
  });

  it("均布平局按 id 排序", () => {
    // 意图：root + id-a、id-b 两个无端口子代，锚距 = max(300, span/(2·sin(π/2))) = 300；
    // 均布角度为 2πi/n，id 序 i=0 对应 0°（正东）、i=1 对应 π（正西）。
    // 断言 id-a 在正东（cx−root.cx≈300，cy≈root.cy），id-b 在正西（cx−root.cx≈−300）。
    const result = computeRadialLayout(
      [node("root"), node("id-a"), node("id-b")],
      [edge("root", "id-a"), edge("root", "id-b")],
      CONFIG,
    );
    const root = result.get("root")!;
    const a = result.get("id-a")!;
    const b = result.get("id-b")!;
    expect(a.cx - root.cx).toBeCloseTo(300, 6);
    expect(a.cy - root.cy).toBeCloseTo(0, 6);
    expect(b.cx - root.cx).toBeCloseTo(-300, 6);
    expect(b.cy - root.cy).toBeCloseTo(0, 6);
  });

  it("无效边被忽略：端点缺失的边不影响布局", () => {
    // 意图：引用不存在节点的边（如数据不同步的脏数据）应被忽略，
    // 两个节点都按孤立节点处理而非报错。
    const result = computeRadialLayout(
      [node("x"), node("y")],
      [edge("x", "ghost")],
      CONFIG,
    );
    expect(result.size).toBe(2);
    const x = result.get("x")!;
    const y = result.get("y")!;
    expect(Math.abs(x.cx - y.cx)).toBeCloseTo(CONFIG.isolatedCell, 6);
    expect(x.cy).toBeCloseTo(y.cy, 6);
  });

  it("多端口父节点：理想位为各入边锚点的均值", () => {
    // 意图：a1、a2、a3 为根节点，水平成行居中于原点（a1(−200,0)、a2(0,0)、a3(200,0)）；
    // portedEdge(a1,b,"bottom") 与 portedEdge(a2,b,"bottom") 的锚点分别为
    // a1+(0,300) 与 a2+(0,300)，b 的理想位 = 均值 (−100,300)。
    // 断言：distance(b,a1) ≈ distance(b,a2)（4 位）；b.cy > a1.cy（南侧）；
    // b.cx ≈ (a1.cx+a2.cx)/2（6 位）；distance(b,a3) > distance(b,a1)。
    const result = computeRadialLayout(
      [node("a1"), node("a2"), node("a3"), node("b")],
      [portedEdge("a1", "b", "bottom"), portedEdge("a2", "b", "bottom")],
      CONFIG,
    );
    const a1 = result.get("a1")!;
    const a2 = result.get("a2")!;
    const a3 = result.get("a3")!;
    const b = result.get("b")!;
    expect(distance(b, a1)).toBeCloseTo(distance(b, a2), 4);
    expect(b.cy).toBeGreaterThan(a1.cy);
    expect(b.cx).toBeCloseTo((a1.cx + a2.cx) / 2, 6);
    expect(distance(b, a3)).toBeGreaterThan(distance(b, a1));
  });

  it("圆心父 + 双端口方位：端口期望方向决定子代位置", () => {
    // 意图：唯一根 a 在圆心，边 a→b 带 sourcePort=right(0)、targetPort=left(π+π≡0)，
    // 两端口期望方向均为正东，子代 b 应落在 a 的正东方向，间距为 ringSpacing。
    const result = computeRadialLayout(
      [node("a"), node("b")],
      [portedEdge("a", "b", "right", "left")],
      CONFIG,
    );
    const a = result.get("a")!;
    const b = result.get("b")!;
    expect(b.cy).toBeCloseTo(a.cy, 6);
    expect(b.cx - a.cx).toBeCloseTo(300, 6);
  });

  it("仅 sourcePort：子代沿端口期望方向展开", () => {
    // 意图：唯一根 a 在圆心，边 a→b 仅带 sourcePort=bottom(π/2)，
    // 子代 b 应落在 a 的正南方向，间距为 ringSpacing。
    const result = computeRadialLayout(
      [node("a"), node("b")],
      [portedEdge("a", "b", "bottom")],
      CONFIG,
    );
    const a = result.get("a")!;
    const b = result.get("b")!;
    expect(b.cx).toBeCloseTo(a.cx, 6);
    expect(b.cy - a.cy).toBeCloseTo(300, 6);
  });

  it("仅 targetPort（反向语义）：进入方向取反后决定子代方位", () => {
    // 意图：唯一根 a 在圆心，边 a→b 仅带 targetPort=top，top 的反向 = bottom(π/2)，
    // 子代 b 应落在 a 正南，与仅 sourcePort=bottom 的行为一致。
    const result = computeRadialLayout(
      [node("a"), node("b")],
      [portedEdge("a", "b", undefined, "top")],
      CONFIG,
    );
    const a = result.get("a")!;
    const b = result.get("b")!;
    expect(b.cx).toBeCloseTo(a.cx, 6);
    expect(b.cy - a.cy).toBeCloseTo(300, 6);
  });

  it("非根父节点的端口方向精确锚定", () => {
    // 意图：r 为根节点，edge(r,m) 无端口（m 在正东 300）；
    // portedEdge(m,c,"bottom") 的锚点 = m + (0,300)，新语义为端口方向精确锚定
    //（非角度偏置），c 应落在 m 正南 300 处。断言：c.cx ≈ m.cx（6 位）；
    // c.cy − m.cy ≈ 300（6 位）。
    const result = computeRadialLayout(
      [node("r"), node("m"), node("c")],
      [edge("r", "m"), portedEdge("m", "c", "bottom")],
      CONFIG,
    );
    const m = result.get("m")!;
    const c = result.get("c")!;
    expect(c.cx - m.cx).toBeCloseTo(0, 6);
    expect(c.cy - m.cy).toBeCloseTo(300, 6);
  });

  it("同端口兄弟成列且组间分侧", () => {
    // 意图：r 根，edge(r,pa)、edge(r,pb) 无端口（pa 正东 (300,0)、pb 正西 (−300,0)）；
    // pa→a1,a2,a3 均 portedEdge(pa,…,"right")，理想位 = pa+(300,0)，
    // 同父同锚向单入边兄弟沿锚向垂直方向（竖直）等距排开：cy 偏移 −200, 0, +200；
    // pb→b1,b2,b3 均 portedEdge(pb,…,"left")，理想位 = pb−(300,0)。
    // 断言：a 组全部 cx ≈ pa.cx+300（6 位）；a 组 cy 均值 ≈ pa.cy（6 位，对称列）；
    // 组内最大两两距离 < 组间最小两两距离。
    const aGroup = ["a1", "a2", "a3"];
    const bGroup = ["b1", "b2", "b3"];
    const result = computeRadialLayout(
      [node("r"), node("pa"), node("pb"), ...aGroup.map(node), ...bGroup.map(node)],
      [
        edge("r", "pa"),
        edge("r", "pb"),
        ...aGroup.map((id) => portedEdge("pa", id, "right")),
        ...bGroup.map((id) => portedEdge("pb", id, "left")),
      ],
      CONFIG,
    );
    const pa = result.get("pa")!;
    for (const id of aGroup) {
      expect(result.get(id)!.cx - pa.cx).toBeCloseTo(300, 6);
    }
    const aMeanY =
      aGroup.reduce((sum, id) => sum + result.get(id)!.cy, 0) / aGroup.length;
    expect(aMeanY - pa.cy).toBeCloseTo(0, 6);
    let maxIntra = 0;
    for (let i = 0; i < aGroup.length; i++) {
      for (let j = i + 1; j < aGroup.length; j++) {
        maxIntra = Math.max(
          maxIntra,
          distance(result.get(aGroup[i])!, result.get(aGroup[j])!),
        );
      }
    }
    let minInter = Infinity;
    for (const aId of aGroup) {
      for (const bId of bGroup) {
        minInter = Math.min(
          minInter,
          distance(result.get(aId)!, result.get(bId)!),
        );
      }
    }
    expect(maxIntra).toBeLessThan(minInter);
  });

  it("宇宙风格右向逐列树", () => {
    // 意图：root→c1,c2,c3 均 portedEdge(root,…,"right","left")，
    // 理想位 = root+(300,0)，同父同锚向单入边兄弟沿锚向垂直方向（竖直）等距排开：
    // c1(300,−200)、c2(300,0)、c3(300,+200)；c1→g1 同样端口，g1 在 c1 正东 300。
    // 断言：c2 在 root 正东 300（cx−root.cx≈300、cy≈root.cy）；
    // c1.cy−root.cy ≈ −(c3.cy−root.cy)（列对称）；g1.cx > c1.cx（继续向东）；
    // 所有子代 cx > root.cx。
    const children = ["c1", "c2", "c3"];
    const result = computeRadialLayout(
      [node("root"), ...children.map(node), node("g1")],
      [
        ...children.map((id) => portedEdge("root", id, "right", "left")),
        portedEdge("c1", "g1", "right", "left"),
      ],
      CONFIG,
    );
    const root = result.get("root")!;
    const c1 = result.get("c1")!;
    const c2 = result.get("c2")!;
    const c3 = result.get("c3")!;
    const g1 = result.get("g1")!;
    expect(c2.cx - root.cx).toBeCloseTo(300, 6);
    expect(c2.cy - root.cy).toBeCloseTo(0, 6);
    expect(c1.cy - root.cy).toBeCloseTo(-(c3.cy - root.cy), 6);
    expect(g1.cx).toBeGreaterThan(c1.cx);
    for (const id of children) {
      expect(result.get(id)!.cx).toBeGreaterThan(root.cx);
    }
  });

  it("同端口 5 子成列且中心对称", () => {
    // 意图：root→k1..k5 均 portedEdge(root,…,"right")，理想位 = root+(300,0)，
    // 同父同锚向单入边兄弟沿锚向垂直方向（竖直）等距排开：
    // 第 i 个偏移 (i−(5−1)/2)×200，即 −400,−200,0,+200,+400。
    // 断言：5 者 cx 均 ≈ root.cx+300（6 位）；cy 按 id 序相对 root.cy 的偏移依次为
    // −400,−200,0,+200,+400（6 位）。
    const kids = ["k1", "k2", "k3", "k4", "k5"];
    const result = computeRadialLayout(
      [node("root"), ...kids.map(node)],
      kids.map((id) => portedEdge("root", id, "right")),
      CONFIG,
    );
    const root = result.get("root")!;
    const expectedOffsets = [-400, -200, 0, 200, 400];
    kids.forEach((id, index) => {
      const point = result.get(id)!;
      expect(point.cx - root.cx).toBeCloseTo(300, 6);
      expect(point.cy - root.cy).toBeCloseTo(expectedOffsets[index], 6);
    });
  });

  it("无端口 8 子成正八边形", () => {
    // 意图：root + 8 个无端口子代，锚距 = max(300, 200/(2·sin(π/8))≈261.2) = 300；
    // 均布角度 2πi/8，相邻角差 π/4。断言：每个子代到 root 距离 ≈ 300（6 位）；
    // 相邻角差均 ≈ π/4（角度排序后逐对相减）。
    const kids = Array.from({ length: 8 }, (_, i) => `c${i}`);
    const result = computeRadialLayout(
      [node("root"), ...kids.map(node)],
      kids.map((id) => edge("root", id)),
      CONFIG,
    );
    const root = result.get("root")!;
    const angles = kids
      .map((id) => {
        const point = result.get(id)!;
        expect(distance(root, point)).toBeCloseTo(300, 6);
        return Math.atan2(point.cy - root.cy, point.cx - root.cx);
      })
      .sort((a, b) => a - b);
    for (let index = 1; index < angles.length; index++) {
      expect(angles[index] - angles[index - 1]).toBeCloseTo(Math.PI / 4, 6);
    }
  });

  it("无端口 20 子自动扩距成正多边形", () => {
    // 意图：root + 20 个无端口子代，理想锚距 = 200/(2·sin(π/20)) ≈ 639.24；
    // 碰撞松弛使部分节点被推向外侧，故使用范围断言：每个子代距离 ≥ 理想值 − 1，
    // 且极差 < 30px（松弛后仍近似等距）；任意两节点不重叠（|dx|≥199 或 |dy|≥119）。
    const kids = Array.from({ length: 20 }, (_, i) => `c${i}`);
    const result = computeRadialLayout(
      [node("root"), ...kids.map(node)],
      kids.map((id) => edge("root", id)),
      CONFIG,
    );
    const root = result.get("root")!;
    const idealDistance = 200 / (2 * Math.sin(Math.PI / 20));
    const distances = kids.map((id) => distance(root, result.get(id)!));
    for (const d of distances) {
      expect(d).toBeGreaterThanOrEqual(idealDistance - 1);
    }
    const maxDist = Math.max(...distances);
    const minDist = Math.min(...distances);
    expect(maxDist - minDist).toBeLessThan(30);
    for (let i = 0; i < kids.length; i++) {
      for (let j = i + 1; j < kids.length; j++) {
        const p1 = result.get(kids[i])!;
        const p2 = result.get(kids[j])!;
        const dx = Math.abs(p1.cx - p2.cx);
        const dy = Math.abs(p1.cy - p2.cy);
        expect(dx >= 199 || dy >= 119).toBe(true);
      }
    }
  });

  it("带端口输入的确定性：多根多簇孤立节点图在乱序输入下输出一致", () => {
    // 意图：算法必须完全确定性，带端口的边参与计算时同样保证
    // 输入顺序不影响输出（所有排序平局按 id 决胜）。
    const buildInput = () => ({
      nodes: [
        node("root"),
        node("c1"),
        node("c2"),
        node("c3"),
        node("g1"),
        node("iso"),
        node("x1"),
        node("x2"),
      ],
      edges: [
        portedEdge("root", "c1", "right", "left"),
        portedEdge("root", "c2", "right", "left"),
        portedEdge("root", "c3", "right", "left"),
        portedEdge("c1", "g1", "right", "left"),
        edge("x1", "x2"),
      ],
    });
    const first = computeRadialLayout(buildInput().nodes, buildInput().edges, CONFIG);
    const secondInput = buildInput();
    const second = computeRadialLayout(secondInput.nodes, secondInput.edges, CONFIG);
    const reversed = computeRadialLayout(
      [...buildInput().nodes].reverse(),
      buildInput().edges,
      CONFIG,
    );
    expect(Object.fromEntries(first)).toEqual(Object.fromEntries(second));
    expect(Object.fromEntries(first)).toEqual(Object.fromEntries(reversed));
  });

  it("防御：带端口的无效边被忽略", () => {
    // 意图：引用不存在节点的边（即使带有端口方向）应被忽略，
    // 两个节点都按孤立节点网格排列而非报错。
    const result = computeRadialLayout(
      [node("x"), node("y")],
      [portedEdge("x", "ghost", "right", "left")],
      CONFIG,
    );
    expect(result.size).toBe(2);
    const x = result.get("x")!;
    const y = result.get("y")!;
    expect(Math.abs(x.cx - y.cx)).toBeCloseTo(CONFIG.isolatedCell, 6);
    expect(x.cy).toBeCloseTo(y.cy, 6);
  });

  it("菱形带捷径：多父节点保持各父的端口轴向间距", () => {
    // 意图：a 为唯一根；b1、b2 同父同锚向成竖列（a 正东 300，cy 偏移 −100/+100）；
    // c1 在 b1 正东 300；d1 的两锚点为 c1+(300,0) 与 b2+(300,0)，均值会拽到 c1 西南侧，
    // 但第四遍约束强制 d1.cx = c1.cx + 300。断言：d1.cx − c1.cx ≈ 300（6 位）；
    // d1.cy − a.cy ≈ 0（6 位，均值保持）；d1.cx > c1.cx；全部 5 节点两两不重叠。
    const result = computeRadialLayout(
      [node("a"), node("b1"), node("b2"), node("c1"), node("d1")],
      [
        portedEdge("a", "b1", "right", "left"),
        portedEdge("a", "b2", "right", "left"),
        portedEdge("b1", "c1", "right", "left"),
        portedEdge("b2", "d1", "right", "left"),
        portedEdge("c1", "d1", "right", "left"),
      ],
      CONFIG,
    );
    const a = result.get("a")!;
    const c1 = result.get("c1")!;
    const d1 = result.get("d1")!;
    // 端口轴向间距约束：d1 在 c1 正东 300（均值侵蚀被约束抵消）
    expect(d1.cx - c1.cx).toBeCloseTo(300, 6);
    // 垂直方向保持两锚点均值（a.cy 为 0 参考线，d1 位于 c1 与 b2 之间均值点）
    expect(d1.cy - a.cy).toBeCloseTo(0, 6);
    // d1 相对 c1 严格偏东
    expect(d1.cx).toBeGreaterThan(c1.cx);
    // 全部 5 节点两两不重叠（|dx|≥199 或 |dy|≥119，允许 1px 容差）
    const allIds = ["a", "b1", "b2", "c1", "d1"];
    for (let i = 0; i < allIds.length; i++) {
      for (let j = i + 1; j < allIds.length; j++) {
        const p1 = result.get(allIds[i])!;
        const p2 = result.get(allIds[j])!;
        const dx = Math.abs(p1.cx - p2.cx);
        const dy = Math.abs(p1.cy - p2.cy);
        expect(dx >= 199 || dy >= 119).toBe(true);
      }
    }
  });

  it("菱形带捷径（垂直方向）：bottom 端口保持垂直轴向间距", () => {
    // 意图：结构同上但所有边改为 bottom→top；b1、b2 成横行（a 正南 300，cx 偏移 −100/+100）；
    // c1 在 b1 正南 300；d1 的两锚点为 c1+(0,300) 与 b2+(0,300)，均值会拽到 c1 西北侧，
    // 但第四遍约束强制 d1.cy = c1.cy + 300。断言：d1.cy − c1.cy ≈ 300（6 位）；
    // d1.cx − a.cx ≈ 0（6 位，均值保持）；d1.cy > c1.cy；全部节点两两不重叠。
    const result = computeRadialLayout(
      [node("a"), node("b1"), node("b2"), node("c1"), node("d1")],
      [
        portedEdge("a", "b1", "bottom", "top"),
        portedEdge("a", "b2", "bottom", "top"),
        portedEdge("b1", "c1", "bottom", "top"),
        portedEdge("b2", "d1", "bottom", "top"),
        portedEdge("c1", "d1", "bottom", "top"),
      ],
      CONFIG,
    );
    const a = result.get("a")!;
    const c1 = result.get("c1")!;
    const d1 = result.get("d1")!;
    // 端口轴向间距约束：d1 在 c1 正南 300（均值侵蚀被约束抵消）
    expect(d1.cy - c1.cy).toBeCloseTo(300, 6);
    // 水平方向保持两锚点均值（a.cx 为 0 参考线，d1 位于 c1 与 b2 之间均值点）
    expect(d1.cx - a.cx).toBeCloseTo(0, 6);
    // d1 相对 c1 严格偏南
    expect(d1.cy).toBeGreaterThan(c1.cy);
    // 全部节点两两不重叠（|dx|≥199 或 |dy|≥119，允许 1px 容差）
    const allIds = ["a", "b1", "b2", "c1", "d1"];
    for (let i = 0; i < allIds.length; i++) {
      for (let j = i + 1; j < allIds.length; j++) {
        const p1 = result.get(allIds[i])!;
        const p2 = result.get(allIds[j])!;
        const dx = Math.abs(p1.cx - p2.cx);
        const dy = Math.abs(p1.cy - p2.cy);
        expect(dx >= 199 || dy >= 119).toBe(true);
      }
    }
  });
});
