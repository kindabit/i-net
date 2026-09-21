// UI 树的查询、等待与基于坐标的点击/输入辅助。

import * as api from "./api.js";
import { sleep, waitForCondition } from "./util.js";

// ---------- 查询 ----------

/**
 * 深度优先展开 UI 树。
 * @param {object[]} nodes 顶层节点数组。
 * @returns {{node: object, depth: number}[]} 带深度的节点列表（先序）。
 */
export function flatten(nodes) {
  const out = [];
  const walk = (node, depth) => {
    out.push({ node, depth });
    for (const child of node.children ?? []) {
      walk(child, depth + 1);
    }
  };
  for (const node of nodes ?? []) {
    walk(node, 0);
  }
  return out;
}

/**
 * 组合节点的 name 与 text 作为自身文本。
 * @param {object} node UI 树节点。
 * @returns {string} 自身文本（已去除首尾空白）。
 */
export function ownText(node) {
  return [node.name, node.text].filter(Boolean).join(" ").trim();
}

/**
 * 递归拼接节点及其子树的文本。
 * @param {object} node UI 树节点。
 * @returns {string} 子树整体文本。
 */
export function subtreeText(node) {
  let text = ownText(node);
  for (const child of node.children ?? []) {
    const childText = subtreeText(child);
    if (childText) {
      text += ` ${childText}`;
    }
  }
  return text.trim();
}

/**
 * 计算节点 bounds 的中心坐标。
 * @param {object} node UI 树节点。
 * @returns {{x: number, y: number}} 中心坐标（整窗物理像素）。
 */
export function boundsCenter(node) {
  const b = node.bounds;
  return { x: b.left + b.width / 2, y: b.top + b.height / 2 };
}

/**
 * 在 UI 树（或指定子树）中查找自身文本匹配的节点。
 * @param {object[]} nodes 顶层节点数组。
 * @param {string} text 要匹配的文本。
 * @param {object} [options] 可选参数。
 * @param {boolean} [options.exact=false] 是否精确匹配（否则为包含匹配）。
 * @param {object|null} [options.root=null] 可选子树根节点；缺省搜索整树。
 * @returns {object|null} 命中的最深节点；未命中时为 null。
 */
export function findByText(nodes, text, { exact = false, root = null } = {}) {
  const entries = root ? flatten([root]) : flatten(nodes);
  const hits = entries.filter(({ node }) => {
    const own = ownText(node);
    const matched = exact ? own === text : own.includes(text);
    return matched && node.bounds && node.bounds.width > 0;
  });
  hits.sort((a, b) => b.depth - a.depth);
  return hits.length > 0 ? hits[0].node : null;
}

/**
 * 在 UI 树（或指定子树）中查找指定 role 的节点。
 * @param {object[]} nodes 顶层节点数组。
 * @param {string} role 目标 role（如 button、textbox、dialog）。
 * @param {object} [options] 可选参数。
 * @param {object|null} [options.root=null] 可选子树根节点；缺省搜索整树。
 * @returns {object[]} 命中节点列表（先序）。
 */
export function findByRole(nodes, role, { root = null } = {}) {
  const entries = root ? flatten([root]) : flatten(nodes);
  return entries
    .map(({ node }) => node)
    .filter((node) => node.role === role);
}

/**
 * 在 UI 树（或指定子树）中查找可编辑控件（name 包含指定片段）。
 * @param {object[]} nodes 顶层节点数组。
 * @param {string} nameFragment 名称片段（如标签文本）。
 * @param {object} [options] 可选参数。
 * @param {object|null} [options.root=null] 可选子树根节点；缺省搜索整树。
 * @returns {object|null} 首个命中的可编辑节点；未命中时为 null。
 */
export function findEditable(nodes, nameFragment, { root = null } = {}) {
  const entries = root ? flatten([root]) : flatten(nodes);
  const hits = entries
    .map(({ node }) => node)
    .filter(
      (node) =>
        node.editable &&
        ownText(node).includes(nameFragment) &&
        node.bounds &&
        node.bounds.width > 0,
    );
  return hits.length > 0 ? hits[0] : null;
}

