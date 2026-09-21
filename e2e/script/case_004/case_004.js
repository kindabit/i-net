// case_004 画布节点基础操作。
//
// 流程：复制 base fixture → 启动应用 → 解锁（lastScene 恢复根画布）→ 模板面板展开与面板互斥
// → 拖拽创建数据节点（「新节点」）→ 编辑节点对话框（F1 标题为空；保存标题/副标题；按钮入口改副标题）
// → 复制节点（副本落于视口中部且按 20px 取整）→ 自定义颜色（绿色预设 + 节点区域像素比对）
// → 创建第二个节点并改名「节点乙」→ 框选三节点 → 批量移动与 20px 网格吸附（两阶段实验）
// → F4 Delete/Backspace 不删除节点 → 滚轮缩放 → 视口持久化（进入画布宇宙再返回）
// → 逻辑删除与回收站（徽标、恢复、F2 永久删除取消、永久删除、重建后清空回收站）→ 输出 PASS/FAIL 报告。
//
// 脚本规范要点：
// - 断言优先使用 UI 树（文本、bounds、states、role/name），截图用于视觉留档；配色效果辅以
//   截图像素比对；配色对话框的色块以 role=button + aria-label 入树、hover 后的 tooltip 以
//   role=tooltip 节点入树（2026-09-21 修复后均基于 UI 树定位与断言）；
// - 预构建调试可执行文件的前端为生产构建（DEBUG=false），NodeDebugOverlay/ViewportDebugOverlay 已被
//   条件编译剔除；20px 网格对齐改以物理像素验证——副本落点经「吸附拖动位移恰为 20 画布像素」反证，
//   批量移动断言「物理位移一致且为 20 画布像素的整数倍」，画布像素与物理像素按卡片宽度换算 DPI；
// - vue-flow 多选期间 nodesselection 叠层会把选中节点从 UI 树中遮挡过滤，因此断言批量位移前先点击
//   空白取消选中，再读取坐标与 bounds；
// - hover 操作按钮在鼠标离开卡片后会以离场动画短暂留在 DOM：定位按钮时须校验其 bounds 位于目标
//   卡片正上方，避免误点上一个节点的同名按钮；
// - 拖动命令分阶段移动并在松开前等待约 400ms，规避输入注入事件合并导致最后一次 mousemove 丢失；
// - 应用生命周期由 withApp 包装，保证无论成败都经 POST /shutdown 收尾；
// - 关键步骤截图存 output；流程中断时额外截取 fatal 截图并写入 report.json。
//
// 运行方式：在项目根目录执行 `node e2e\script\case_004\case_004.js`

import path from "node:path";
import { fileURLToPath } from "node:url";
import * as api from "../lib/api.js";
import { withApp, prepareCaseOutput } from "../lib/app.js";
import { prepareDataDir } from "../lib/fixtures.js";
import * as image from "../lib/image.js";
import { Report, finalizeReport } from "../lib/report.js";
import * as ui from "../lib/ui.js";
import { sleep, waitForCondition } from "../lib/util.js";

const CASE_DIR = path.dirname(fileURLToPath(import.meta.url));
const OUTPUT_DIR = path.join(CASE_DIR, "output");
const DATA_DIR = path.join(OUTPUT_DIR, "data");

/** fixture 数据库名称与密码（见 _fixtures\README.md） */
const DB_NAME = "zz-e2e-base";
const DB_PASSWORD = "e2e-password";

/** 第一个数据节点的落点（节点左上角，窗口物理像素），位于 fixture 节点左下方空白区 */
const NODE_A_DROP = { x: 260, y: 620 };
/** 第二个数据节点的落点（窗口物理像素），位于第一个节点下方空白区 */
const NODE_B_DROP = { x: 260, y: 830 };
/** 点击空白处取消选中/关闭面板的坐标（画布左上空白区，窗口物理像素） */
const BLANK_POINT = { x: 200, y: 300 };
/** 悬停前的清场坐标（画布右上空白区，窗口物理像素） */
const NEUTRAL_POINT = { x: 1200, y: 300 };

/** 绿色预设的亮色/暗色背景字面量（见 src\node-colors\color-presets.ts） */
const PRESET_GREEN_LIGHT = "#e8f5e9ff";
const PRESET_GREEN_DARK = "#1b5e20ff";
/** 浅色主题下绿色预设背景 #e8f5e9 的 RGB 分量 */
const PRESET_GREEN_LIGHT_RGB = { r: 232, g: 245, b: 233 };
/** 数据节点配色对话框预设组合的色块可访问名称（即色块 tooltip 文案，顺序与预设列表一致） */
const PRESET_SWATCH_NAMES = ["蓝", "绿", "紫", "橙", "青", "粉", "灰"];
/** 数据节点配色对话框的字段名（亮色/暗色两栏相同，顺序与字段定义表一致） */
const NODE_FIELD_NAMES = [
  "背景",
  "边框",
  "选中边框",
  "标题",
  "副标题",
  "图标",
  "连接点",
  "工具按钮",
];
/** 亮色主题字段色块的可访问名称（「亮色主题 + 字段名」） */
const LIGHT_SWATCH_NAMES = NODE_FIELD_NAMES.map((name) => `亮色主题 ${name}`);
/** 暗色主题字段色块的可访问名称（「暗色主题 + 字段名」） */
const DARK_SWATCH_NAMES = NODE_FIELD_NAMES.map((name) => `暗色主题 ${name}`);

const report = new Report("case_004 画布节点基础操作");

let shotIndex = 0;

/**
 * 截取当前界面并保存到 output 目录（带自增编号）。
 * @param {string} label 截图标签。
 * @returns {Promise<string>} 截图文件路径。
 */
async function snap(label) {
  shotIndex += 1;
  const file = path.join(OUTPUT_DIR, `${String(shotIndex).padStart(2, "0")}-${label}.png`);
  await api.saveScreenshot(file);
  console.log(`  (screenshot) ${file}`);
  return file;
}

/**
 * 等待首页就绪并确保界面语言为中文：首页未出现中文名称输入框时，
 * 通过右上角「切换语言」菜单切回「简体中文」。
 * @returns {Promise<boolean>} 是否执行了语言切换。
 */
async function ensureChineseUi() {
  const initial = await waitForCondition(
    async () => {
      const tree = await api.uiTree();
      return ui.findEditable(tree, "数据库名称") ?? ui.findEditable(tree, "Database Name")
        ? tree
        : null;
    },
    { timeout: 20000, label: "home page name input" },
  );
  if (ui.findEditable(initial, "数据库名称")) {
    return false;
  }
  console.log("  (ui language is not Chinese; switching back via the language menu)");
  const button =
    ui.findButton(initial, "切换语言") ?? ui.findButton(initial, "Switch Language");
  if (!button) {
    throw new Error("language switch button not found on home page");
  }
  await ui.clickNode(button);
  await ui.clickText("简体中文");
  await ui.waitForEditable("数据库名称", { timeout: 10000 });
  return true;
}

