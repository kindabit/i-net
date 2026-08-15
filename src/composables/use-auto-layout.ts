/**
 * 自动布局组合式函数。
 *
 * 在 `视图 → use-auto-layout → radial-layout` 调用链中，本模块作为视图与布局算法之间的适配层：
 * 从视图采集节点/边数据 → 调用 radial-layout 计算目标坐标 → 播放 rAF 动画移动节点 → 回调视图完成持久化。
 */
import { ref, type Ref } from "vue";
import {
  computeRadialLayout,
  DEFAULT_RADIAL_LAYOUT_CONFIG,
} from "@/utils/radial-layout";
import type {
  RadialLayoutEdge,
  RadialLayoutNode,
  RadialPortDirection,
} from "@/utils/radial-layout";
import { snackbarErrorCode } from "@/composables/use-snackbar";
import type { MoveNodeVO } from "@/api-types";

/** 自动布局所需的单个节点信息（对应 vue-flow GraphNode 的子集）。 */
interface AutoLayoutNode {
  id: string;
  position: { x: number; y: number };
  dimensions?: { width: number; height: number };
}

/** 自动布局所需的单条边信息。 */
interface AutoLayoutEdge {
  source: string;
  target: string;
  /** 可选：源 handle id（vue-flow 边的 sourceHandle，可能为 null）。 */
  sourceHandle?: string | null;
  /** 可选：目标 handle id（vue-flow 边的 targetHandle，可能为 null）。 */
  targetHandle?: string | null;
}

/** 自动布局所需的视图适配参数。 */
export interface AutoLayoutOptions {
  /** 获取当前 vue-flow 节点列表（GraphNode，含 position 与可选 dimensions）。 */
  getNodes: () => AutoLayoutNode[];
  /** 获取当前 vue-flow 边列表（只需 source/target）。 */
  getEdges: () => AutoLayoutEdge[];
  /** 持久化回调：动画结束后以最终坐标调用（对应后端批量移动 API）。 */
  persist: (items: MoveNodeVO[]) => Promise<void>;
  /**
   * 可选：动画结束、persist 之前的同步回调。
   * 用于把布局结果同步回父组件的 nodes 数组（整体替换每个被影响节点的 position 对象），
   * 避免后续节点增删时被 vue-flow 的 parseNode 用 props 中的旧 position 覆盖 store。
   * 在 persist 之前调用，保证持久化完成时 props 与 store 已经同源。
   * 失败的回滚语义与 persist 保持一致：不回滚。
   */
  onNodesMoved?: (items: MoveNodeVO[]) => void;
  /** vue-flow 的 snapGrid 配置，布局结果对齐到该网格。 */
  snapGrid: [number, number];
  /** 读取不到 node.dimensions 时的兜底尺寸。 */
  fallbackSize: { width: number; height: number };
}

/**
 * easeInOutCubic 缓动函数。
 * @param t 原始进度（0~1）
 * @returns 缓动后的进度（0~1）
 */
function easeInOutCubic(t: number): number {
  return t < 0.5 ? 4 * t * t * t : 1 - Math.pow(-2 * t + 2, 3) / 2;
}

/**
 * requestAnimationFrame 动画封装：在指定时长内每帧调用 onFrame(progress)，动画完成时 resolve。
 * @param duration 动画时长（毫秒）
 * @param onFrame 每帧回调，参数为 0~1 的缓动进度
 * @returns Promise，动画完成时 resolve
 */
function animate(
  duration: number,
  onFrame: (progress: number) => void,
): Promise<void> {
  return new Promise((resolve) => {
    const startTime = performance.now();
    function frame(now: number) {
      const elapsed = now - startTime;
      const rawProgress = Math.min(elapsed / duration, 1);
      const easedProgress = easeInOutCubic(rawProgress);
      onFrame(easedProgress);
      if (rawProgress < 1) {
        requestAnimationFrame(frame);
      } else {
        resolve();
      }
    }
    requestAnimationFrame(frame);
  });
}

/**
 * 获取节点的实际尺寸（优先 dimensions，为 0 或 undefined 时用 fallbackSize）。
 * @param node vue-flow 节点
 * @param fallbackSize 兜底尺寸
 * @returns 节点的宽高
 */
function getNodeSize(
  node: AutoLayoutNode,
  fallbackSize: { width: number; height: number },
): { width: number; height: number } {
  const width =
    node.dimensions?.width && node.dimensions.width > 0
      ? node.dimensions.width
      : fallbackSize.width;
  const height =
    node.dimensions?.height && node.dimensions.height > 0
      ? node.dimensions.height
      : fallbackSize.height;
  return { width, height };
}

/**
 * 将 vue-flow 边的 handle id 映射为布局算法的端口方向。
 *
 * 普通画布 DataNode 的 handle id 即四方向（top/bottom/left/right），直接透传；
 * 画布宇宙 CanvasNode 的 handle id 为 source-right/target-left，映射为对应方向；
 * 空串、null、undefined 及任何未识别的 id 一律返回 undefined（算法按无端口处理）。
 * @param handle vue-flow 边的 handle id（可能为 null/undefined）
 * @returns 端口方向；无法识别时返回 undefined
 */
