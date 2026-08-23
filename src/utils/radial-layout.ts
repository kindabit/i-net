/**
 * 罗盘锚定分层布局算法（纯函数模块，不依赖 Vue / vue-flow）。
 *
 * 本文件是编排门面：管线各 pass 已拆分至 ./layout/ 下的独立模块——
 *   sanitize        Pass 1 输入清理（边过滤去重 + 邻接表）
 *   layering        Pass 2 最长路径分层（Kahn 拓扑）
 *   components      Pass 3 连通分量划分（并查集）
 *   orientation     Pass 4 边方向规范化（汇聚星型反转，保证方向翻转对称）
 *   component-layout Pass 5 分量布局（根行 → 理想位 → 兄弟展开 → 端口约束）
 *   collision       Pass 6 碰撞松弛（确定性）
 *   compose         Pass 7 全局合成（分量平铺 + 孤立网格 + 归一）
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
 * - 边方向仅经 orientation pass 规范后使用：汇聚星型分量（多纯源、
 *   方位发散）整体反转，使视觉枢纽成为分层根；其余结构保持原方向。
 *   判据变更需同步补充翻转对称性测试（radial-layout.test.ts）。
 * - 已知边界（如需进一步优化可从这些点入手）：
 *   ① 边跨节点仅在布局侧缓解，完全消除需改 CustomEdge.vue 为正交路由；
 *   ② 间距约束先于碰撞松弛执行，拥挤场景松弛可能轻微侵蚀间距，
 *      严格保证需约束-松弛交替求解；
 *   ③ 病态输入（同一节点左右/上下矛盾端口）按 left/top 覆盖
 *      right/bottom 取舍；④ 无端口边的方向推断（流向/均布）仅是兜底，
 *      真实数据（CanvasView/CanvasUniverseView）每条边都带端口。
 */

import { layoutComponent, type ComponentLayout } from "./layout/component-layout";
import { composeLayout } from "./layout/compose";
import { groupComponents } from "./layout/components";
import { assignLayers } from "./layout/layering";
import { normalizeOrientation } from "./layout/orientation";
import { sanitizeGraph } from "./layout/sanitize";
import type {
  RadialLayoutConfig,
  RadialLayoutEdge,
  RadialLayoutNode,
  RadialLayoutPoint,
} from "./layout/types";

export {
  DEFAULT_RADIAL_LAYOUT_CONFIG,
  type RadialLayoutConfig,
  type RadialLayoutEdge,
  type RadialLayoutNode,
  type RadialLayoutPoint,
  type RadialPortDirection,
} from "./layout/types";

/**
 * 计算 DAG 的罗盘锚定分层布局。
 *
 * 流程：输入清理 → 最长路径分层（Kahn 拓扑）→ 并查集划分连通分量 →
 * 边方向规范化（汇聚星型分量反转）→（如发生反转）重建邻接并重新分层 →
 * 每个分量各自罗盘锚定布局 → 分量按大小降序（平局按最小 id）网格平铺 →
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
  if (nodes.length === 0) {
    return new Map();
  }
  const nodeIds = nodes.map((node) => node.id);

  // Pass 1：输入清理。
  const graph = sanitizeGraph(nodes, edges);

  // Pass 2：首次分层（供方向规范化做环检测与结构判定）。
  let layer = assignLayers(nodeIds, graph);

  // Pass 3：连通分量划分（按无向连通，方向规范化不影响划分结果）。
  const componentIds = groupComponents(nodes, graph.edges);

  // Pass 4：边方向规范化——汇聚星型分量反转，使视觉枢纽成为分层根。
  const orientedEdges = normalizeOrientation(graph, layer, componentIds);
  let effectiveGraph = graph;
  if (orientedEdges !== null) {
    effectiveGraph = sanitizeGraph(nodes, orientedEdges);
    layer = assignLayers(nodeIds, effectiveGraph);
  }

  // Pass 5：每个分量内布局（成功分层的节点参与，不足 2 个按孤立处理）；
  // 未分层节点（环相关，防御性场景）降级为孤立节点。
  const components: ComponentLayout[] = [];
  const isolatedIds: string[] = [];
  for (const ids of componentIds.values()) {
    const acyclic = ids.filter((id) => layer.has(id));
    isolatedIds.push(...ids.filter((id) => !layer.has(id)));
    if (acyclic.length >= 2) {
      components.push(
        layoutComponent(acyclic, layer, effectiveGraph.incoming, graph.nodeById, config),
      );
    } else {
      isolatedIds.push(...acyclic);
    }
  }

  // Pass 6（分量内碰撞松弛已在 layoutComponent 末尾执行）+
  // Pass 7：全局合成。
  return composeLayout(components, isolatedIds, graph.nodeById, config);
}
