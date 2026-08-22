/**
 * 节点移动和迁移系统，以一种统一的方式管理节点在画布内的移动和跨画布的迁移行为
 *
 * 本系统由状态机驱动，该文件定义本系统的状态机
 *
 * 基本流程：
 * 1. 用户通过定义良好的“入口”操作启动本系统
 * 2. 用户执行一系列对系统状态进行变更的操作
 * 3. 变更操作产生副作用（各组件通过 watch 状态机状态来自行传播副作用）
 * 4. 用户通过定义良好的“出口”操作关闭本系统
 */
import { Ref, ref } from "vue";
import { Node as VFNode, Edge as VFEdge } from "@vue-flow/core";
import { DataNodeData } from "@/vf-convert";

/**
 * 迁移目标的类型以及相关数据
 */
export type RelocatingTarget =
  { type: 'canvas-node', nodeId: string, canvasId: string } |
  { type: 'shadow-node', nodeId: string, shadowId: string } |
  { type: 'breadcrumb-segment', canvasId: string, canvasName: string }

/**
 * 节点集的迁移合法性
 * legal - 合法
 * has-shadow - 有影子节点
 * has-canvas - 有画布节点
 * has-external - 节点集与外部之间存在边
 */
export type RelocatingLegality = 'legal' | 'has-shadow' | 'has-canvas' | 'has-external';

/**
 * 状态机的模式
 * move - 普通移动，在画布内移动
 * relocate - 节点迁移，在画布之间迁移
 */
export type Mode = 'move' | 'relocate';

/**
 * 拖拽结束事件的监听器的类型
 */
export type OnDragStopEffectFn = (mode: Mode, draggedNodes: VFNode[], draggedNodesRelocatingLegality: RelocatingLegality, pointerPosition: { x: number, y: number }, relocatingTarget: RelocatingTarget | null) => void;

