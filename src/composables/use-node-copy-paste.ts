/**
 * 节点复制粘贴的应用内存剪贴板与键盘快捷键。
 *
 * 剪贴板为模块级状态：复制时快照选中节点的 id 与坐标，粘贴时据此在目标画布创建副本，
 * 因此剪贴板可在不同画布之间共享；本模块不读写系统剪贴板。
 * 粘贴落点优先取最近一次指针位置（挂载期间监听 pointermove 记录），指针不在画布容器内时
 * 回退到容器中心；多节点按复制时的相对布局整体平移并逐轴吸附到网格。
 * 快捷键为 Ctrl/Cmd+C 与 Ctrl/Cmd+V，仅在无对话框/菜单/面板打开且焦点不在输入控件内时生效。
 */
import { nextTick, onMounted, onUnmounted, ref, type Ref } from "vue";
import { useVueFlow } from "@vue-flow/core";
import type { GraphNode, Node as VFNode } from "@vue-flow/core";
import { userDatabaseNodeCopy } from "@/api";
import { toVFNode } from "@/vf-convert";
import { DATA_NODE_WIDTH, DATA_NODE_HEIGHT } from "@/node-size";
import { isErrorCode } from "@/error-code";
import { snackbarErrorCode } from "@/composables/use-snackbar";

/** 剪贴板条目：复制时快照的节点 id 与坐标 */
interface NodeClipboardItem {
  /** 被复制节点的 id */
  nodeId: string;
  /** 复制时节点的 x 坐标（左上角） */
  x: number;
  /** 复制时节点的 y 坐标（左上角） */
  y: number;
}

/** 应用内存剪贴板（模块级单例，跨画布共享） */
const clipboardItems: Ref<NodeClipboardItem[]> = ref([]);

/** 最近一次指针位置（屏幕坐标），挂载期间由 pointermove 更新；null 表示尚未收到指针事件 */
let lastPointerPosition: { x: number; y: number } | null = null;

/** setupNodeCopyPaste 的配置项 */
interface NodeCopyPasteOptions {
  /** 副本归属的目标画布 id（当前画布） */
  canvasId: string;
  /** 当前画布的节点列表 */
  nodes: Ref<VFNode[]>;
  /** 画布容器元素 */
  containerRef: Ref<HTMLElement | undefined>;
  /** 吸附网格 [x, y] */
  snapGrid: [number, number];
  /** 判断复制粘贴快捷键当前是否可用（对话框/菜单/面板打开时不可用） */
  isShortcutEnabled: () => boolean;
}

/**
 * 在画布页面的 setup 中调用：注册复制粘贴快捷键与指针位置监听，组件卸载时注销。
 * @param options 配置项
 * @returns 无返回值
 */