/**
 * 在 UI 树（或指定子树）中查找文本匹配的按钮。
 * @param {object[]} nodes 顶层节点数组。
 * @param {string} text 按钮文本（含 name）。
 * @param {object} [options] 可选参数。
 * @param {object|null} [options.root=null] 可选子树根节点；缺省搜索整树。
 * @param {boolean} [options.exact=false] 是否精确匹配子树文本。
 * @returns {object|null} 命中的按钮节点；未命中时为 null。
 */
export function findButton(nodes, text, { root = null, exact = false } = {}) {
  const entries = root ? flatten([root]) : flatten(nodes);
  const buttons = entries
    .map(({ node }) => node)
    .filter((node) => node.role === "button" && node.bounds && node.bounds.width > 0);
  for (const button of buttons) {
    const whole = subtreeText(button);
    if (exact ? whole === text : whole.includes(text)) {
      return button;
    }
  }
  return null;
}

/**
 * 在 UI 树中查找包含指定标题文本的对话框（取面积最小者，即最内层）。
 * @param {object[]} nodes 顶层节点数组。
 * @param {string} titleText 对话框标题或内容片段。
 * @returns {object|null} 命中的对话框节点；未命中时为 null。
 */
export function findDialog(nodes, titleText) {
  const dialogs = findByRole(nodes, "dialog").filter(
    (d) => d.bounds && d.bounds.width > 100 && d.bounds.height > 80,
  );
  const hits = dialogs.filter((d) => subtreeText(d).includes(titleText));
  if (hits.length === 0) {
    return null;
  }
  hits.sort(
    (a, b) => a.bounds.width * a.bounds.height - b.bounds.width * b.bounds.height,
  );
  return hits[0];
}

/**
 * 在 UI 树（或指定子树）中查找具有指定属性值的节点。
 * @param {object[]} nodes 顶层节点数组。
 * @param {string} attrName 属性名（如 title、data-handlepos）。
 * @param {string} attrValue 属性值。
 * @param {object} [options] 可选参数。
 * @param {object|null} [options.root=null] 可选子树根节点；缺省搜索整树。
 * @returns {object|null} 首个命中的节点；未命中时为 null。
 */
export function findAttr(nodes, attrName, attrValue, { root = null } = {}) {
  const entries = root ? flatten([root]) : flatten(nodes);
  for (const { node } of entries) {
    if (
      node.attrs &&
      node.attrs[attrName] === attrValue &&
      node.bounds &&
      node.bounds.width > 0
    ) {
      return node;
    }
  }
  return null;
}

/**
 * 判断 UI 树中是否存在匹配文本的节点。
 * @param {object[]} nodes 顶层节点数组。
 * @param {string} text 要匹配的文本。
 * @param {object} [options] 可选参数（见 findByText）。
 * @returns {boolean} 存在时为 true。
 */
export function hasText(nodes, text, options = {}) {
  return findByText(nodes, text, options) !== null;
}

// ---------- 等待 ----------

/**
 * 等待指定文本出现。
 * @param {string} text 目标文本。
 * @param {object} [options] 可选参数。
 * @param {number} [options.timeout=10000] 超时毫秒数。
 * @param {number} [options.interval=250] 轮询间隔毫秒数。
 * @param {boolean} [options.exact=false] 是否精确匹配。
 * @returns {Promise<object>} 命中的节点。
 */
export async function waitForText(text, { timeout = 10000, interval = 250, exact = false } = {}) {
  return waitForCondition(
    async () => findByText(await api.uiTree(), text, { exact }),
    { timeout, interval, label: `text "${text}"` },
  );
}

