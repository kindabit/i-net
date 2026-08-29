import { describe, expect, it } from "vitest";

import {
  computeHighlightedEdgeIds,
  computeHighlightedNodeIds,
} from "./use-neighbor-highlight";

/** 构造测试用边（仅派生逻辑所需的三个字段）。 */
function edge(id: string, source: string, target: string) {
  return { id, source, target };
}

describe("computeHighlightedEdgeIds", () => {
  it("选中集合为空时返回空集合", () => {
    // 意图：无选中节点是常态边界，任何边都不应高亮。
    const result = computeHighlightedEdgeIds(new Set(), [edge("e1", "a", "b")]);
    expect(result.size).toBe(0);
  });

  it("边列表为空时返回空集合", () => {
    // 意图：选中节点存在但画布无边，是合法边界，应返回空而非报错。
    const result = computeHighlightedEdgeIds(new Set(["a"]), []);
    expect(result.size).toBe(0);
  });

  it("选中节点无相连边时返回空集合", () => {
    // 意图：孤立节点被选中时不应有任何边高亮。
    const result = computeHighlightedEdgeIds(new Set(["a"]), [edge("e1", "b", "c")]);
    expect(result.size).toBe(0);
  });

  it("单选节点时仅其相连边进入集合", () => {
    // 意图：核心路径——作为 source 或 target 的边都算相连；不相邻边不高亮。
    const result = computeHighlightedEdgeIds(new Set(["b"]), [
      edge("e1", "a", "b"), // b 是 target，相连
      edge("e2", "b", "c"), // b 是 source，相连
      edge("e3", "c", "d"), // 与 b 无关
    ]);
    expect(result).toEqual(new Set(["e1", "e2"]));
  });

  it("多选节点时取相连边的并集且去重", () => {
    // 意图：多选是已确认需求；两个选中节点共享的边只应出现一次。
    const result = computeHighlightedEdgeIds(new Set(["a", "b"]), [
      edge("e1", "a", "b"), // 同时与 a、b 相连，去重验证
      edge("e2", "a", "c"), // 仅与 a 相连
      edge("e3", "d", "b"), // 仅与 b 相连
      edge("e4", "c", "d"), // 与 a、b 均无关
    ]);
    expect(result).toEqual(new Set(["e1", "e2", "e3"]));
  });

  it("自环边在节点被选中时进入集合", () => {
    // 意图：source === target 的自环边与该节点相连，不应被漏掉。
    const result = computeHighlightedEdgeIds(new Set(["a"]), [
      edge("e1", "a", "a"),
      edge("e2", "b", "c"),
    ]);
    expect(result).toEqual(new Set(["e1"]));
  });
});

describe("computeHighlightedNodeIds", () => {
  it("选中边列表为空时返回空集合", () => {
    // 意图：无选中边是常态边界，任何节点都不应因边而高亮。
    const result = computeHighlightedNodeIds([]);
    expect(result.size).toBe(0);
  });

  it("单条边时其 source 与 target 均进入集合", () => {
    // 意图：核心路径——边的两个端点都应高亮。
    const result = computeHighlightedNodeIds([{ source: "a", target: "b" }]);
    expect(result).toEqual(new Set(["a", "b"]));
  });

  it("多条边共享端点时去重", () => {
    // 意图：边允许多选（已确认需求）；共享端点只应出现一次。
    const result = computeHighlightedNodeIds([
      { source: "a", target: "b" },
      { source: "b", target: "c" },
    ]);
    expect(result).toEqual(new Set(["a", "b", "c"]));
  });

  it("多条无关联边时所有端点均进入集合", () => {
    // 意图：多选互不相关的边时，所有端点都应高亮。
    const result = computeHighlightedNodeIds([
      { source: "a", target: "b" },
      { source: "c", target: "d" },
    ]);
    expect(result).toEqual(new Set(["a", "b", "c", "d"]));
  });

  it("自环边仅产生一个节点", () => {
    // 意图：source === target 的自环边只对应一个端点节点，不应重复。
    const result = computeHighlightedNodeIds([{ source: "a", target: "a" }]);
    expect(result).toEqual(new Set(["a"]));
  });
});
