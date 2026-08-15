/**
 * 画布回收站状态管理。
 * 与节点回收站（use-recycle-bin.ts）的区别在于所有变更后整体重载列表以消化后端的级联行为。
 * 维护画布宇宙中已逻辑删除画布列表，提供逻辑删除/恢复/物理删除/清空操作。
 */
import { ref, type Ref } from "vue";
import type { Canvas } from "@/api-types";
import {
  userDatabaseCanvasList,
  userDatabaseCanvasLogicalDelete,
  userDatabaseCanvasRestore,
  userDatabaseCanvasPhysicalDelete,
} from "@/api";
import { snackbarErrorCode } from "@/composables/use-snackbar";
import { isErrorCode } from "@/error-code";

export function useCanvasRecycleBin() {
  const deletedCanvases: Ref<Canvas[]> = ref([]);

  /** 加载已逻辑删除画布列表 */
  async function load(): Promise<void> {
    try {
      deletedCanvases.value = await userDatabaseCanvasList(true);
    } catch (e) {
      snackbarErrorCode(e);
    }
  }

  /**
   * 逻辑删除画布。
   * 后端会级联逻辑删除整个子画布树，成功后整体重载回收站列表以同步级联结果。
   * 返回 true，失败 snackbar 返回 false。
   */
  async function logicalDelete(id: string): Promise<boolean> {
    try {
      await userDatabaseCanvasLogicalDelete(id);
      await load();
      return true;
    } catch (e) {
      snackbarErrorCode(e);
      return false;
    }
  }

  /**
   * 恢复画布到指定坐标。
   * 后端会连带恢复已删除的祖先画布，成功后整体重载回收站列表以消化级联恢复行为。
   * 返回 true，失败 snackbar 返回 false。
   */
  async function restore(canvas: Canvas, x: number, y: number): Promise<boolean> {
    try {
      await userDatabaseCanvasRestore(canvas.id, x, y);
      await load();
      return true;
    } catch (e) {
      snackbarErrorCode(e);
      return false;
    }
  }

  /**
   * 物理删除画布及其子画布树。
   * 后端会连带物理删除子树中已在回收站的画布，成功后整体重载回收站列表。
   * 返回 true，失败 snackbar 返回 false。
   */
  async function physicalDelete(canvas: Canvas): Promise<boolean> {
    try {
      await userDatabaseCanvasPhysicalDelete(canvas.id);
      await load();
      return true;
    } catch (e) {
      snackbarErrorCode(e);
      return false;
    }
  }

  /**
   * 清空回收站：遍历副本逐个物理删除。
   * 容错：若画布已作为祖先的子孙被连带物理删除（NoCanvasWithSuchId）则跳过；
   * 其它错误 snackbar 并中断；循环结束后整体重载列表以同步状态。
   */
  async function empty(): Promise<void> {
    const items = [...deletedCanvases.value];
    for (const canvas of items) {
      try {
        await userDatabaseCanvasPhysicalDelete(canvas.id);
      } catch (e) {
        if (isErrorCode(e, "NoCanvasWithSuchId")) {
          continue;
        }
        snackbarErrorCode(e);
        break;
      }
    }
    await load();
  }

  return { deletedCanvases, load, logicalDelete, restore, physicalDelete, empty };
}
