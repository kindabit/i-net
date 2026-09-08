import { beforeEach, describe, expect, it, vi } from "vitest";

import { userDatabaseEdgeDelete } from "@/api";
import { snackbarErrorCode } from "@/composables/use-snackbar";
import { deleteEdgeWithDisconnectConfirm } from "./use-edge-delete";

vi.mock("@/api", () => ({ userDatabaseEdgeDelete: vi.fn() }));
vi.mock("@/composables/use-snackbar", () => ({ snackbarErrorCode: vi.fn() }));

/** 构造断连错误：与后端 EdgeDeleteDisconnectsNodes 的传输结构一致 */
function disconnectError(nodes: unknown) {
  return { variant: "EdgeDeleteDisconnectsNodes", data: { nodes } };
}

describe("deleteEdgeWithDisconnectConfirm", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("无断连影响时以未确认姿态一次删除成功", async () => {
    // 意图：常态路径——后端直接接受删除，不应请求确认，也不应二次调用
    vi.mocked(userDatabaseEdgeDelete).mockResolvedValueOnce(undefined);
    const requestConfirm = vi.fn().mockResolvedValue(true);

    const result = await deleteEdgeWithDisconnectConfirm("edge-1", requestConfirm);

    expect(result).toBe(true);
    expect(userDatabaseEdgeDelete).toHaveBeenCalledTimes(1);
    expect(userDatabaseEdgeDelete).toHaveBeenCalledWith("edge-1", false);
    expect(requestConfirm).not.toHaveBeenCalled();
  });

  it("断连且用户确认时把受影响节点交给确认回调并以 confirmed=true 重试", async () => {
    // 意图：核心路径——后端提示断连后须带上受影响节点标题请求确认，确认后才重试删除
    vi.mocked(userDatabaseEdgeDelete)
      .mockRejectedValueOnce(disconnectError(["节点甲", "节点乙"]))
      .mockResolvedValueOnce(undefined);
    const requestConfirm = vi.fn().mockResolvedValue(true);

    const result = await deleteEdgeWithDisconnectConfirm("edge-1", requestConfirm);

    expect(result).toBe(true);
    expect(requestConfirm).toHaveBeenCalledWith(["节点甲", "节点乙"]);
    expect(userDatabaseEdgeDelete).toHaveBeenNthCalledWith(2, "edge-1", true);
  });

  it("用户取消时不重试并返回 false", async () => {
    // 意图：用户拒绝断连影响时，边必须保持存在，不得执行确认态删除
    vi.mocked(userDatabaseEdgeDelete).mockRejectedValueOnce(disconnectError(["节点甲"]));
    const requestConfirm = vi.fn().mockResolvedValue(false);

    const result = await deleteEdgeWithDisconnectConfirm("edge-1", requestConfirm);

    expect(result).toBe(false);
    expect(userDatabaseEdgeDelete).toHaveBeenCalledTimes(1);
  });

  it("非断连错误时交由 snackbar 展示且不重试", async () => {
    // 意图：边不存在等错误没有重试语义，应提示后直接返回失败
    const error = { variant: "NoEdgeWithSuchId", data: {} };
    vi.mocked(userDatabaseEdgeDelete).mockRejectedValueOnce(error);
    const requestConfirm = vi.fn().mockResolvedValue(true);

    const result = await deleteEdgeWithDisconnectConfirm("edge-1", requestConfirm);

    expect(result).toBe(false);
    expect(snackbarErrorCode).toHaveBeenCalledWith(error);
    expect(requestConfirm).not.toHaveBeenCalled();
    expect(userDatabaseEdgeDelete).toHaveBeenCalledTimes(1);
  });

  it("确认后重试仍失败时提示并返回 false", async () => {
    // 意图：确认态删除仍可能失败（如并发删除），须提示用户且不得误报成功
    const error = { variant: "NoEdgeWithSuchId", data: {} };
    vi.mocked(userDatabaseEdgeDelete)
      .mockRejectedValueOnce(disconnectError(["节点甲"]))
      .mockRejectedValueOnce(error);

    const result = await deleteEdgeWithDisconnectConfirm("edge-1", async () => true);

    expect(result).toBe(false);
    expect(snackbarErrorCode).toHaveBeenCalledWith(error);
  });

  it("断连错误的 nodes 载荷缺失时以空列表请求确认", async () => {
    // 意图：载荷缺失是防御性场景，不应崩溃，仍照常走确认流程
    vi.mocked(userDatabaseEdgeDelete)
      .mockRejectedValueOnce(disconnectError(undefined))
      .mockResolvedValueOnce(undefined);
    const requestConfirm = vi.fn().mockResolvedValue(true);

    const result = await deleteEdgeWithDisconnectConfirm("edge-1", requestConfirm);

    expect(result).toBe(true);
    expect(requestConfirm).toHaveBeenCalledWith([]);
  });
});
