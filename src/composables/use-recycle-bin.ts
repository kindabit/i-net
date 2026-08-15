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

  /** 物理删除单个节点 */
  async function physicalDelete(node: Node): Promise<boolean> {
    try {
      await userDatabaseNodePhysicalDelete(node.id);
      deletedNodes.value = deletedNodes.value.filter((n) => n.id !== node.id);
      return true;
    } catch (e) {
      snackbarErrorCode(e);
      return false;
    }
  }

  /** 清空回收站：前端循环逐个物理删除 */
  async function empty(): Promise<void> {
    const items = [...deletedNodes.value];
    for (const node of items) {
      try {
        await userDatabaseNodePhysicalDelete(node.id);
        deletedNodes.value = deletedNodes.value.filter((n) => n.id !== node.id);
      } catch (e) {
        snackbarErrorCode(e);
        break;
      }
    }
  }

  return { deletedNodes, load, logicalDelete, restore, physicalDelete, empty };
}