function toPortDirection(handle: string | null | undefined): RadialPortDirection | undefined {
  if (handle === "top" || handle === "bottom" || handle === "left" || handle === "right") {
    return handle;
  }
  if (handle === "source-right") return "right";
  if (handle === "target-left") return "left";
  return undefined;
}

/**
 * 创建自动布局组合式函数。
 *
 * 供两个画布视图共用：封装了数据采集、布局计算、rAF 动画、持久化的完整流程，
 * 并提供防重入与无变化跳过机制。
 * @param options 视图适配参数（获取节点/边、持久化回调、snapGrid、fallbackSize）
 * @returns isLayouting 布局进行中标志；applyAutoLayout 触发自动布局的异步函数
 */
export function useAutoLayout(options: AutoLayoutOptions): {
  isLayouting: Ref<boolean>;
  applyAutoLayout: () => Promise<void>;
} {
  const isLayouting = ref(false);

  /**
    * 执行自动布局：采集数据 → 计算布局 → rAF 动画移动节点 → 持久化。
   *
   * 流程：
   * 1. 防重入：布局进行中时直接返回。
   * 2. 收集节点尺寸与边，调用 computeRadialLayout 计算中心坐标。
   * 3. 换算为左上角坐标并对齐到 snap 网格。
   * 4. 若所有节点新旧坐标相同则跳过动画与持久化。
   * 5. 300ms rAF 动画（easeInOutCubic）逐帧更新节点 position。
   * 6. 动画结束后调用 persist 批量持久化；失败时通过 snackbarErrorCode 提示（不回滚）。
   * @returns 无返回值
   */
  async function applyAutoLayout(): Promise<void> {
    if (isLayouting.value) return;
    isLayouting.value = true;

    try {
      const vfNodes = options.getNodes();
      const vfEdges = options.getEdges();

      // 收集节点尺寸（优先 dimensions，否则 fallback）
      const layoutNodes: RadialLayoutNode[] = vfNodes.map((node) => {
        const { width, height } = getNodeSize(node, options.fallbackSize);
        return { id: node.id, width, height };
      });

      const layoutEdges: RadialLayoutEdge[] = vfEdges.map((edge) => ({
        source: edge.source,
        target: edge.target,
        sourcePort: toPortDirection(edge.sourceHandle),
        targetPort: toPortDirection(edge.targetHandle),
      }));

      // 计算布局（中心坐标）
      const centerPoints = computeRadialLayout(
        layoutNodes,
        layoutEdges,
        DEFAULT_RADIAL_LAYOUT_CONFIG,
      );

      // 换算为左上角坐标并对齐到 snap 网格
      const targetPositions = new Map<string, { x: number; y: number }>();
      for (const node of vfNodes) {
        const point = centerPoints.get(node.id);
        if (!point) continue;
        const { width, height } = getNodeSize(node, options.fallbackSize);
        const x =
          Math.round((point.cx - width / 2) / options.snapGrid[0]) *
          options.snapGrid[0];
        const y =
          Math.round((point.cy - height / 2) / options.snapGrid[1]) *
          options.snapGrid[1];
        targetPositions.set(node.id, { x, y });
      }

      // 检查是否有变化：所有节点新旧坐标相同则跳过
      let hasChange = false;
      for (const node of vfNodes) {
        const target = targetPositions.get(node.id);
        if (!target) continue;
        if (node.position.x !== target.x || node.position.y !== target.y) {
          hasChange = true;
          break;
        }
      }
      if (!hasChange) return;

      // 记录旧位置用于插值
      const oldPositions = new Map<string, { x: number; y: number }>();
      for (const node of vfNodes) {
        oldPositions.set(node.id, { x: node.position.x, y: node.position.y });
      }

      // rAF 动画：300ms，easeInOutCubic 缓动
      await animate(300, (progress) => {
        for (const node of vfNodes) {
          const old = oldPositions.get(node.id);
          const target = targetPositions.get(node.id);
          if (!old || !target) continue;
          node.position.x = old.x + (target.x - old.x) * progress;
          node.position.y = old.y + (target.y - old.y) * progress;
        }
      });

      // 确保最终位置精确对齐目标值
      for (const node of vfNodes) {
        const target = targetPositions.get(node.id);
        if (!target) continue;
        node.position.x = target.x;
        node.position.y = target.y;
      }

      // 持久化：构造批量移动条目
      const items: MoveNodeVO[] = [];
      for (const [id, pos] of targetPositions) {
        items.push({ id, x: pos.x, y: pos.y });
      }
      // 先同步父组件 nodes，再调 persist：保证持久化发起时 props 与 store 同源，
      // 避免 persist 异步完成期间 props 仍持有旧坐标导致被 parseNode 回滚。
      options.onNodesMoved?.(items);
      await options.persist(items);
    } catch (e) {
      snackbarErrorCode(e);
    } finally {
      isLayouting.value = false;
    }
  }

  return { isLayouting, applyAutoLayout };
}