export function setupNodeCopyPaste(options: NodeCopyPasteOptions): void {
  const { getSelectedNodes, addSelectedNodes, removeSelectedNodes, findNode, screenToFlowCoordinate } = useVueFlow();

  /**
   * 把当前选中节点快照写入剪贴板：影子节点与画布数据节点不参与复制，
   * 过滤后为空时不改变剪贴板。
   * @returns 无返回值
   */
  function copySelection(): void {
    const selected = getSelectedNodes.value.filter(
      (n) => !n.data.shadowOriginId && !n.data.canvasRefId,
    );
    if (selected.length === 0) return;
    clipboardItems.value = selected.map((n) => ({ nodeId: n.id, x: n.position.x, y: n.position.y }));
  }

  /**
   * 计算粘贴落点（画布坐标）：最近指针位置在画布容器内时取该位置，否则取容器中心。
   * @param rect 画布容器的视口矩形
   * @returns 粘贴落点（画布坐标）
   */
  function resolveDropPoint(rect: DOMRect): { x: number; y: number } {
    const pointer = lastPointerPosition;
    const inside =
      pointer !== null &&
      pointer.x >= rect.left &&
      pointer.x <= rect.right &&
      pointer.y >= rect.top &&
      pointer.y <= rect.bottom;
    const screenPoint = inside ? pointer : { x: rect.left + rect.width / 2, y: rect.top + rect.height / 2 };
    return screenToFlowCoordinate(screenPoint);
  }

  /**
   * 把剪贴板中的节点逐条复制到当前画布：多节点按复制时的相对布局整体平移，
   * 使包围盒中心落在粘贴落点，并逐轴吸附到网格；源节点已被删除时跳过该节点，
   * 其它错误提示后中止剩余粘贴（已粘贴的副本保留）；全部处理完成后选中新副本。
   * @returns 无返回值
   */
  async function paste(): Promise<void> {
    const items = clipboardItems.value;
    if (items.length === 0) return;
    const container = options.containerRef.value;
    if (!container) return;
    const dropPoint = resolveDropPoint(container.getBoundingClientRect());

    const minX = Math.min(...items.map((i) => i.x));
    const minY = Math.min(...items.map((i) => i.y));
    const maxX = Math.max(...items.map((i) => i.x)) + DATA_NODE_WIDTH;
    const maxY = Math.max(...items.map((i) => i.y)) + DATA_NODE_HEIGHT;
    const dx = dropPoint.x - (minX + maxX) / 2;
    const dy = dropPoint.y - (minY + maxY) / 2;

    const createdIds: string[] = [];
    for (const item of items) {
      const x = Math.round((item.x + dx) / options.snapGrid[0]) * options.snapGrid[0];
      const y = Math.round((item.y + dy) / options.snapGrid[1]) * options.snapGrid[1];
      try {
        const created = await userDatabaseNodeCopy(item.nodeId, options.canvasId, x, y);
        options.nodes.value.push(toVFNode(created));
        createdIds.push(created.id);
      } catch (e) {
        if (isErrorCode(e, "NoNodeWithSuchId")) continue;
        snackbarErrorCode(e);
        break;
      }
    }

    if (createdIds.length === 0) return;
    await nextTick();
    const createdNodes = createdIds
      .map((id) => findNode(id))
      .filter((n): n is GraphNode => n !== undefined);
    removeSelectedNodes([...getSelectedNodes.value]);
    addSelectedNodes(createdNodes);
  }

  /**
   * 处理 window 的 keydown：焦点在输入控件内、修饰键不匹配或快捷键不可用时放行；
   * 命中 Ctrl/Cmd+C 时复制选中节点，命中 Ctrl/Cmd+V 时在落点粘贴。
   * @param event 键盘事件
   * @returns 无返回值
   */
  function onKeyDown(event: KeyboardEvent): void {
    const target = event.target;
    if (
      target instanceof HTMLElement &&
      (target.tagName === "INPUT" || target.tagName === "TEXTAREA" || target.isContentEditable)
    ) {
      return;
    }
    if (!(event.ctrlKey || event.metaKey)) return;
    if (event.shiftKey || event.altKey) return;
    const key = event.key.toLowerCase();
    if (key === "c") {
      if (!options.isShortcutEnabled()) return;
      event.preventDefault();
      copySelection();
      return;
    }
    if (key === "v") {
      if (!options.isShortcutEnabled()) return;
      event.preventDefault();
      void paste();
    }
  }

  /**
   * 记录最新指针位置，供粘贴时定位落点。
   * @param event 指针事件
   * @returns 无返回值
   */
  function onPointerMove(event: PointerEvent): void {
    lastPointerPosition = { x: event.clientX, y: event.clientY };
  }

  onMounted(() => {
    window.addEventListener("keydown", onKeyDown);
    window.addEventListener("pointermove", onPointerMove, { passive: true });
  });

  onUnmounted(() => {
    window.removeEventListener("keydown", onKeyDown);
    window.removeEventListener("pointermove", onPointerMove);
  });
}
