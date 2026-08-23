/**
 * 布局算法内部共享工具：确定性比较器、角度归一化、并查集。
 *
 * 完全确定性是算法不变量之一：所有排序平局按节点 id 决胜，
 * 所有浮点求和先排序以消除输入顺序带来的舍入差异。
 */

/** 字符串 id 的确定性比较器（按 UTF-16 码元序）。 */
export function compareId(a: string, b: string): number {
  return a < b ? -1 : a > b ? 1 : 0;
}

/**
 * 将角度归一化到 [0, 2π)。
 *
 * @param angle 任意角度（弧度）。
 * @returns 归一化后的角度（弧度）。
 */
export function normalizeAngle(angle: number): number {
  const fullCircle = 2 * Math.PI;
  return ((angle % fullCircle) + fullCircle) % fullCircle;
}

/** 并查集：按边（无视方向）把节点聚合为连通分量。 */
export class UnionFind {
  private readonly parent = new Map<string, string>();

  /**
   * 查找节点所在集合的根（带路径压缩）。
   *
   * @param id 节点 id。
   * @returns 根节点 id；未加入任何集合的节点以自身为根。
   */
  find(id: string): string {
    const parent = this.parent.get(id);
    if (parent === undefined || parent === id) {
      return id;
    }
    const root = this.find(parent);
    this.parent.set(id, root);
    return root;
  }

  /**
   * 合并两个节点所在的集合。
   *
   * @param a 节点 id。
   * @param b 节点 id。
   */
  union(a: string, b: string): void {
    this.parent.set(this.find(a), this.find(b));
  }
}