/**
 * 等待指定文本出现，并在短暂静止期后复查其仍然存在。
 * 用于规避 Vuetify 消息过渡期间新旧文本瞬时并存造成的误判与截图中间态。
 * @param {string} text 目标文本。
 * @param {object} [options] 可选参数。
 * @param {number} [options.timeout=10000] 超时毫秒数。
 * @param {number} [options.settleMs=400] 首次命中后的静止等待毫秒数。
 * @param {boolean} [options.exact=false] 是否精确匹配。
 * @returns {Promise<object>} 复查后仍命中的节点。
 */
export async function waitForTextStable(text, { timeout = 10000, settleMs = 400, exact = false } = {}) {
  const node = await waitForText(text, { timeout, exact });
  await sleep(settleMs);
  const again = findByText(await api.uiTree(), text, { exact });
  if (!again) {
    throw new Error(`text "${text}" disappeared within the ${settleMs}ms settle window`);
  }
  return again;
}

/**
 * 等待指定文本消失。
 * @param {string} text 目标文本。
 * @param {object} [options] 可选参数。
 * @param {number} [options.timeout=10000] 超时毫秒数。
 * @param {number} [options.interval=250] 轮询间隔毫秒数。
 * @returns {Promise<boolean>} 文本消失时为 true（超时抛异常）。
 */
export async function waitForGone(text, { timeout = 10000, interval = 250 } = {}) {
  await waitForCondition(
    async () => !hasText(await api.uiTree(), text),
    { timeout, interval, label: `text "${text}" gone` },
  );
  return true;
}

/**
 * 等待可编辑控件出现。
 * @param {string} nameFragment 名称片段。
 * @param {object} [options] 可选参数。
 * @param {number} [options.timeout=10000] 超时毫秒数。
 * @param {number} [options.interval=250] 轮询间隔毫秒数。
 * @returns {Promise<object>} 命中的可编辑节点。
 */
export async function waitForEditable(nameFragment, { timeout = 10000, interval = 250 } = {}) {
  return waitForCondition(
    async () => findEditable(await api.uiTree(), nameFragment),
    { timeout, interval, label: `editable "${nameFragment}"` },
  );
}

/**
 * 等待按钮可见且未禁用。
 * @param {string} text 按钮文本。
 * @param {object} [options] 可选参数。
 * @param {number} [options.timeout=10000] 超时毫秒数。
 * @param {number} [options.interval=250] 轮询间隔毫秒数。
 * @returns {Promise<object>} 命中的按钮节点。
 */
export async function waitForButton(text, { timeout = 10000, interval = 250 } = {}) {
  return waitForCondition(
    async () => {
      const button = findButton(await api.uiTree(), text);
      return button && button.states?.disabled !== true ? button : null;
    },
    { timeout, interval, label: `enabled button "${text}"` },
  );
}

/**
 * 等待对话框出现。
 * @param {string} titleText 对话框标题或内容片段。
 * @param {object} [options] 可选参数。
 * @param {number} [options.timeout=10000] 超时毫秒数。
 * @param {number} [options.interval=250] 轮询间隔毫秒数。
 * @returns {Promise<object>} 命中的对话框节点。
 */
export async function waitForDialog(titleText, { timeout = 10000, interval = 250 } = {}) {
  return waitForCondition(
    async () => findDialog(await api.uiTree(), titleText),
    { timeout, interval, label: `dialog "${titleText}"` },
  );
}

/**
 * 等待对话框消失。
 * @param {string} titleText 对话框标题或内容片段。
 * @param {object} [options] 可选参数。
 * @param {number} [options.timeout=10000] 超时毫秒数。
 * @param {number} [options.interval=250] 轮询间隔毫秒数。
 * @returns {Promise<boolean>} 对话框消失时为 true（超时抛异常）。
 */
export async function waitForDialogGone(titleText, { timeout = 10000, interval = 250 } = {}) {
  await waitForCondition(
    async () => findDialog(await api.uiTree(), titleText) === null,
    { timeout, interval, label: `dialog "${titleText}" gone` },
  );
  return true;
}

// ---------- 基于坐标的点击与输入 ----------

