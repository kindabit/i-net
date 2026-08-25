/**
 * 回收站状态管理。
 * 维护当前画布已逻辑删除节点列表，提供逻辑删除/恢复/物理删除/清空操作。
 */
import { ref, type Ref } from "vue";
import type { Node } from "@/api-types";
import {
  userDatabaseNodeList,
  userDatabaseNodeLogicalDelete,
  userDatabaseNodeRestore,
  userDatabaseNodePhysicalDelete,
} from "@/api";
import { snackbarErrorCode } from "@/composables/use-snackbar";
import { isErrorCode } from "@/error-code";

export function useRecycleBin(canvasId: string) {
  const deletedNodes: Ref<Node[]> = ref([]);

  /** 加载已逻辑删除节点列表 */
  async function load(): Promise<void> {
    try {
      deletedNodes.value = await userDatabaseNodeList(canvasId, true);
    } catch (e) {
      snackbarErrorCode(e);
    }
  }

  /** 逻辑删除节点。成功：API 返回的被删节点加入回收站列表，返回 true；失败 snackbar 返回 false */
  async function logicalDelete(id: string): Promise<boolean> {
    try {
      const node = await userDatabaseNodeLogicalDelete(id);
      deletedNodes.value.push(node);
      return true;
    } catch (e) {
      snackbarErrorCode(e);
      return false;
    }
  }

  /** 恢复节点到指定坐标 */
  async function restore(node: Node, x: number, y: number): Promise<boolean> {
    try {
      await userDatabaseNodeRestore(node.id, x, y);
      deletedNodes.value = deletedNodes.value.filter((n) => n.id !== node.id);
      return true;
    } catch (e) {
      snackbarErrorCode(e);
      return false;
    }
  }

  /**
   * 物理删除单个节点（双阶段确认）。
   * 先以 confirmed=false 调用；若后端提示该节点的影子子树在其它画布中存在关联节点
   * （返回 NodeDeleteDisconnectsNodes），将该错误原样上抛，调用方据此弹出断连确认
   * 对话框并在用户确认后以 confirmed=true 重试；其它错误走 snackbar。
   * @param node 待物理删除的节点
   * @param confirmed 是否已确认级联断开节点连接
   * @returns 删除成功返回 true；NodeDeleteDisconnectsNodes 时直接上抛；其它失败返回 false
   */
  async function physicalDelete(node: Node, confirmed: boolean): Promise<boolean> {
    try {
      await userDatabaseNodePhysicalDelete(node.id, confirmed);
      deletedNodes.value = deletedNodes.value.filter((n) => n.id !== node.id);
      return true;
    } catch (e) {
      if (!confirmed && isErrorCode(e, "NodeDeleteDisconnectsNodes")) {
        throw e;
      }
      snackbarErrorCode(e);
      return false;
    }
  }

  /**
   * 清空回收站：前端循环逐个物理删除。
   * 清空本身已是一次性确认，逐节点以 confirmed=true 调用，跳过后端的断连检测，
   * 不逐节点弹断连确认对话框。
   */
  async function empty(): Promise<void> {
    const items = [...deletedNodes.value];
    for (const node of items) {
      try {
        await userDatabaseNodePhysicalDelete(node.id, true);
        deletedNodes.value = deletedNodes.value.filter((n) => n.id !== node.id);
      } catch (e) {
        snackbarErrorCode(e);
        break;
      }
    }
  }

  return { deletedNodes, load, logicalDelete, restore, physicalDelete, empty };
}