/**
 * 解锁 fixture 数据库：名称步选择候选并确认，密码步输入密码确认，等待进入根画布。
 * @returns {Promise<void>} 无返回值。
 */
async function unlock() {
  await ui.clickEditable("数据库名称");
  await api.postInput([api.typeText(DB_NAME)]);
  await ui.waitForTextStable(DB_NAME, { timeout: 8000 });
  await api.postInput([api.keyClick(["Enter"])]);
  await ui.clickButton("确认");
  await ui.waitForEditable("密码", { timeout: 20000 });
  await ui.typeIntoEditable("密码", DB_PASSWORD);
  await ui.clickButton("确认");
  await ui.waitForTextStable("账号 A", { timeout: 40000, settleMs: 800 });
}

/**
 * 返回 UI 树中文本精确匹配的 #text 节点，按 left 升序排序。
 * @param {object[]} tree UI 树顶层节点数组。
 * @param {string} text 目标文本。
 * @returns {object[]} 命中的文本节点数组。
 */
function textNodes(tree, text) {
  return ui
    .flatten(tree)
    .filter(({ node }) => node.tag === "#text" && (node.text ?? "").trim() === text)
    .map(({ node }) => node)
    .sort((a, b) => a.bounds.left - b.bounds.left);
}

/**
 * 返回 UI 树中全部 vue-flow 节点卡片（role=group 且带 data-id 的容器）。
 * @param {object[]} tree UI 树顶层节点数组。
 * @returns {object[]} 节点卡片组数组。
 */
function cardGroups(tree) {
  return ui
    .flatten(tree)
    .filter(({ node }) => node.role === "group" && node.attrs?.["data-id"])
    .map(({ node }) => node);
}

/**
 * 查找指定标题的第 index 个节点卡片（按标题文本 left 升序取序，并取包含该文本的最近卡片）。
 * @param {object[]} tree UI 树顶层节点数组。
 * @param {string} text 节点标题文本。
 * @param {number} [index=0] 同名节点序号（按 left 升序）。
 * @returns {object|null} 命中的节点卡片；未命中时为 null。
 */
function cardOf(tree, text, index = 0) {
  const titles = textNodes(tree, text);
  if (titles.length <= index) {
    return null;
  }
  const title = titles[index];
  const candidates = cardGroups(tree).filter((group) =>
    ui
      .flatten([group])
      .some(({ node }) => node.tag === "#text" && (node.text ?? "").trim() === text),
  );
  candidates.sort(
    (a, b) =>
      Math.hypot(a.bounds.left - title.bounds.left, a.bounds.top - title.bounds.top) -
      Math.hypot(b.bounds.left - title.bounds.left, b.bounds.top - title.bounds.top),
  );
  return candidates[0] ?? null;
}

/**
 * 在 UI 树中查找可访问名称精确匹配的按钮（含 role=button 的配色色块）。
 * @param {object[]} tree UI 树顶层节点数组。
 * @param {string} name 目标可访问名称（aria-label）。
 * @returns {object|null} 命中节点；未命中时为 null。
 */
function findButtonByName(tree, name) {
  return (
    ui
      .findByRole(tree, "button")
      .find((node) => node.name === name && node.bounds && node.bounds.width > 0) ?? null
  );
}

/**
 * 读取 UI 树中全部 tooltip 节点（role=tooltip）的显示文本。
 * @param {object[]} tree UI 树顶层节点数组。
 * @returns {string[]} tooltip 文本列表（已去除首尾空白，过滤空串）。
 */
function tooltipTexts(tree) {
  return ui
    .findByRole(tree, "tooltip")
    .map((node) => ui.subtreeText(node).trim())
    .filter((text) => text !== "");
}

/**
 * 计算完全包住指定节点的框选矩形（物理像素），并校验不会完整框住其它节点。
 * vue-flow 默认 SelectionMode.Full：只有被矩形完全包含的节点才会被选中。
 * @param {object[]} tree UI 树顶层节点数组。
 * @param {string[]} texts 目标节点标题（可重复，重复文本按出现顺序取序）。
 * @param {number} margin 四周留白（物理像素）。
 * @returns {{left: number, top: number, width: number, height: number}} 框选矩形。
 */
function computeBox(tree, texts, margin) {
  const counts = new Map();
  const cards = [];
  for (const text of texts) {
    const index = counts.get(text) ?? 0;
    counts.set(text, index + 1);
    const card = cardOf(tree, text, index);
    if (!card) {
      throw new Error(`computeBox: card "${text}"[${index}] not found`);
    }
    cards.push(card);
  }
  const left = Math.min(...cards.map((c) => c.bounds.left)) - margin;
  const top = Math.min(...cards.map((c) => c.bounds.top)) - margin;
  const right = Math.max(...cards.map((c) => c.bounds.left + c.bounds.width)) + margin;
  const bottom = Math.max(...cards.map((c) => c.bounds.top + c.bounds.height)) + margin;
  const targetIds = new Set(cards.map((c) => c.attrs["data-id"]));
  const others = cardGroups(tree).filter((group) => {
    if (targetIds.has(group.attrs["data-id"])) {
      return false;
    }
    const hasText = ui.flatten([group]).some(({ node }) => node.tag === "#text");
    if (!hasText) {
      return false;
    }
    const b = group.bounds;
    return b.left >= left && b.left + b.width <= right && b.top >= top && b.top + b.height <= bottom;
  });
  if (others.length > 0) {
    throw new Error(
      `computeBox: box ${JSON.stringify({ left, top, right, bottom })} fully contains other nodes: ${JSON.stringify(
        others.map((o) => o.bounds),
      )}`,
    );
  }
  return { left, top, width: right - left, height: bottom - top };
}

/**
 * 由节点卡片宽度推算窗口的 DPI 缩放（卡片固定 160 画布像素宽，见 src\node-size.ts）。
 * @param {object} card 节点卡片节点（bounds 为窗口物理像素）。
 * @returns {number} DPI 缩放比例（物理像素 / 画布像素）。
 */
function dpiOfCard(card) {
  return card.bounds.width / 160;
}

/**
 * 判断两个物理坐标是否同余于 20 画布像素对应的物理长度（容差 1 物理像素）。
 * 20px 网格吸附后的节点坐标（画布坐标）为 20 的整数倍，折算到物理像素后两两同余。
 * @param {number} a 物理坐标（如卡片 left）。
 * @param {number} b 物理坐标（如卡片 left）。
 * @param {number} dpi DPI 缩放比例。
 * @returns {boolean} 同余于同一网格线时为 true。
 */
function sameGridLine(a, b, dpi) {
  const mod = 20 * dpi;
  const rem = Math.abs(a - b) % mod;
  return rem <= 1 || mod - rem <= 1;
}

