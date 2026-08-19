import { describe, expect, it } from "vitest";

import type { Canvas } from "@/api-types";
import { collapseChain } from "./canvas-breadcrumb";

/** 构造测试用画布，名称与 id 相同以便断言。 */
function canvas(id: string, parentId: string | null): Canvas {
  return { id, parent_id: parentId, name: id, x: 0, y: 0, deleted: false, color: "" };
}

/** 按给定 id 顺序构造层级链：首元素为根画布，其余依次链接为父子关系。 */
function chainOf(...ids: string[]): Canvas[] {
  return ids.map((id, index) => canvas(id, index === 0 ? null : ids[index - 1]));
}

describe("collapseChain", () => {
  it("链长 1（当前即根画布）：不折叠", () => {
    // 意图：当前画布就是根画布，不存在上一级画布，唯一节点直接可见。
    const chain = chainOf("root");
    expect(collapseChain(chain)).toEqual({ collapsed: false, visible: chain });
  });

  it("链长 2（根 > 当前）：不折叠", () => {
    // 意图：根画布同时就是上一级画布，两层均直接可见，无中间节点可折叠。
    const chain = chainOf("root", "cur");
    expect(collapseChain(chain)).toEqual({ collapsed: false, visible: chain });
  });

  it("链长 3（根 > 父 > 当前）：不折叠", () => {
    // 意图：根画布与上一级画布固定显示后已覆盖全链，三层全部直接可见。
    const chain = chainOf("root", "parent", "cur");
    expect(collapseChain(chain)).toEqual({ collapsed: false, visible: chain });
  });

  it("链长 4：唯一中间画布仍折叠，根与上一级固定保留", () => {
    // 意图：根 > a > 父 > 当前，中间仅 a 一层也折叠进省略号（规则统一），
    // hidden 恰为 [a]，root/parent/current 分别指向链首、尾二位、链尾。
    const chain = chainOf("root", "a", "parent", "cur");
    const result = collapseChain(chain);
    expect(result.collapsed).toBe(true);
    if (!result.collapsed) return;
    expect(result.root.id).toBe("root");
    expect(result.parent.id).toBe("parent");
    expect(result.current.id).toBe("cur");
    expect(result.hidden.map((c) => c.id)).toEqual(["a"]);
  });

  it("链长 6：折叠全部中间画布且保持从根到父的顺序", () => {
    // 意图：根 > a > b > c > 父 > 当前，hidden 依序为 [a, b, c]，
    // 使省略号菜单自上而下对应从根到父的导航路径。
    const chain = chainOf("root", "a", "b", "c", "parent", "cur");
    const result = collapseChain(chain);
    expect(result.collapsed).toBe(true);
    if (!result.collapsed) return;
    expect(result.root.id).toBe("root");
    expect(result.parent.id).toBe("parent");
    expect(result.current.id).toBe("cur");
    expect(result.hidden.map((c) => c.id)).toEqual(["a", "b", "c"]);
  });
});