function useNodeMoveAndRelocate() {

  /**
   * 状态机是否处于激活状态
   */
  let active = ref(false);

  /**
   * 当前的节点位置变更模式
   */
  let mode: Ref<Mode | null> = ref(null);

  /**
   * 正在被用户拖动的节点集，包含节点 id
   */
  let nodeSet: Ref<VFNode[]> = ref([]);

  /**
   * 用户正在拖动的节点集是否是合法的可迁移节点集
   * 只有当节点集中的每个节点都是合法的可迁移节点、并且节点集中的每个节点的边都只与节点集内部的节点相连时，这个节点集才是合法的可迁移节点集
   * 非画布节点，且非影子节点的节点是合法的可迁移节点
   */
  let nodeSetRelocatingLegality: Ref<RelocatingLegality | null> = ref(null);

  /**
   * 光标位置
   */
  let pointerPosition: Ref<{ x: number; y: number; } | null> = ref(null);

  /**
   * 迁移目标
   */
  let relocatingTarget: Ref<RelocatingTarget | null> = ref(null);

  /**
   * 查询指定位置上有哪些节点的函数
   */
  let queryNodeAtPosition: ((position: { x: number, y: number }) => VFNode[]) | null = null;

  /**
   * 查询指定位置上有哪个面包屑片段的函数
   */
  let queryBreadcrumbAtPosition: ((position: { x: number, y: number }) => { canvasId: string, canvasName: string} | null) | null = null;

  /**
   * 拖拽结束事件的监听器
   */
  const onDragStopEffects: OnDragStopEffectFn[] = [];

  // 开始 - 状态机的内部逻辑

  /**
   * 在执行入口操作之前校验状态机的状态，所有状态应该都为 null ，所有 query 函数应该都不为 null
   * @param entranceType 入口类型
   */
  function validateStatusBeforeEntranceOperation(entranceType: string) {
    if (mode.value !== null) {
      throw new Error(`entrance operation: ${entranceType}: status validation failed: mode !== null: mode = ` + mode.value);
    }
    if (nodeSet.value.length !== 0) {
      throw new Error(`entrance operation: ${entranceType}: status validation failed: nodeSet.length !== 0: nodeSet.length = ` + nodeSet.value.length);
    }
    if (nodeSetRelocatingLegality.value !== null) {
      throw new Error(`entrance operation: ${entranceType}: status validation failed: nodeSetRelocatingLegality !== null: nodeSetRelocatingLegality = ` + nodeSetRelocatingLegality.value);
    }
    if (pointerPosition.value !== null) {
      throw new Error(`entrance operation: ${entranceType}: status validation failed: pointerPosition.value !== null: pointerPosition.value = ` + JSON.stringify(pointerPosition.value));
    }
    if (relocatingTarget.value !== null) {
      throw new Error(`entrance operation: ${entranceType}: status validation failed: relocatingTarget.value !== null: relocatingTarget.value = ` + JSON.stringify(relocatingTarget.value));
    }
    if (queryNodeAtPosition === null) {
      throw new Error(`entrance operation: ${entranceType}: status validation failed: queryNodeAtPosition === null: queryNodeAtPosition not ready`);
    }
    if (queryBreadcrumbAtPosition === null) {
      throw new Error(`entrance operation: ${entranceType}: status validation failed: queryBreadcrumbAtPosition === null: queryBreadcrumbAtPosition not ready`);
    }
  }

  /**
   * 计算节点集的迁移合法性
   * @param draggedNodes 被拖动的节点
   * @param allEdges 所有边
   * @returns 节点集的迁移合法性
   */
  function calculateLegality(draggedNodes: VFNode[], allEdges: VFEdge[]): RelocatingLegality {
    let set = new Set();
    for (let node of draggedNodes) {
      if ((node.data as DataNodeData).canvasId) {
        return 'has-canvas';
      }
      if ((node.data as DataNodeData).shadowId) {
        return 'has-shadow';
      }
      set.add(node.id);
    }
    for (let edge of allEdges) {
      if ((set.has(edge.source) && !set.has(edge.target)) || (set.has(edge.target) && !set.has(edge.source))) {
        return 'has-external';
      }
    }
    return 'legal';
  }

  /**
   * 根据光标位置计算迁移目标
   * @param position 光标的当前位置
   * @returns 迁移目标
   */
  function calculateRelocatingTarget(position: { x: number, y: number }): RelocatingTarget | null {
    // 此函数被调用的时间必然在入口操作之后，因此 query 函数必然不可能为 null
    let nodes = queryNodeAtPosition!(position);
    for (let node of nodes) {
      let data = node.data as DataNodeData;
      if (data.shadowId) {
        return {
          type: 'shadow-node',
          nodeId: node.id,
          shadowId: data.shadowId,
        };
      }
      if (data.canvasId) {
        return {
          type: 'canvas-node',
          nodeId: node.id,
          canvasId: data.canvasId,
        };
      }
    }
    let breadcrumb = queryBreadcrumbAtPosition!(position);
    if (breadcrumb) {
      return {
        type: 'breadcrumb-segment',
        canvasId: breadcrumb.canvasId,
        canvasName: breadcrumb.canvasName,
      };
    }
    return null;
  }

  /**
   * 处理用户按下 alt 键的行为，该事件可通过 window 对象进行监听，无需依赖其它组件
   * @param event 键盘事件
   */
  function onAltKeyDown(event: KeyboardEvent) {
    if (event.key !== 'Alt') {
      return;
    }
    if (active.value) {
      // 用户在状态机已启动时按下 alt 键
      // 切换至迁移模式
      mode.value = 'relocate';
    }
    else {
      // 用户在状态机尚未启动时按下 alt 键，是“入口”操作
      // 校验状态是否正确
      validateStatusBeforeEntranceOperation('alt key down');
      // 激活状态机
      active.value = true;
      // 切换至迁移模式
      mode.value = 'relocate';
    }
  }

  /**
   * 按“alt 键已松开”的语义推进状态机：节点集为空时是“出口”操作，否则切换回移动模式。
   * 调用前提：状态机已激活且处于迁移模式。
   */
  function handleAltReleased() {
    if (nodeSet.value.length === 0) {
      // 用户在节点集为空时松开 alt 键，是“出口”操作
      initialize();
    }
    else {
      // 否则只是普通的状态迁移，将模式切换为移动模式
      mode.value = 'move';
    }
  }

  /**
   * 处理用户松开 alt 键的行为，该事件可通过 window 对象进行监听，无需依赖其它组件。
   * 浏览器不保证 keyup 与 keydown 严格配对（如 Alt+Tab 切入本窗口后松开 Alt、
   * 左右 Alt 先后按下再先后松开），对不配对的 keyup 静默忽略。
   * @param event 键盘事件
   */
  function onAltKeyUp(event: KeyboardEvent) {
    if (event.key !== 'Alt') {
      return;
    }
    // 不配对的 keyup：状态机未激活，或本次松开已被处理（模式已不在 relocate）
    if (!active.value || mode.value !== 'relocate') {
      return;
    }
    handleAltReleased();
  }

  /**
   * 处理窗口失焦的行为。失焦后无法收到 alt 的 keyup（事件发往其它窗口），
   * 为避免状态机滞留在迁移模式，失焦即视为 alt 键已松开。
   */
  function onWindowBlur() {
    if (!active.value || mode.value !== 'relocate') {
      return;
    }
    handleAltReleased();
  }

  /**
   * 状态机初始化，将所有状态重置为初始值（查询函数与事件监听保持不变）
   */
  function initialize() {
    active.value = false;
    mode.value = null;
    nodeSet.value = [];
    nodeSetRelocatingLegality.value = null;
    pointerPosition.value = null;
    relocatingTarget.value = null;
  }

  // 结束 - 状态机的内部逻辑

  /**
   * 设置查询指定位置上有哪些节点的函数
   * @param fn 查询指定位置上有哪些节点的函数
   */
  function setQueryNodeAtPosition(fn: (position: { x: number, y: number }) => VFNode[]) {
    queryNodeAtPosition = fn;
  }

  /**
   * 设置查询指定位置上有哪个面包屑片段的函数
   * @param fn 查询指定位置上有哪个面包屑片段的函数
   */
  function setQueryBreadcrumbAtPosition(fn: (position: { x: number, y: number }) => { canvasId: string, canvasName: string} | null) {
    queryBreadcrumbAtPosition = fn;
  }

  /**
   * 开始监听键盘事件与窗口失焦事件
   */
  function listenKeyboardEvents() {
    window.addEventListener('keydown', onAltKeyDown);
    window.addEventListener('keyup', onAltKeyUp);
    window.addEventListener('blur', onWindowBlur);
  }

  /**
   * 结束监听键盘事件与窗口失焦事件
   */
  function unlistenKeyboardEvents() {
    window.removeEventListener('keydown', onAltKeyDown);
    window.removeEventListener('keyup', onAltKeyUp);
    window.removeEventListener('blur', onWindowBlur);
  }

  /**
   * 处理用户开始拖动节点的行为，该事件需要依赖其它组件从 vue-flow 获取
   * @param draggedNodes 被拖动的节点 id 列表
   * @param allEdges 所有的边
   * @param mouseEvent 鼠标事件
   */
  function onDragStart(draggedNodes: VFNode[], allEdges: VFEdge[], mouseEvent: MouseEvent) {
    if (!active.value) {
      // 用户在状态机尚未启动时拖动节点，是“入口”操作
      validateStatusBeforeEntranceOperation('drag start');
      // 激活状态机
      active.value = true;
      // 切换至移动模式
      mode.value = 'move';
      // 更新节点集
      nodeSet.value = draggedNodes;
      // 更新节点集的迁移合法性
      nodeSetRelocatingLegality.value = calculateLegality(draggedNodes, allEdges);
      // 更新光标位置
      pointerPosition.value = { x: mouseEvent.clientX, y: mouseEvent.clientY };
      // 更新迁移目标
      relocatingTarget.value = calculateRelocatingTarget({ x: mouseEvent.clientX, y: mouseEvent.clientY });
    }
    else {
      // 更新节点集
      nodeSet.value = draggedNodes;
      // 更新节点集的迁移合法性
      nodeSetRelocatingLegality.value = calculateLegality(draggedNodes, allEdges);
      // 更新光标位置
      pointerPosition.value = { x: mouseEvent.clientX, y: mouseEvent.clientY };
      // 更新迁移目标
      relocatingTarget.value = calculateRelocatingTarget({ x: mouseEvent.clientX, y: mouseEvent.clientY });
    }
  }

  /**
   * 处理用户拖动节点的过程的行为，该事件需要依赖其它组件从 vue-flow 获取
   * @param draggedNodes 被拖动的节点 id 列表
   * @param allEdges 所有的边
   * @param mouseEvent 鼠标事件
   */
  function onDrag(draggedNodes: VFNode[], allEdges: VFEdge[], mouseEvent: MouseEvent) {
    if (active.value === false) {
      throw new Error('transition operation: drag: status validation failed: active === false: system inactive');
    }
    // 更新节点集
    nodeSet.value = draggedNodes;
    // 更新节点集的迁移合法性
    nodeSetRelocatingLegality.value = calculateLegality(draggedNodes, allEdges);
    // 更新光标位置
    pointerPosition.value = { x: mouseEvent.clientX, y: mouseEvent.clientY };
    // 更新迁移目标
    relocatingTarget.value = calculateRelocatingTarget({ x: mouseEvent.clientX, y: mouseEvent.clientY });
  }

  /**
   * 处理用户放下节点的行为，该事件需要依赖其它组件从 vue-flow 获取
   * @param draggedNodes 被拖动的节点列表
   * @param allEdges 所有的边
   * @param mouseEvent 鼠标事件
   */
  function onDragStop(draggedNodes: VFNode[], allEdges: VFEdge[], mouseEvent: MouseEvent) {
    if (active.value === false) {
      throw new Error('transition operation: drag stop: status validation failed: active === false: system inactive');
    }
    if (mode.value === null) {
      throw new Error('transition operation: drag stop: status validation failed: mode === null: missing mode');
    }
    // 更新节点集
    nodeSet.value = [];
    // 更新节点集的迁移合法性
    nodeSetRelocatingLegality.value = null
    // 更新光标位置
    pointerPosition.value = { x: mouseEvent.clientX, y: mouseEvent.clientY };
    // 更新迁移目标
    relocatingTarget.value = null;

    // 传播拖拽结束事件（先于出口操作，保证监听器拿到本次拖拽的有效模式与落点位置）
    let draggedNodesRelocatingLegality = calculateLegality(draggedNodes, allEdges);
    for (const effect of onDragStopEffects) {
      effect(mode.value, draggedNodes, draggedNodesRelocatingLegality, pointerPosition.value, calculateRelocatingTarget(pointerPosition.value));
    }

    // 用户在松开 alt 键时放下节点，是“出口”操作
    if (mode.value === 'move') {
      initialize();
    }
  }

  /**
   * 添加拖拽结束事件的监听器
   * @param listener 监听器
   */
  function listenOnDragStop(listener: OnDragStopEffectFn) {
    if (onDragStopEffects.findIndex(effect => effect === listener) === -1) {
      onDragStopEffects.push(listener);
    }
  }

  /**
   * 移除拖拽结束事件的监听器
   * @param listener 监听器
   */
  function unlistenOnDragStop(listener: OnDragStopEffectFn) {
    let index = onDragStopEffects.findIndex(effect => effect === listener);
    if (index !== -1) {
      onDragStopEffects.splice(index, 1);
    }
  }

  return {
    active,
    mode,
    nodeSet,
    nodeSetRelocatingLegality,
    pointerPosition,
    relocatingTarget,
    setQueryNodeAtPosition,
    setQueryBreadcrumbAtPosition,
    listenKeyboardEvents,
    unlistenKeyboardEvents,
    onDragStart,
    onDrag,
    onDragStop,
    listenOnDragStop,
    unlistenOnDragStop,
  }
}

/**
 * 以单例模式构造状态机，确保在不同的组件里引用的都是相同的状态机
 */
const SINGLETON: ReturnType<typeof useNodeMoveAndRelocate> = useNodeMoveAndRelocate();

export default SINGLETON;