/**
 * 悬停指定节点并点击其操作按钮。
 *
 * 先把鼠标移到空白处清掉其它节点残留的悬浮按钮离场动画，再移到目标节点标题中心；
 * 按钮定位时校验按钮 bounds 位于目标卡片正上方，点击前重新读取并定位，避免误点相邻节点按钮。
 * @param {string} text 节点标题文本。
 * @param {string} actionTitle 操作按钮的 title（如「编辑节点」）。
 * @param {number} [index=0] 同名节点序号（按 left 升序）。
 * @returns {Promise<void>} 无返回值。
 */
async function hoverNodeAction(text, actionTitle, index = 0) {
  await api.postInput([api.mouseMove(NEUTRAL_POINT.x, NEUTRAL_POINT.y)]);
  await waitForCondition(async () => ui.findAttr(await api.uiTree(), "title", actionTitle) === null, {
    timeout: 4000,
    interval: 150,
    label: `stale "${actionTitle}" cleared`,
  }).catch(() => {});
  const tree = await api.uiTree();
  const title = textNodes(tree, text)[index];
  const card = cardOf(tree, text, index);
  if (!title || !card) {
    throw new Error(`hoverNodeAction: node "${text}"[${index}] not found`);
  }
  const center = ui.boundsCenter(title);
  await api.postInput([api.mouseMove(center.x, center.y)]);
  const findNearCard = (t, c) =>
    ui
      .flatten(t)
      .map(({ node }) => node)
      .find(
        (n) =>
          n.attrs?.title === actionTitle &&
          n.bounds.left + n.bounds.width >= c.bounds.left - 40 &&
          n.bounds.left <= c.bounds.left + c.bounds.width + 40 &&
          n.bounds.top >= c.bounds.top - 130 &&
          n.bounds.top + n.bounds.height <= c.bounds.top + 10,
      ) ?? null;
  await waitForCondition(
    async () => {
      const t = await api.uiTree();
      const c = cardOf(t, text, index);
      return c ? findNearCard(t, c) : null;
    },
    { timeout: 6000, interval: 200, label: `button "${actionTitle}" near "${text}"` },
  );
  await sleep(300);
  const fresh = await api.uiTree();
  const button = findNearCard(fresh, cardOf(fresh, text, index));
  if (!button) {
    throw new Error(`hoverNodeAction: button "${actionTitle}" lost for "${text}"[${index}]`);
  }
  await ui.clickNode(button);
}

/**
 * 在指定矩形上按下左键拖动执行框选（起点与终点为窗口物理像素），松开前等待以稳定选中状态。
 * @param {{left: number, top: number, width: number, height: number}} box 框选矩形。
 * @returns {Promise<void>} 无返回值。
 */
async function boxSelect(box) {
  await api.postInput([
    api.mouseMove(box.left, box.top),
    api.mousePress("left"),
    api.mouseMove(box.left + box.width, box.top + box.height),
    api.wait(300),
    api.mouseRelease("left"),
  ]);
  await sleep(600);
}

/**
 * 点击画布空白处取消选中（选中叠层会遮挡节点，读取 UI 树前必须先取消选中）。
 * @returns {Promise<void>} 无返回值。
 */
async function deselect() {
  await api.postInput([api.mouseMove(BLANK_POINT.x, BLANK_POINT.y), api.mouseClick("left", 1)]);
  await sleep(700);
}

/**
 * 从起点水平拖动指定距离（窗口物理像素）：分阶段移动并在最后阶段后等待再松开，
 * 规避输入注入事件合并导致最后一次 mousemove 丢失、位移偏小的问题。
 * @param {{x: number, y: number}} start 起点（窗口物理像素）。
 * @param {number} deltaX 水平位移（窗口物理像素，可为负）。
 * @returns {Promise<void>} 无返回值。
 */
async function dragHorizontally(start, deltaX) {
  await api.postInput([
    api.mouseMove(start.x, start.y),
    api.mousePress("left"),
    api.mouseMove(start.x + deltaX * 0.4, start.y),
    api.wait(150),
    api.mouseMove(start.x + deltaX * 0.75, start.y),
    api.wait(150),
    api.mouseMove(start.x + deltaX, start.y),
    api.wait(400),
    api.mouseRelease("left"),
  ]);
}

/**
 * 通过模板面板拖拽创建一个数据节点（标题「新节点」），并等待面板自动关闭。
 * @param {{x: number, y: number}} drop 落点（节点左上角，窗口物理像素）。
 * @returns {Promise<void>} 无返回值。
 */
async function dragCreateDataNode(drop) {
  await ui.clickNode(ui.findAttr(await api.uiTree(), "title", "新建节点"));
  await waitForCondition(async () => ui.findAttr(await api.uiTree(), "title", "拖拽创建数据节点"), {
    timeout: 8000,
    label: "模板面板展开",
  });
  await sleep(400);
  const handle = ui.findAttr(await api.uiTree(), "title", "拖拽创建数据节点");
  await api.postInput([api.mouseDrag(ui.boundsCenter(handle), drop)]);
  await waitForCondition(async () => textNodes(await api.uiTree(), "新节点").length >= 1, {
    timeout: 10000,
    label: "数据节点创建",
  });
  await waitForCondition(
    async () => ui.findAttr(await api.uiTree(), "title", "拖拽创建数据节点") === null,
    { timeout: 6000, interval: 200, label: "模板面板自动关闭" },
  );
  await sleep(700);
}

/**
 * 打开指定节点（按「新节点」序）的编辑对话框并把标题改为指定名称。
 * @param {number} index 同名节点序号（按 left 升序）。
 * @param {string} newTitle 新标题。
 * @returns {Promise<void>} 无返回值。
 */
async function renameNewNode(index, newTitle) {
  await ui.clickText("新节点", { count: 2 });
  await ui.waitForDialog("编辑节点", { timeout: 8000 });
  await sleep(500);
  await ui.typeIntoEditable("标题", newTitle);
  const dialog = ui.findDialog(await api.uiTree(), "编辑节点");
  await ui.clickNode(ui.findButton(await api.uiTree(), "确认", { root: dialog }));
  await ui.waitForDialogGone("编辑节点", { timeout: 8000 });
  await sleep(700);
}

/**
 * 确保画布回收站面板处于打开状态（已打开时不重复点击，避免 toggle 关闭）。
 * @returns {Promise<void>} 无返回值。
 */
async function ensureRecycleBinPanel() {
  const tree = await api.uiTree();
  if (!ui.findButton(tree, "清空回收站")) {
    const button = await waitForCondition(
      async () => ui.findAttr(await api.uiTree(), "title", "回收站"),
      { timeout: 8000, interval: 200, label: "「回收站」按钮可见" },
    );
    await ui.clickNode(button);
    await waitForCondition(
      async () => ui.findButton(await api.uiTree(), "清空回收站"),
      { timeout: 6000, interval: 200, label: "回收站面板打开" },
    );
    await sleep(400);
  }
}

