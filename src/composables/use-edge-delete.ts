/**
 * 删除边并处理由此引发的影子断连确认。
 *
 * 删除边会级联删除其产生的影子节点（可能跨越多级画布），若这些影子在子画布内还
 * 有关联节点，后端以 EdgeDeleteDisconnectsNodes 返回受影响节点标题，由调用方确认后
 * 以 confirmed=true 重试。删除边与删除影子节点（等价于删除产生它的边）两条路径共用本流程。
 */
import { userDatabaseEdgeDelete } from "@/api";
import { snackbarErrorCode } from "@/composables/use-snackbar";
import { isErrorCode } from "@/error-code";

/**
 * 删除指定边，并在该删除会断开影子节点的关联连接时请求调用方确认后重试。
 *
 * 先以未确认姿态调用后端；后端以 EdgeDeleteDisconnectsNodes 返回受影响节点标题时，
 * 把标题列表交给 requestConfirm，用户确认后再以 confirmed=true 重试。其它错误与重试
 * 失败均无重试语义，交给 snackbar 展示。
 *
 * @param edgeId 待删除的边 id
 * @param requestConfirm 请求用户确认的回调，入参为受影响的节点标题列表；resolve 出 false 表示用户取消，此时不执行删除
 * @returns 边已被删除时返回 true；删除失败或用户取消时返回 false
 */
export async function deleteEdgeWithDisconnectConfirm(
  edgeId: string,
  requestConfirm: (affectedNodes: string[]) => Promise<boolean>,
): Promise<boolean> {
  try {
    await userDatabaseEdgeDelete(edgeId, false);
    return true;
  } catch (e) {
    // 非断连错误（边不存在、端点缺失、数据损坏等）：无重试语义，直接展示
    if (!isErrorCode(e, "EdgeDeleteDisconnectsNodes")) {
      snackbarErrorCode(e);
      return false;
    }
    // 载荷缺失或类型不符是防御性场景，退化为空列表继续走确认流程，不阻塞删除
    const rawNodes = e.data?.nodes;
    const affected = Array.isArray(rawNodes) ? rawNodes.map(String) : [];
    if (!(await requestConfirm(affected))) return false;
  }
  try {
    await userDatabaseEdgeDelete(edgeId, true);
    return true;
  } catch (e) {
    snackbarErrorCode(e);
    return false;
  }
}