/**
 * 点击给定节点的 bounds 中心。
 * @param {object} node 目标节点（坐标应为当前界面下的新鲜值）。
 * @param {object} [options] 可选参数。
 * @param {number} [options.count=1] 点击次数。
 * @returns {Promise<void>} 无返回值。
 */
export async function clickNode(node, { count = 1 } = {}) {
  const center = boundsCenter(node);
  await api.postInput([api.mouseMove(center.x, center.y), api.mouseClick("left", count)]);
}

/**
 * 等待目标出现、等界面静止后重新定位并点击（避免动画期间坐标漂移）。
 * @param {() => Promise<object|null>} finder 目标查找函数。
 * @param {object} options 可选参数。
 * @param {number} [options.timeout=8000] 等待超时毫秒数。
 * @param {number} [options.settleMs=350] 首次命中后的静止等待毫秒数。
 * @param {number} [options.count=1] 点击次数。
 * @param {string} options.label 等待条件名（用于错误消息）。
 * @returns {Promise<object>} 被点击的节点。
 */
async function clickWithRefind(finder, { timeout = 8000, settleMs = 350, count = 1, label }) {
  let target = await waitForCondition(finder, { timeout, label });
  await sleep(settleMs);
  target = (await finder()) ?? target;
  await clickNode(target, { count });
  return target;
}

/**
 * 查找文本并点击其所在节点。
 * @param {string} text 目标文本。
 * @param {object} [options] 可选参数。
 * @param {number} [options.timeout=8000] 等待超时毫秒数。
 * @param {number} [options.count=1] 点击次数。
 * @param {boolean} [options.exact=false] 是否精确匹配文本。
 * @returns {Promise<object>} 被点击的节点。
 */
export async function clickText(text, { timeout = 8000, count = 1, exact = false } = {}) {
  return clickWithRefind(
    async () => findByText(await api.uiTree(), text, { exact }),
    { timeout, count, label: `clickable text "${text}"` },
  );
}

/**
 * 查找按钮并点击。
 * @param {string} text 按钮文本。
 * @param {object} [options] 可选参数。
 * @param {number} [options.timeout=8000] 等待超时毫秒数。
 * @param {number} [options.count=1] 点击次数。
 * @param {boolean} [options.exact=false] 是否精确匹配子树文本。
 * @returns {Promise<object>} 被点击的按钮节点。
 */
export async function clickButton(text, { timeout = 8000, count = 1, exact = false } = {}) {
  return clickWithRefind(
    async () => findButton(await api.uiTree(), text, { exact }),
    { timeout, count, label: `clickable button "${text}"` },
  );
}

/**
 * 查找可编辑控件、聚焦点击（用于随后输入）。
 * @param {string} nameFragment 名称片段。
 * @param {object} [options] 可选参数。
 * @param {number} [options.timeout=8000] 等待超时毫秒数。
 * @returns {Promise<object>} 被点击的可编辑节点。
 */
export async function clickEditable(nameFragment, { timeout = 8000 } = {}) {
  return clickWithRefind(
    async () => findEditable(await api.uiTree(), nameFragment),
    { timeout, label: `clickable editable "${nameFragment}"` },
  );
}

/**
 * 聚焦可编辑控件并输入文本（默认先全选以覆盖已有内容）。
 * @param {string} nameFragment 名称片段。
 * @param {string} text 要输入的文本。
 * @param {object} [options] 可选参数。
 * @param {boolean} [options.selectAll=true] 是否先 Ctrl+A 全选。
 * @returns {Promise<void>} 无返回值。
 */
export async function typeIntoEditable(nameFragment, text, { selectAll = true } = {}) {
  await clickEditable(nameFragment);
  if (selectAll) {
    await api.postInput([api.keyClick(["Control", "a"])]);
  }
  await api.postInput([api.typeText(text)]);
}

/**
 * 读取当前界面中的全部文本（调试辅助）。
 * @returns {Promise<string[]>} 文本列表。
 */
export async function visibleTexts() {
  const all = flatten(await api.uiTree());
  return all.map(({ node }) => node.text).filter(Boolean);
}