/**
 * 读取回收站徽标（role=status 的 VBadge 徽章）中的数字文本。
 * @param {object[]} tree UI 树顶层节点数组。
 * @returns {string[]} 数字文本列表；无徽标时为空数组。
 */
function badgeTexts(tree) {
  return ui
    .findByRole(tree, "status")
    .map((node) =>
      ui
        .flatten([node])
        .filter((entry) => entry.node.tag === "#text")
        .map((entry) => entry.node.text ?? "")
        .join(""),
    )
    .filter((text) => /^\d+$/.test(text.trim()));
}

/**
 * 统计截图中指定区域内出现次数最多的颜色（RGB 众数）。
 * @param {import("pngjs").PNG} png 整窗截图 PNG。
 * @param {{left: number, top: number, width: number, height: number}} region 统计区域。
 * @returns {{r: number, g: number, b: number, count: number}|null} 众数颜色；区域无效时为 null。
 */
function dominantColor(png, region) {
  const cropped = image.cropPng(png, region);
  const counts = new Map();
  for (let i = 0; i < cropped.data.length; i += 4) {
    const key = (cropped.data[i] << 16) | (cropped.data[i + 1] << 8) | cropped.data[i + 2];
    counts.set(key, (counts.get(key) ?? 0) + 1);
  }
  let best = -1;
  let bestCount = -1;
  for (const [key, count] of counts) {
    if (count > bestCount) {
      best = key;
      bestCount = count;
    }
  }
  if (best < 0) {
    return null;
  }
  return { r: (best >> 16) & 255, g: (best >> 8) & 255, b: best & 255, count: bestCount };
}

/**
 * 判断颜色是否接近给定 RGB（各分量差值不超过 8）。
 * @param {{r: number, g: number, b: number}|null} color 实测颜色。
 * @param {{r: number, g: number, b: number}} expected 期望颜色。
 * @returns {boolean} 接近时为 true。
 */
function colorNear(color, expected) {
  if (!color) {
    return false;
  }
  return (
    Math.abs(color.r - expected.r) <= 8 &&
    Math.abs(color.g - expected.g) <= 8 &&
    Math.abs(color.b - expected.b) <= 8
  );
}

/**
 * 读取 PNG 中指定物理像素点的颜色分量。
 * @param {import("pngjs").PNG} png 整窗截图 PNG。
 * @param {number} x 横坐标（窗口物理像素）。
 * @param {number} y 纵坐标（窗口物理像素）。
 * @returns {{r: number, g: number, b: number, a: number}} 像素分量。
 */
function pixelAt(png, x, y) {
  const idx = (png.width * y + x) << 2;
  return { r: png.data[idx], g: png.data[idx + 1], b: png.data[idx + 2], a: png.data[idx + 3] };
}

/**
 * 判断像素是否为节点卡片的选中边框色（主题 primary 蓝色系）。
 * @param {{r: number, g: number, b: number}} pixel 像素分量。
 * @returns {boolean} 蓝色系时为 true。
 */
function isSelectionBorder(pixel) {
  return pixel.b - pixel.r >= 80 && pixel.b >= 150;
}

/**
 * 用例主流程。
 * @returns {Promise<void>} 无返回值。
 */
async function main() {
  report.section("S0 前置与解锁（前置条件）");
  const info = await api.health();
  report.check(info.width > 0 && info.height > 0, "调试自动化服务可用", `${info.width}x${info.height}`);
  await ensureChineseUi();
  await unlock();
  const unlockedTree = await api.uiTree();
  report.check(ui.hasText(unlockedTree, "账号 A"), "解锁后进入根画布并出现节点「账号 A」");
  report.check(ui.hasText(unlockedTree, "备注 B"), "解锁后出现节点「备注 B」");
  await sleep(600);
  await snap("unlocked-root-canvas");

  report.section("S1 悬浮菜单与模板面板（步骤 1）");
  const menuTree = await api.uiTree();
  report.check(ui.findAttr(menuTree, "title", "新建节点") !== null, "步骤 1 悬浮菜单存在按钮「新建节点」");
  report.check(ui.findAttr(menuTree, "title", "回收站") !== null, "步骤 1 悬浮菜单存在按钮「回收站」");
  report.check(ui.findAttr(menuTree, "title", "字典管理") !== null, "步骤 1 悬浮菜单存在按钮「字典管理」");
  report.check(ui.findAttr(menuTree, "title", "自动布局") !== null, "步骤 1 悬浮菜单存在按钮「自动布局」");
  await ui.clickNode(ui.findAttr(menuTree, "title", "新建节点"));
  await waitForCondition(async () => ui.findAttr(await api.uiTree(), "title", "拖拽创建数据节点"), {
    timeout: 8000,
    label: "模板面板展开",
  });
  await sleep(400);
  const panelTree = await api.uiTree();
  report.check(
    ui.findAttr(panelTree, "title", "拖拽创建数据节点") !== null,
    "步骤 1 模板面板出现手柄「拖拽创建数据节点」",
  );
  report.check(
    ui.findAttr(panelTree, "title", "拖拽创建画布数据节点") !== null,
    "步骤 1 模板面板出现手柄「拖拽创建画布数据节点」",
  );
  await snap("step1-template-panel");

  // 面板互斥：打开回收站面板 → 模板面板关闭；再次点击关闭回收站面板
  await ui.clickNode(ui.findAttr(await api.uiTree(), "title", "回收站"));
  await waitForCondition(async () => ui.findButton(await api.uiTree(), "清空回收站"), {
    timeout: 6000,
    label: "回收站面板打开",
  });
  const exclusiveTree = await api.uiTree();
  report.check(
    ui.findAttr(exclusiveTree, "title", "拖拽创建数据节点") === null,
    "步骤 1 打开回收站面板后模板面板关闭（两面板互斥）",
  );
  await snap("step1b-panel-exclusive");
  await ui.clickNode(ui.findAttr(await api.uiTree(), "title", "回收站"));
  await waitForCondition(async () => !ui.findButton(await api.uiTree(), "清空回收站"), {
    timeout: 6000,
    label: "回收站面板关闭",
  });
  await ui.clickNode(ui.findAttr(await api.uiTree(), "title", "新建节点"));
  await waitForCondition(async () => ui.findAttr(await api.uiTree(), "title", "拖拽创建数据节点"), {
    timeout: 8000,
    label: "模板面板重新展开",
  });
  await sleep(400);
  report.check(true, "步骤 1 模板面板可再次展开（收起/展开切换正常）");

  report.section("S2 拖拽创建数据节点（步骤 2）");
  const handle = ui.findAttr(await api.uiTree(), "title", "拖拽创建数据节点");
  await api.postInput([api.mouseDrag(ui.boundsCenter(handle), NODE_A_DROP)]);
  await ui.waitForText("新节点", { timeout: 10000 });
  const panelClosed = await waitForCondition(
    async () => ui.findAttr(await api.uiTree(), "title", "拖拽创建数据节点") === null,
    { timeout: 6000, interval: 200, label: "模板面板自动关闭" },
  )
    .then(() => true)
    .catch(() => false);
  report.check(panelClosed, "步骤 2 创建成功后模板面板自动关闭");
  await sleep(500);
  const createdTree = await api.uiTree();
  const createdCard = cardOf(createdTree, "新节点");
  report.check(createdCard !== null, "步骤 2 画布出现节点「新节点」", JSON.stringify(createdCard?.bounds));
  await snap("step2-node-created");

  report.section("S3 编辑节点对话框（步骤 3）");
  await ui.clickText("新节点", { count: 2 });
  await ui.waitForDialog("编辑节点", { timeout: 8000 });
  await sleep(600);
  const editTree = await api.uiTree();
  report.check(ui.findDialog(editTree, "编辑节点") !== null, "步骤 3 双击卡片打开「编辑节点」对话框");
  report.check(ui.findEditable(editTree, "标题") !== null, "步骤 3 对话框包含「标题」输入框");
  report.check(ui.findEditable(editTree, "副标题") !== null, "步骤 3 对话框包含「副标题」输入框");
  await snap("step3-edit-dialog");

  report.section("S4 标题为空失败路径（步骤 4、F1）");
  await ui.clickEditable("标题");
  await api.postInput([api.keyClick(["Control", "a"]), api.keyClick(["Delete"])]);
  await sleep(300);
  const emptyDialog = ui.findDialog(await api.uiTree(), "编辑节点");
  await ui.clickNode(ui.findButton(await api.uiTree(), "确认", { root: emptyDialog }));
  const emptyError = await ui.waitForTextStable("标题不能为空", { timeout: 6000 }).catch(() => null);
  report.check(emptyError !== null, "F1 标题置空提交提示「标题不能为空」");
  report.check(ui.findDialog(await api.uiTree(), "编辑节点") !== null, "F1 对话框保持打开，未提交");
  await snap("f1-title-empty");

  report.section("S5 标题与副标题保存（步骤 5）");
  await ui.typeIntoEditable("标题", "节点甲");
  await ui.typeIntoEditable("副标题", "备注甲");
  const saveDialog = ui.findDialog(await api.uiTree(), "编辑节点");
  await ui.clickNode(ui.findButton(await api.uiTree(), "确认", { root: saveDialog }));
  await ui.waitForDialogGone("编辑节点", { timeout: 8000 });
  await sleep(600);
  const renamedTree = await api.uiTree();
  report.check(ui.hasText(renamedTree, "节点甲"), "步骤 5 卡片显示「节点甲」");
  report.check(ui.hasText(renamedTree, "备注甲"), "步骤 5 卡片显示「备注甲」");
  report.check(textNodes(renamedTree, "新节点").length === 0, "步骤 5 旧标题「新节点」消失");
  await snap("step5-renamed");

  report.section("S6 按钮入口编辑副标题（步骤 6）");
  await hoverNodeAction("节点甲", "编辑节点");
  await ui.waitForDialog("编辑节点", { timeout: 8000 });
  await sleep(500);
  await ui.typeIntoEditable("副标题", "备注甲改");
  const subDialog = ui.findDialog(await api.uiTree(), "编辑节点");
  await ui.clickNode(ui.findButton(await api.uiTree(), "确认", { root: subDialog }));
  await ui.waitForDialogGone("编辑节点", { timeout: 8000 });
  await sleep(500);
  const subTree = await api.uiTree();
  report.check(ui.hasText(subTree, "备注甲改"), "步骤 6 hover 按钮打开编辑对话框并更新副标题为「备注甲改」");
  report.check(textNodes(subTree, "备注甲").length === 0, "步骤 6 旧副标题「备注甲」消失");
  await snap("step6-subtitle-updated");

  report.section("S7 复制节点（步骤 7）");
  const beforeCopyTree = await api.uiTree();
  report.check(textNodes(beforeCopyTree, "节点甲").length === 1, "步骤 7 复制前画布仅一个「节点甲」");
  await hoverNodeAction("节点甲", "复制节点");
  await waitForCondition(async () => textNodes(await api.uiTree(), "节点甲").length >= 2, {
    timeout: 8000,
    label: "副本出现",
  });
  await sleep(700);
  const copiedTree = await api.uiTree();
  const copyCard = cardOf(copiedTree, "节点甲", 1);
  report.check(textNodes(copiedTree, "节点甲").length === 2, "步骤 7 画布出现第二个「节点甲」卡片");
  const copyCenterX = copyCard.bounds.left + copyCard.bounds.width / 2;
  const copyCenterY = copyCard.bounds.top + copyCard.bounds.height / 2;
  report.check(
    Math.abs(copyCenterX - info.width / 2) <= 200 && Math.abs(copyCenterY - info.height / 2) <= 250,
    "步骤 7 副本位于视口中部区域",
    `center=(${copyCenterX},${copyCenterY}) window=(${info.width / 2},${info.height / 2})`,
  );
  // 副本落点按 20px 取整：对副本做一次已知水平拖动，吸附后的物理位移应恰为 20 画布像素
  // （若副本落点未取整，吸附位移会带出落点相对网格的偏移，不再是 20 画布像素的整数倍）。
  const copyDpi = dpiOfCard(copyCard);
  const copyStart = ui.boundsCenter(textNodes(copiedTree, "节点甲")[1]);
  await dragHorizontally(copyStart, 37);
  await sleep(600);
  await deselect();
  const probedTree = await api.uiTree();
  const probedCopyCard = cardOf(probedTree, "节点甲", 1);
  const probeDelta = probedCopyCard.bounds.left - copyCard.bounds.left;
  report.check(
    Math.abs(probeDelta - 20 * copyDpi) <= 1,
    "步骤 7 副本画布坐标按 20px 取整（吸附拖动位移恰为 20 画布像素）",
    `物理位移=${probeDelta}px，20 画布像素=${20 * copyDpi}px`,
  );
  await snap("step7-copied");

  report.section("S8 自定义颜色与预设（步骤 8-9）");
  await hoverNodeAction("节点甲", "自定义颜色", 0);
  await ui.waitForDialog("自定义数据节点颜色", { timeout: 8000 });
  await sleep(700);
  const colorTree = await api.uiTree();
  const colorDialog = ui.findDialog(colorTree, "自定义数据节点颜色");
  report.check(colorDialog !== null, "步骤 8 打开「自定义数据节点颜色」对话框");
  report.check(ui.hasText(colorTree, "预设组合"), "步骤 8 对话框包含「预设组合」区块");
  report.check(
    ui.findButton(colorTree, "保存", { root: colorDialog }) !== null,
    "步骤 8 对话框包含「保存」按钮",
  );
  report.check(
    ui.findButton(colorTree, "取消", { root: colorDialog }) !== null,
    "步骤 8 对话框包含「取消」按钮",
  );
  // 色块以 role=button + name（aria-label）入树：预设组合（tooltip 文案）与亮/暗字段色块
  const presetHit = PRESET_SWATCH_NAMES.filter((name) => findButtonByName(colorTree, name) !== null);
  report.check(
    presetHit.length === PRESET_SWATCH_NAMES.length,
    "步骤 8 预设组合 7 个色块以 role=button + name 入树",
    presetHit.join("/"),
  );
  const lightHit = LIGHT_SWATCH_NAMES.filter((name) => findButtonByName(colorTree, name) !== null);
  report.check(
    lightHit.length === LIGHT_SWATCH_NAMES.length,
    "步骤 8 亮色主题 8 个字段色块以 role=button + name 入树",
    lightHit.join("/"),
  );
  const darkHit = DARK_SWATCH_NAMES.filter((name) => findButtonByName(colorTree, name) !== null);
  report.check(
    darkHit.length === DARK_SWATCH_NAMES.length,
    "步骤 8 暗色主题 8 个字段色块以 role=button + name 入树",
    darkHit.join("/"),
  );
  await snap("step8-color-dialog");

  // hover 预设「绿」色块：UI 树出现 role=tooltip 节点且文本为「绿」
  const greenSwatch = findButtonByName(colorTree, "绿");
  if (!greenSwatch) {
    throw new Error("预设「绿」色块未在 UI 树中");
  }
  const greenCenter = ui.boundsCenter(greenSwatch);
  await api.postInput([api.mouseMove(greenCenter.x, greenCenter.y)]);
  const tooltipShown = await waitForCondition(
    async () => (tooltipTexts(await api.uiTree()).includes("绿") ? true : null),
    { timeout: 6000, interval: 200, label: "预设「绿」tooltip 出现" },
  )
    .then(() => true)
    .catch(() => false);
  report.check(
    tooltipShown,
    "步骤 8 hover「绿」色块后 UI 树出现 role=tooltip 节点且文本为「绿」",
  );
  await snap("step9-swatch-hover");

  // 点击「绿」色块应用预设
  await api.postInput([api.mouseClick("left", 1)]);
  const presetApplied = await waitForCondition(
    async () => {
      const t = await api.uiTree();
      return ui.hasText(t, PRESET_GREEN_LIGHT) && ui.hasText(t, PRESET_GREEN_DARK);
    },
    { timeout: 5000, interval: 150, label: "绿色预设色值回填" },
  )
    .then(() => true)
    .catch(() => false);
  report.check(
    presetApplied,
    "步骤 9 点击第 2 个色块后对话框背景字段值变为绿色预设",
    `${PRESET_GREEN_LIGHT} / ${PRESET_GREEN_DARK}`,
  );

  // 鼠标移开后 tooltip 节点从 UI 树消失
  await api.postInput([api.mouseMove(NEUTRAL_POINT.x, NEUTRAL_POINT.y)]);
  const tooltipGone = await waitForCondition(
    async () => (tooltipTexts(await api.uiTree()).includes("绿") ? null : true),
    { timeout: 6000, interval: 200, label: "「绿」tooltip 消失" },
  )
    .then(() => true)
    .catch(() => false);
  report.check(tooltipGone, "步骤 9 鼠标移开后「绿」tooltip 节点从 UI 树消失");

  const colorSaveDialog = ui.findDialog(await api.uiTree(), "自定义数据节点颜色");
  await ui.clickNode(ui.findButton(await api.uiTree(), "保存", { root: colorSaveDialog }));
  await ui.waitForDialogGone("自定义数据节点颜色", { timeout: 8000 });
  await sleep(700);
  await api.postInput([api.mouseMove(NEUTRAL_POINT.x, NEUTRAL_POINT.y)]);
  await sleep(600);
  const coloredTree = await api.uiTree();
  const coloredCard = cardOf(coloredTree, "节点甲", 0);
  const coloredRegion = {
    left: coloredCard.bounds.left - 10,
    top: coloredCard.bounds.top - 10,
    width: coloredCard.bounds.width + 20,
    height: coloredCard.bounds.height + 20,
  };
  const domColor = dominantColor(image.decodePng(await api.screenshot()), coloredRegion);
  report.check(
    colorNear(domColor, PRESET_GREEN_LIGHT_RGB),
    "步骤 9 保存后左侧「节点甲」背景呈现绿色预设（区域主色比对）",
    domColor ? `rgb(${domColor.r},${domColor.g},${domColor.b})` : "no-color",
  );
  await snap("step9-colored");

  report.section("S9 创建第二个节点并改名（步骤 10）");
  await api.postInput([api.mouseMove(NEUTRAL_POINT.x, NEUTRAL_POINT.y)]);
  await sleep(300);
  await dragCreateDataNode(NODE_B_DROP);
  report.check(textNodes(await api.uiTree(), "新节点").length === 1, "步骤 10 画布出现第二个「新节点」");
  await renameNewNode(0, "节点乙");
  const threeTree = await api.uiTree();
  report.check(textNodes(threeTree, "节点甲").length === 2, "步骤 10 画布存在 2 个「节点甲」");
  report.check(textNodes(threeTree, "节点乙").length === 1, "步骤 10 画布存在「节点乙」");
  await snap("step10-three-nodes");

  report.section("S10 框选三节点（步骤 11）");
  const fixtureBefore = await api.uiTree();
  const accountBefore = textNodes(fixtureBefore, "账号 A")[0]?.bounds;
  const noteBefore = textNodes(fixtureBefore, "备注 B")[0]?.bounds;
  const box1 = computeBox(fixtureBefore, ["节点甲", "节点甲", "节点乙"], 30);
  const selectedCards = [
    cardOf(fixtureBefore, "节点甲", 0),
    cardOf(fixtureBefore, "节点甲", 1),
    cardOf(fixtureBefore, "节点乙", 0),
  ];
  await boxSelect(box1);
  await snap("step11-box-selected");
  const selectedPng = image.decodePng(await api.screenshot());
  const borderPixels = selectedCards.map((card) =>
    pixelAt(
      selectedPng,
      Math.round(card.bounds.left + card.bounds.width / 4),
      Math.round(card.bounds.top) + 1,
    ),
  );
  report.check(
    borderPixels.every(isSelectionBorder),
    "步骤 11 框选后三个节点卡片呈选中态（三个卡片上边框像素均为选中主色）",
    JSON.stringify(borderPixels),
  );

  report.section("S11 批量移动与 20px 网格吸附（步骤 12）");
  const dragStart1 = ui.boundsCenter(textNodes(fixtureBefore, "节点甲")[0]);
  await dragHorizontally(dragStart1, 37);
  await sleep(600);
  await deselect();
  const snappedTree = await api.uiTree();
  const snappedBounds = {
    a: cardOf(snappedTree, "节点甲", 0).bounds,
    a2: cardOf(snappedTree, "节点甲", 1).bounds,
    b: cardOf(snappedTree, "节点乙", 0).bounds,
  };
  const snappedDpi = dpiOfCard(cardOf(snappedTree, "节点甲", 0));
  const gridPx = 20 * snappedDpi;
  const onSameGrid =
    sameGridLine(snappedBounds.a.left, snappedBounds.a2.left, snappedDpi) &&
    sameGridLine(snappedBounds.a.top, snappedBounds.a2.top, snappedDpi) &&
    sameGridLine(snappedBounds.a.left, snappedBounds.b.left, snappedDpi) &&
    sameGridLine(snappedBounds.a.top, snappedBounds.b.top, snappedDpi);
  report.check(
    onSameGrid,
    "步骤 12 第一次批量拖动后三个节点位置两两同余于 20 画布像素（吸附到同一网格）",
    JSON.stringify(snappedBounds),
  );
  await snap("step12-first-drag-snapped");

  const box2 = computeBox(snappedTree, ["节点甲", "节点甲", "节点乙"], 30);
  await boxSelect(box2);
  const dragStart2 = ui.boundsCenter(textNodes(snappedTree, "节点甲")[0]);
  await dragHorizontally(dragStart2, 53);
  await sleep(700);
  await deselect();
  const movedTree = await api.uiTree();
  const boundsAfter = {
    a: cardOf(movedTree, "节点甲", 0).bounds,
    a2: cardOf(movedTree, "节点甲", 1).bounds,
    b: cardOf(movedTree, "节点乙", 0).bounds,
  };
  const delta = {
    a: { x: boundsAfter.a.left - snappedBounds.a.left, y: boundsAfter.a.top - snappedBounds.a.top },
    a2: {
      x: boundsAfter.a2.left - snappedBounds.a2.left,
      y: boundsAfter.a2.top - snappedBounds.a2.top,
    },
    b: { x: boundsAfter.b.left - snappedBounds.b.left, y: boundsAfter.b.top - snappedBounds.b.top },
  };
  report.check(
    delta.a.x === delta.a2.x &&
      delta.a2.x === delta.b.x &&
      delta.a.y === 0 &&
      delta.a2.y === 0 &&
      delta.b.y === 0,
    "步骤 12 各选中节点位移一致且垂直方向不变（UI 树 bounds 前后变化一致）",
    JSON.stringify(delta),
  );
  const gridSteps = delta.a.x / gridPx;
  report.check(
    Math.abs(gridSteps - 2) <= 0.05,
    "步骤 12 第二次拖动位移为 40 画布像素（20 画布像素的整数倍，53 物理像素吸附为 40）",
    `物理位移=${delta.a.x}px，20 画布像素=${gridPx}px，格数=${gridSteps}`,
  );
  const accountAfter = textNodes(movedTree, "账号 A")[0]?.bounds;
  const noteAfter = textNodes(movedTree, "备注 B")[0]?.bounds;
  report.check(
    JSON.stringify(accountBefore) === JSON.stringify(accountAfter) &&
      JSON.stringify(noteBefore) === JSON.stringify(noteAfter),
    "步骤 12 未被框选的 fixture 节点未被批量移动",
  );
  await snap("step12-second-drag-moved");

  report.section("S12 删除键失败路径（步骤 21、F4）");
  await ui.clickText("节点甲", { count: 1 });
  await sleep(400);
  await api.postInput([api.keyClick(["Delete"]), api.keyClick(["Backspace"])]);
  await sleep(600);
  const afterKeysTree = await api.uiTree();
  report.check(
    textNodes(afterKeysTree, "节点甲").length === 2 && textNodes(afterKeysTree, "节点乙").length === 1,
    "F4 Delete/Backspace 后节点仍存在（delete-key-code=null，删除键已禁用）",
  );
  await snap("f4-delete-key-ignored");

  report.section("S13 视口缩放（步骤 13）");
  const zoomBefore = textNodes(afterKeysTree, "节点甲")[0].bounds;
  await api.postInput([api.mouseMove(NEUTRAL_POINT.x, 320), api.mouseScroll("down", 3)]);
  await sleep(1200);
  const zoomedTree = await api.uiTree();
  const zoomAfter = textNodes(zoomedTree, "节点甲")[0].bounds;
  report.check(
    zoomAfter.width < zoomBefore.width - 2 || zoomAfter.height < zoomBefore.height - 2,
    "步骤 13 滚轮缩小后「节点甲」bounds 尺寸变小",
    `${JSON.stringify(zoomBefore)} -> ${JSON.stringify(zoomAfter)}`,
  );
  await snap("step13-zoomed-out");

  report.section("S14 视口持久化（步骤 14）");
  await api.postInput([api.mouseMove(700, 300)]);
  await ui.clickText("画布宇宙");
  await ui.waitForText("根画布", { timeout: 10000 });
  await sleep(1000);
  await snap("step14-universe");
  await ui.clickText("根画布", { count: 2 });
  await ui.waitForText("账号 A", { timeout: 15000 });
  await sleep(1200);
  const backTree = await api.uiTree();
  const backBounds = textNodes(backTree, "节点甲")[0]?.bounds;
  report.check(
    backBounds !== undefined &&
      Math.abs(backBounds.width - zoomAfter.width) <= 2 &&
      Math.abs(backBounds.height - zoomAfter.height) <= 2,
    "步骤 14 进入画布宇宙再返回后「节点甲」尺寸与缩放后一致（±2px），视口缩放持久化",
    `${JSON.stringify(zoomAfter)} -> ${JSON.stringify(backBounds)}`,
  );
  await snap("step14-viewport-persisted");

  report.section("S15 逻辑删除与徽标（步骤 15）");
  const beforeDeleteTree = await api.uiTree();
  const nodeBBeforeDelete = cardOf(beforeDeleteTree, "节点乙").bounds;
  await hoverNodeAction("节点乙", "删除节点");
  await ui.waitForGone("节点乙", { timeout: 10000 });
  await sleep(900);
  const deletedTree = await api.uiTree();
  report.check(!ui.hasText(deletedTree, "节点乙"), "步骤 15 「节点乙」立即从画布消失（逻辑删除无确认框）");
  report.check(
    badgeTexts(deletedTree).includes("1"),
    "步骤 15 回收站按钮出现徽标「1」",
    JSON.stringify(badgeTexts(deletedTree)),
  );
  await snap("step15-deleted-badge");

  report.section("S16 回收站恢复节点（步骤 16）");
  await ensureRecycleBinPanel();
  const recyclePanelTree = await api.uiTree();
  report.check(ui.hasText(recyclePanelTree, "节点乙"), "步骤 16 回收站面板出现「节点乙」条目");
  report.check(
    ui.findAttr(recyclePanelTree, "title", "恢复节点") !== null,
    "步骤 16 面板条目包含「恢复节点」按钮",
  );
  await snap("step16-panel-open");
  await ui.clickNode(
    await waitForCondition(async () => ui.findAttr(await api.uiTree(), "title", "恢复节点"), {
      timeout: 6000,
      label: "恢复按钮可见",
    }),
  );
  await waitForCondition(
    async () => {
      const t = await api.uiTree();
      return ui.hasText(t, "节点乙") && ui.findAttr(t, "title", "恢复节点") === null;
    },
    { timeout: 10000, label: "节点恢复且面板条目消失" },
  );
  await sleep(900);
  const restoredTree = await api.uiTree();
  const restoredCard = cardOf(restoredTree, "节点乙");
  report.check(restoredCard !== null, "步骤 16 「节点乙」重新出现在画布");
  report.check(
    Math.abs(restoredCard.bounds.left - nodeBBeforeDelete.left) <= 2 &&
      Math.abs(restoredCard.bounds.top - nodeBBeforeDelete.top) <= 2,
    "步骤 16 恢复后位于原坐标（±2px）",
    `${JSON.stringify(nodeBBeforeDelete)} -> ${JSON.stringify(restoredCard.bounds)}`,
  );
  report.check(
    ui.findAttr(restoredTree, "title", "恢复节点") === null,
    "步骤 16 面板中该条目消失",
  );
  await snap("step16-restored");

  report.section("S17 永久删除确认框与取消（步骤 17、F2）");
  await hoverNodeAction("节点乙", "删除节点");
  await ui.waitForGone("节点乙", { timeout: 10000 });
  await sleep(800);
  await ensureRecycleBinPanel();
  await ui.clickNode(
    await waitForCondition(async () => ui.findAttr(await api.uiTree(), "title", "永久删除"), {
      timeout: 6000,
      label: "永久删除按钮可见",
    }),
  );
  await ui.waitForDialog("永久删除节点", { timeout: 8000 });
  const physicalText = `确定要永久删除节点"节点乙"吗？此操作不可撤销。`;
  const physicalShown = await ui.waitForTextStable(physicalText, { timeout: 6000 }).catch(() => null);
  report.check(
    physicalShown !== null,
    "步骤 17 确认框标题「永久删除节点」与正文与预期一致",
    physicalText,
  );
  await snap("step17-physical-confirm");
  const physicalDialog = ui.findDialog(await api.uiTree(), "永久删除节点");
  await ui.clickNode(ui.findButton(await api.uiTree(), "取消", { root: physicalDialog }));
  const physicalGone = await ui
    .waitForDialogGone("永久删除节点", { timeout: 6000 })
    .catch(() => false);
  report.check(physicalGone, "F2 点击「取消」后确认框关闭");
  await ensureRecycleBinPanel();
  const keptTree = await api.uiTree();
  report.check(
    ui.findAttr(keptTree, "title", "永久删除") !== null,
    "F2 取消后「节点乙」仍保留在回收站面板中",
  );
  await snap("f2-cancel-kept");

  report.section("S18 永久删除（步骤 18）");
  await ui.clickNode(
    await waitForCondition(async () => ui.findAttr(await api.uiTree(), "title", "永久删除"), {
      timeout: 6000,
      label: "永久删除按钮可见",
    }),
  );
  await ui.waitForDialog("永久删除节点", { timeout: 8000 });
  await sleep(400);
  const physicalDialog2 = ui.findDialog(await api.uiTree(), "永久删除节点");
  await ui.clickNode(ui.findButton(await api.uiTree(), "确认", { root: physicalDialog2 }));
  await sleep(1400);
  const physicallyDeletedTree = await api.uiTree();
  report.check(!ui.hasText(physicallyDeletedTree, "节点乙"), "步骤 18 永久删除后画布中无「节点乙」");
  report.check(badgeTexts(physicallyDeletedTree).length === 0, "步骤 18 回收站徽标消失");
  await ensureRecycleBinPanel();
  const emptyBinTree = await api.uiTree();
  report.check(ui.hasText(emptyBinTree, "回收站为空"), "步骤 18 重新打开回收站面板显示「回收站为空」");
  await snap("step18-recycle-bin-empty");

  report.section("S19 重建节点后清空回收站（步骤 19）");
  await api.postInput([api.mouseMove(700, 550), api.mouseClick("left", 1)]);
  await sleep(500);
  await dragCreateDataNode({ x: 1150, y: 950 });
  await renameNewNode(0, "节点乙");
  report.check(textNodes(await api.uiTree(), "节点乙").length === 1, "步骤 19 重建「节点乙」成功");
  await hoverNodeAction("节点乙", "删除节点");
  await ui.waitForGone("节点乙", { timeout: 10000 });
  await sleep(900);
  await ensureRecycleBinPanel();
  const clearTree = await api.uiTree();
  const clearButton = ui.findButton(clearTree, "清空回收站");
  report.check(clearButton?.states?.disabled === false, "步骤 19 面板底部「清空回收站」按钮可用");
  await ui.clickNode(clearButton);
  await ui.waitForDialog("清空回收站", { timeout: 8000 });
  const emptyText = "确定要清空回收站吗？共 1 个节点将被永久删除。此操作不可撤销。";
  const emptyShown = await ui.waitForTextStable(emptyText, { timeout: 6000 }).catch(() => null);
  report.check(emptyShown !== null, "步骤 19 清空回收站确认框标题与正文与预期一致", emptyText);
  await snap("step19-empty-confirm");
  await sleep(400);
  const clearDialog = ui.findDialog(await api.uiTree(), "清空回收站");
  await ui.clickNode(ui.findButton(await api.uiTree(), "确认", { root: clearDialog }));
  await sleep(1600);
  const emptiedTree = await api.uiTree();
  report.check(!ui.hasText(emptiedTree, "节点乙"), "步骤 19 清空后画布中无「节点乙」");
  report.check(badgeTexts(emptiedTree).length === 0, "步骤 19 清空后回收站徽标消失");
  await ensureRecycleBinPanel();
  const emptiedPanelTree = await api.uiTree();
  report.check(ui.hasText(emptiedPanelTree, "回收站为空"), "步骤 19 清空后面板显示「回收站为空」");
  await snap("step19-emptied");
}

let fatal = null;
try {
  prepareCaseOutput(OUTPUT_DIR);
  prepareDataDir(DATA_DIR, "base");
  await withApp(
    { dataDir: DATA_DIR, logFile: path.join(OUTPUT_DIR, "app.log") },
    async () => {
      try {
        await main();
      } catch (error) {
        await snap("fatal").catch(() => {});
        throw error;
      }
    },
  );
} catch (error) {
  fatal = error;
  report.check(false, "用例执行中断", error.message);
} finally {
  finalizeReport(report, OUTPUT_DIR, fatal);
}
