// case_017 节点标签与书签。
//
// 流程：复制 base fixture → 启动应用 → 解锁（lastScene 恢复根画布）
// → S1 标签云空态 + 编辑对话框收藏书签与标签规整（重复/纯空格输入后仅一个 chip）+
//   保存后卡片出现书签图标（像素断言）
// → S2 重开编辑对话框回填已收藏态与标签，取消分支不写库
// → S3 标签云面板与模板面板双向互斥、「标签：重要」分组列表、Esc 关闭后标签云重载
// → S4 书签列表嵌套编辑取消书签 → 列表重载空态 → 画布书签图标消失（nodeUpdated 同步）→
//   重开对话框确认书签未收藏且标签保留
// → S5 给「备注 B」加标签「重要」、给「账号 A」重新加书签
// → S6 全局搜索「重要」经标签匹配命中两个节点、候选副标题为本地化「根画布」
// → S7 子画布构造（新画布/新节点）+ 标签云双标签字号分档（重要 33 vs 次要 24 物理像素）
// → S8 书签列表双画布分组（组序与 localeCompare 一致）+ 跳转到子画布并居中
// → S9 日志对话框按关键词过滤断言 NodeTagsModify / NodeBookmarkModify 名称与详情文案
// → 输出 PASS/FAIL 报告。
//
// 脚本规范要点：
// - 断言优先使用 UI 树（role / name / attrs / bounds / states）：标签 chip 不产生独立节点，
//   其文本聚合进标签 combobox 的 text（如「重要 标签」），按聚合文本断言恰好一个目标标签；
// - 书签图标是无 title 的 VIcon，不进入 UI 树，用截图像素计数断言（实测基线 68、收藏 234、
//   取消 68，判据 b-r>=80 且 b>=150，区域取卡片 bounds 内缩 4 物理像素以排除选中边框）；
// - 书签/标签列表对话框中每行都有「编辑节点」「跳转到节点」按钮，必须按节点标题的垂直位置
//   取同行按钮，不能取首个命中；
// - 嵌套编辑对话框与列表对话框的标题都用「#text 精确等于标题」定位（findDialog 的宽泛
//   子树文本匹配会把按钮 name 计入，嵌套时会命中错误的对话框）；
// - 等待一律用等待函数表达状态条件（对话框出现/消失、空态、文本出现、节点位置稳定），
//   sleep 仅用于动画稳定；
// - 应用生命周期由 withApp 包装，保证无论成败都经 POST /shutdown 收尾；
// - 关键步骤截图存 output；流程中断时额外截取 fatal 截图并写入 report.json。
//
// 运行方式：在项目根目录执行 `node e2e\script\case_017\case_017.js`

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

/** S1/S5 使用的标签名 */
const TAG_IMPORTANT = "重要";
/** S7 子画布节点使用的标签名 */
const TAG_SECONDARY = "次要";

/** 画布数据节点「新画布」的拖拽创建落点（窗口物理像素） */
const CHILD_DROP = { x: 350, y: 400 };
/** 子画布内数据节点「新节点」的拖拽创建落点（窗口物理像素） */
const CHILD_NODE_DROP = { x: 800, y: 600 };
/** 点击画布空白处取消选中并复位 hover 的坐标（窗口物理像素） */
const BLANK_POINT = { x: 200, y: 900 };
/** 清场坐标（窗口物理像素，避免悬浮按钮/提示残留） */
const NEUTRAL_POINT = { x: 1200, y: 900 };
/** 画布深色点阵背景上的蓝色像素判据（与主题 primary #1976D2 同色系） */
const BLUE_MIN_CHANNEL = 150;
const BLUE_MIN_DIFF = 80;
/** 书签图标出现判定：收藏后蓝色像素相对基线的增量下限（实测增量 166） */
const BOOKMARK_BLUE_DELTA_MIN = 100;
/** 书签图标消失判定：取消后蓝色像素相对基线的余量上限（实测余量 0） */
const BOOKMARK_BLUE_RESIDUAL_MAX = 40;
/** 标签字号分档判定：大标签较小标签的高度差下限（实测 33 vs 24） */
const TAG_FONT_HEIGHT_DELTA_MIN = 4;
/** 跳转居中容差：与画布区域中心的最大偏差比例（沿用 case_010 契约） */
const CENTER_TOLERANCE = 0.2;

const report = new Report("case_017 节点标签与书签");

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

// ---------- UI 树查询 ----------

/**
 * 返回 UI 树中的全部 vue-flow 节点卡片（role=group、带 data-id、aria-roledescription=node）。
 * @param {object[]} tree UI 树顶层节点数组。
 * @returns {object[]} 卡片节点数组。
 */
function cards(tree) {
  return ui
    .flatten(tree)
    .filter(
      ({ node }) =>
        node.role === "group" &&
        node.attrs?.["data-id"] &&
        node.attrs["aria-roledescription"] === "node",
    )
    .map(({ node }) => node);
}

/**
 * 按标题文本查找节点卡片。
 * @param {object[]} tree UI 树顶层节点数组。
 * @param {string} text 节点标题。
 * @returns {object|null} 卡片；未命中时为 null。
 */
function cardOf(tree, text) {
  return (
    cards(tree).find((card) =>
      ui
        .flatten([card])
        .some(({ node }) => node.tag === "#text" && (node.text ?? "").trim() === text),
    ) ?? null
  );
}

/**
 * 返回 UI 树中文本精确匹配的 #text 节点（按 top、left 升序取首个）。
 * @param {object[]} tree UI 树顶层节点数组。
 * @param {string} text 目标文本。
 * @returns {object|null} 文本节点；未命中时为 null。
 */
function textNode(tree, text) {
  return (
    ui
      .flatten(tree)
      .map(({ node }) => node)
      .filter((node) => node.tag === "#text" && (node.text ?? "").trim() === text)
      .sort((a, b) => a.bounds.top - b.bounds.top || a.bounds.left - b.bounds.left)[0] ?? null
  );
}

/**
 * 返回面包屑各级文案（根画布在首、当前画布在尾）。
 * @param {object[]} tree UI 树顶层节点数组。
 * @returns {string[]} 面包屑文案数组。
 */
function crumbText(tree) {
  return ui
    .flatten(tree)
    .map(({ node }) => node)
    .filter((node) => node.role === "listitem" && node.bounds.top < 140)
    .sort((a, b) => a.bounds.left - b.bounds.left)
    .map((node) => ui.subtreeText(node).trim());
}

/**
 * 按「#text 精确等于标题」定位最内层对话框（嵌套时取面积最小者）。
 * 列表对话框的按钮 name（如「编辑节点」）会进入 subtreeText，故不能用 findDialog 的
 * 宽泛匹配做嵌套定位。
 * @param {object[]} tree UI 树顶层节点数组。
 * @param {string} title 对话框标题文本。
 * @returns {object|null} 对话框节点；未命中时为 null。
 */
function dialogByTitle(tree, title) {
  const hits = ui
    .findByRole(tree, "dialog")
    .filter(
      (dialog) =>
        dialog.bounds &&
        dialog.bounds.width > 100 &&
        dialog.bounds.height > 80 &&
        ui
          .flatten([dialog])
          .some(({ node }) => node.tag === "#text" && (node.text ?? "").trim() === title),
    );
  hits.sort((a, b) => a.bounds.width * a.bounds.height - b.bounds.width * b.bounds.height);
  return hits[0] ?? null;
}

/**
 * 返回编辑对话框内的标签 combobox。
 * @param {object[]} tree UI 树顶层节点数组。
 * @returns {object|null} combobox 节点；未命中时为 null。
 */
function tagCombobox(tree) {
  return (
    ui
      .flatten(tree)
      .map(({ node }) => node)
      .find((node) => node.role === "combobox" && ui.ownText(node).includes("标签")) ?? null
  );
}

/**
 * 返回标签 combobox 的聚合文本分词（chip 文本与 label 文本）。
 * @param {object[]} tree UI 树顶层节点数组。
 * @returns {string[]} 分词数组。
 */
function tagTokens(tree) {
  return (tagCombobox(tree)?.text ?? "").split(/\s+/).filter(Boolean);
}

/**
 * 返回标签云面板是否打开（面板标题为 #text「标签云」；工具栏按钮不是 #text）。
 * @param {object[]} tree UI 树顶层节点数组。
 * @returns {boolean} 面板打开时为 true。
 */
function tagCloudOpen(tree) {
  return ui
    .flatten(tree)
    .some(({ node }) => node.tag === "#text" && (node.text ?? "").trim() === "标签云");
}

/**
 * 在列表对话框中查找指定节点行的操作按钮（取与节点标题文本垂直距离最近者）。
 * @param {object[]} tree UI 树顶层节点数组。
 * @param {string} nodeTitle 节点标题文本。
 * @param {string} actionTitle 按钮 title（如「编辑节点」「跳转到节点」）。
 * @returns {object|null} 按钮节点；未命中时为 null。
 */
function listActionForNode(tree, nodeTitle, actionTitle) {
  const title = ui
    .flatten(tree)
    .map(({ node }) => node)
    .find((node) => node.tag === "#text" && (node.text ?? "").trim() === nodeTitle);
  if (!title) {
    return null;
  }
  const buttons = ui
    .flatten(tree)
    .map(({ node }) => node)
    .filter((node) => node.attrs?.title === actionTitle);
  buttons.sort(
    (a, b) =>
      Math.abs(a.bounds.top - title.bounds.top) - Math.abs(b.bounds.top - title.bounds.top),
  );
  return buttons[0] ?? null;
}

/**
 * 返回全局搜索输入框（placeholder 为「搜索节点…」）。
 * @param {object[]} tree UI 树顶层节点数组。
 * @returns {object|null} 输入框节点；未命中时为 null。
 */
function searchInput(tree) {
  return (
    ui
      .flatten(tree)
      .map(({ node }) => node)
      .find((node) => node.editable && node.attrs?.placeholder === "搜索节点…") ?? null
  );
}

/**
 * 返回搜索候选列表项（下拉内 role=listitem、宽大于 300、位于上半窗）。
 * @param {object[]} tree UI 树顶层节点数组。
 * @returns {object[]} 候选列表项数组（按 top 升序）。
 */
function candidateItems(tree) {
  return ui
    .flatten(tree)
    .map(({ node }) => node)
    .filter((node) => node.role === "listitem" && node.bounds.width > 300 && node.bounds.top < 600)
    .sort((a, b) => a.bounds.top - b.bounds.top);
}

/**
 * 返回日志对话框（标题 #text 为「日志」）。
 * @param {object[]} tree UI 树顶层节点数组。
 * @returns {object|null} 日志对话框；未命中时为 null。
 */
function logDialog(tree) {
  return dialogByTitle(tree, "日志");
}

/**
 * 返回日志对话框的搜索输入框（name 为「搜索日志内容」）。
 * @param {object[]} tree UI 树顶层节点数组。
 * @returns {object|null} 输入框；未命中时为 null。
 */
function logSearchInput(tree) {
  return (
    ui
      .flatten(tree)
      .map(({ node }) => node)
      .find((node) => node.editable && node.name === "搜索日志内容") ?? null
  );
}

// ---------- 像素 ----------

/**
 * 统计 PNG 中指定矩形内蓝色像素数量（判据 b-r>=80 且 b>=150，矩形按 inset 内缩）。
 * @param {import("pngjs").PNG} png 整窗截图 PNG。
 * @param {{left: number, top: number, width: number, height: number}} region 统计区域。
 * @param {number} [inset=4] 内缩像素（排除卡片选中边框）。
 * @returns {number} 蓝色像素数量。
 */
function countBluePixels(png, region, inset = 4) {
  const left = Math.round(region.left) + inset;
  const top = Math.round(region.top) + inset;
  const right = Math.round(region.left + region.width) - inset;
  const bottom = Math.round(region.top + region.height) - inset;
  let count = 0;
  for (let y = top; y < bottom; y += 1) {
    for (let x = left; x < right; x += 1) {
      const index = (png.width * y + x) << 2;
      const r = png.data[index];
      const b = png.data[index + 2];
      if (b - r >= BLUE_MIN_DIFF && b >= BLUE_MIN_CHANNEL) {
        count += 1;
      }
    }
  }
  return count;
}

/**
 * 截取整窗并统计指定卡片区域内的蓝色像素数量（调用前应确保界面状态稳定）。
 * @param {{left: number, top: number, width: number, height: number}} bounds 卡片 bounds。
 * @returns {Promise<number>} 蓝色像素数量。
 */
async function cardBlueCount(bounds) {
  return countBluePixels(image.decodePng(await api.screenshot()), bounds);
}

// ---------- 等待 ----------

/**
 * 等待标签云面板出现标签按钮。
 * @param {string} tag 标签名。
 * @param {object} [options] 可选参数。
 * @param {number} [options.timeout=8000] 超时毫秒数。
 * @returns {Promise<object>} 标签按钮节点。
 */
async function waitTagButton(tag, { timeout = 8000 } = {}) {
  return waitForCondition(
    async () => ui.findButton(await api.uiTree(), tag, { exact: true }),
    { timeout, interval: 200, label: `标签按钮「${tag}」` },
  );
}

/**
 * 等待指定标题的对话框出现且编辑对话框数据加载完成。
 * @param {string} title 对话框标题。
 * @param {object} [options] 可选参数。
 * @param {number} [options.timeout=8000] 超时毫秒数。
 * @returns {Promise<object>} 对话框节点。
 */
async function waitDialogByTitle(title, { timeout = 8000 } = {}) {
  return waitForCondition(
    async () => dialogByTitle(await api.uiTree(), title),
    { timeout, interval: 200, label: `对话框「${title}」` },
  );
}

/**
 * 等待指定标题的对话框消失。
 * @param {string} title 对话框标题。
 * @param {object} [options] 可选参数。
 * @param {number} [options.timeout=8000] 超时毫秒数。
 * @returns {Promise<boolean>} 消失时为 true。
 */
async function waitDialogByTitleGone(title, { timeout = 8000 } = {}) {
  await waitForCondition(
    async () => dialogByTitle(await api.uiTree(), title) === null,
    { timeout, interval: 200, label: `对话框「${title}」消失` },
  );
  return true;
}

/**
 * 等待指定文本精确出现，并在短暂静止期后复查仍在（规避动画中间态）。
 * @param {string} text 目标文本。
 * @param {object} [options] 可选参数。
 * @param {number} [options.timeout=8000] 超时毫秒数。
 * @param {number} [options.settleMs=400] 静止复查毫秒数。
 * @returns {Promise<object|null>} 复查命中的文本节点；超时为 null。
 */
async function waitTextStableOrNull(text, { timeout = 8000, settleMs = 400 } = {}) {
  return waitForCondition(
    async () => {
      const tree = await api.uiTree();
      if (!textNode(tree, text)) {
        return null;
      }
      await sleep(settleMs);
      return textNode(await api.uiTree(), text);
    },
    { timeout, interval: 250, label: `文本「${text}」稳定` },
  )
    .then((node) => node)
    .catch(() => null);
}

/**
 * 等待指定标题的节点文本位置连续两次读取一致（用于跳转居中动画结束）。
 * @param {string} title 节点标题。
 * @param {object} [options] 可选参数。
 * @param {number} [options.timeout=6000] 超时毫秒数。
 * @returns {Promise<object>} 稳定后的文本节点。
 */
async function waitNodePositionStable(title, { timeout = 6000 } = {}) {
  let previous = null;
  return waitForCondition(
    async () => {
      const node = textNode(await api.uiTree(), title);
      if (!node) {
        return null;
      }
      const key = `${Math.round(node.bounds.left)},${Math.round(node.bounds.top)}`;
      if (previous === key) {
        return node;
      }
      previous = key;
      return null;
    },
    { timeout, interval: 300, label: `节点「${title}」位置稳定` },
  );
}

// ---------- 通用操作 ----------

/**
 * 等待首页就绪并确保界面语言为中文。
 * @returns {Promise<void>} 无返回值。
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
    return;
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
}

/**
 * 解锁 fixture 数据库并等待进入根画布。
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
 * 把鼠标移到清场坐标并等待 hover 状态退场。
 * @returns {Promise<void>} 无返回值。
 */
async function clearHover() {
  await api.postInput([api.mouseMove(NEUTRAL_POINT.x, NEUTRAL_POINT.y)]);
  await sleep(500);
}

/**
 * 点击画布空白处取消选中并复位 hover。
 * @returns {Promise<void>} 无返回值。
 */
async function deselect() {
  await api.postInput([api.mouseMove(BLANK_POINT.x, BLANK_POINT.y), api.mouseClick("left", 1)]);
  await sleep(650);
  await clearHover();
}

/**
 * 点击工具栏悬浮菜单按钮（按 attrs.title 定位）。
 * @param {string} title 按钮 title（如「新建节点」「书签」「标签云」「搜索」）。
 * @returns {Promise<void>} 无返回值。
 */
async function clickToolbar(title) {
  const button = await waitForCondition(
    async () => ui.findAttr(await api.uiTree(), "title", title),
    { timeout: 8000, interval: 200, label: `工具栏按钮「${title}」` },
  );
  await ui.clickNode(button);
}

/**
 * 确保标签云面板处于指定状态（按面板标题 #text 判断，用工具栏按钮 toggle）。
 * @param {boolean} open 目标状态：true 打开、false 关闭。
 * @returns {Promise<void>} 无返回值。
 */
async function ensureTagCloud(open) {
  if (tagCloudOpen(await api.uiTree()) === open) {
    return;
  }
  await clickToolbar("标签云");
  await waitForCondition(async () => tagCloudOpen(await api.uiTree()) === open, {
    timeout: 8000,
    interval: 200,
    label: `标签云面板${open ? "打开" : "关闭"}`,
  });
  await sleep(450);
}

/**
 * 双击节点卡片打开编辑对话框并等待控件就绪。
 * @param {string} title 节点标题。
 * @returns {Promise<void>} 无返回值。
 */
async function openEditDialog(title) {
  await clearHover();
  await ui.clickText(title, { count: 2 });
  await waitDialogByTitle("编辑节点");
  await waitForCondition(async () => tagCombobox(await api.uiTree()), {
    timeout: 8000,
    interval: 200,
    label: "编辑对话框数据加载完成（标签 combobox 出现）",
  });
  await sleep(350);
}

/**
 * 在编辑对话框中点击书签切换按钮（按当前 title 定位「加入书签」或「取消书签」）。
 * @param {string} currentTitle 当前按钮 title。
 * @param {string} nextTitle 点击后按钮应变为的 title。
 * @returns {Promise<void>} 无返回值。
 */
async function clickBookmarkToggle(currentTitle, nextTitle) {
  const button = await waitForCondition(
    async () => ui.findAttr(await api.uiTree(), "title", currentTitle),
    { timeout: 6000, interval: 200, label: `书签按钮「${currentTitle}」` },
  );
  await ui.clickNode(button);
  await waitForCondition(
    async () => ui.findAttr(await api.uiTree(), "title", nextTitle) !== null,
    { timeout: 6000, interval: 200, label: `书签按钮切换为「${nextTitle}」` },
  );
  await sleep(300);
}

/**
 * 点击编辑对话框的「确认」按钮并等待对话框关闭。
 * @returns {Promise<void>} 无返回值。
 */
async function confirmEditDialog() {
  const dialog = dialogByTitle(await api.uiTree(), "编辑节点");
  if (!dialog) {
    throw new Error("confirmEditDialog: edit dialog not found");
  }
  await ui.clickNode(ui.findButton(await api.uiTree(), "确认", { root: dialog, exact: true }));
  await waitDialogByTitleGone("编辑节点");
  await sleep(700);
}

/**
 * 点击编辑对话框的「取消」按钮并等待对话框关闭。
 * @returns {Promise<void>} 无返回值。
 */
async function cancelEditDialog() {
  const dialog = dialogByTitle(await api.uiTree(), "编辑节点");
  if (!dialog) {
    throw new Error("cancelEditDialog: edit dialog not found");
  }
  await ui.clickNode(ui.findButton(await api.uiTree(), "取消", { root: dialog, exact: true }));
  await waitDialogByTitleGone("编辑节点");
  await sleep(700);
}

/**
 * 在编辑对话框的标签 combobox 中输入一个标签并回车生成 chip。
 * @param {string} text 标签文本（可含首尾空白，用于验证规整）。
 * @returns {Promise<void>} 无返回值。
 */
async function typeTag(text) {
  const combo = tagCombobox(await api.uiTree());
  if (!combo) {
    throw new Error("typeTag: tag combobox not found");
  }
  await ui.clickNode(combo);
  await api.postInput([api.typeText(text), api.keyClick(["Enter"])]);
  await sleep(450);
}

/**
 * 在编辑对话框中完成「收藏/取消收藏 + 标签输入」并保存。
 * @param {string} title 节点标题。
 * @param {object} options 操作选项。
 * @param {boolean|null} options.bookmark 目标书签状态；null 表示不切换。
 * @param {string|null} options.tag 要添加的标签文本；null 表示不添加。
 * @returns {Promise<void>} 无返回值。
 */
async function editBookmarkAndTag(title, { bookmark = null, tag = null } = {}) {
  await openEditDialog(title);
  if (bookmark === true) {
    await clickBookmarkToggle("加入书签", "取消书签");
  } else if (bookmark === false) {
    await clickBookmarkToggle("取消书签", "加入书签");
  }
  if (tag !== null) {
    await typeTag(tag);
  }
  await confirmEditDialog();
}

/**
 * 打开书签列表对话框。
 * @returns {Promise<void>} 无返回值。
 */
async function openBookmarkList() {
  await clickToolbar("书签");
  await waitDialogByTitle("书签");
  await sleep(500);
}

/**
 * 打开日志对话框（点击右下角「日志」按钮）并等待日志加载。
 * @returns {Promise<void>} 无返回值。
 */
async function openLogDialog() {
  const button = await waitForCondition(
    async () => {
      const tree = await api.uiTree();
      return (
        ui
          .flatten(tree)
          .map(({ node }) => node)
          .filter((node) => node.role === "button" && ui.subtreeText(node).includes("日志"))
          .sort((a, b) => b.bounds.top - a.bounds.top)[0] ?? null
      );
    },
    { timeout: 8000, interval: 250, label: "「日志」按钮" },
  );
  await ui.clickNode(button);
  await waitDialogByTitle("日志");
  await sleep(1200);
}

/**
 * 在日志对话框中按关键词过滤（先聚焦输入框全选替换，再点「搜索」按钮）。
 * @param {string} keyword 关键词。
 * @returns {Promise<void>} 无返回值。
 */
async function filterLogs(keyword) {
  const input = await waitForCondition(
    async () => logSearchInput(await api.uiTree()),
    { timeout: 6000, interval: 200, label: "「搜索日志内容」输入框" },
  );
  await ui.clickNode(input);
  await api.postInput([api.keyClick(["Control", "a"]), api.typeText(keyword)]);
  const button = await waitForCondition(
    async () =>
      ui
        .flatten(await api.uiTree())
        .map(({ node }) => node)
        .find((node) => node.role === "button" && (node.text ?? "").trim() === "搜索") ?? null,
    { timeout: 6000, interval: 200, label: "日志对话框「搜索」按钮" },
  );
  await ui.clickNode(button);
  await sleep(1000);
}

/**
 * 从模板面板拖拽指定手柄到画布落点创建节点，并等待面板自动关闭。
 * @param {string} handleTitle 拖拽手柄的 title。
 * @param {{x: number, y: number}} drop 落点（窗口物理像素）。
 * @returns {Promise<void>} 无返回值。
 */
async function dragCreateNode(handleTitle, drop) {
  const tree = await api.uiTree();
  if (!ui.findAttr(tree, "title", handleTitle)) {
    await clickToolbar("新建节点");
    await waitForCondition(async () => ui.findAttr(await api.uiTree(), "title", handleTitle), {
      timeout: 8000,
      interval: 200,
      label: `模板手柄「${handleTitle}」`,
    });
    await sleep(500);
  }
  const handle = ui.findAttr(await api.uiTree(), "title", handleTitle);
  await api.postInput([api.mouseDrag(ui.boundsCenter(handle), drop)]);
  await waitForCondition(async () => ui.findAttr(await api.uiTree(), "title", handleTitle) === null, {
    timeout: 8000,
    interval: 200,
    label: "模板面板自动关闭",
  });
  await sleep(800);
}

/**
 * 用例主流程。
 * @returns {Promise<void>} 无返回值。
 */
async function main() {
  const info = await api.health();

  report.section("S0 前置与解锁（前置条件）");
  report.check(info.width > 0 && info.height > 0, "调试自动化服务可用", `${info.width}x${info.height}`);
  await ensureChineseUi();
  await unlock();
  const unlocked = await api.uiTree();
  const accountCard = cardOf(unlocked, "账号 A");
  const noteCard = cardOf(unlocked, "备注 B");
  report.check(accountCard !== null && noteCard !== null, "解锁后根画布出现「账号 A」与「备注 B」");
  report.check(
    crumbText(unlocked).join("/") === "画布宇宙/根画布",
    "按 lastScene 恢复场景（面包屑为「画布宇宙 / 根画布」）",
    crumbText(unlocked).join("/"),
  );
  await snap("unlocked-root-canvas");

  report.section("S1 收藏与标签规整（步骤 S1、F1、F3）");
  // 标签云空态（当前无任何标签）
  await ensureTagCloud(true);
  const emptyCloudTree = await api.uiTree();
  report.check(ui.hasText(emptyCloudTree, "暂无标签"), "S1 无标签时标签云显示空态「暂无标签」");
  report.check(
    ui.findButton(emptyCloudTree, TAG_IMPORTANT, { exact: true }) === null,
    "S1 空态标签云无标签按钮",
  );
  await snap("s1-tag-cloud-empty");
  await ensureTagCloud(false);
  // 收藏前的书签图标像素基线
  await clearHover();
  const accountBounds = cardOf(await api.uiTree(), "账号 A").bounds;
  const baseBlue = await cardBlueCount(accountBounds);
  console.log(`  (pixel) bookmark icon baseline blue = ${baseBlue}`);
  // 双击打开编辑对话框：书签切换 + 标签规整
  await openEditDialog("账号 A");
  const dialogTree = await api.uiTree();
  report.check(tagCombobox(dialogTree) !== null, "S1 编辑对话框包含「标签」combobox（可编辑）");
  report.check(
    tagCombobox(dialogTree)?.states?.disabled !== true,
    "S1 数据节点的标签 combobox 非禁用",
  );
  report.check(
    ui.findAttr(dialogTree, "title", "加入书签") !== null,
    "S1 未收藏时书签按钮 title 为「加入书签」",
  );
  await clickBookmarkToggle("加入书签", "取消书签");
  report.check(
    ui.findAttr(await api.uiTree(), "title", "取消书签") !== null,
    "S1 点击书签按钮后 title 变为「取消书签」（已收藏态）",
  );
  await typeTag(TAG_IMPORTANT);
  report.check(
    tagTokens(await api.uiTree()).includes(TAG_IMPORTANT),
    "S1 输入「重要」回车后 combobox 出现该标签",
    tagTokens(await api.uiTree()).join("|"),
  );
  await typeTag(` ${TAG_IMPORTANT} `);
  await typeTag("   ");
  const normalizedTokens = tagTokens(await api.uiTree());
  report.check(
    normalizedTokens.filter((token) => token === TAG_IMPORTANT).length === 1 &&
      normalizedTokens.length === 2,
    "F1 重复值（「 重要 」）与纯空白输入被规整：combobox 中「重要」恰好出现一次、无空标签",
    normalizedTokens.join("|"),
  );
  await snap("s1-edit-dialog-bookmark-tag");
  await confirmEditDialog();
  await deselect();
  const afterS1Blue = await cardBlueCount(cardOf(await api.uiTree(), "账号 A").bounds);
  console.log(`  (pixel) after bookmark blue = ${afterS1Blue}`);
  report.check(
    afterS1Blue >= baseBlue + BOOKMARK_BLUE_DELTA_MIN,
    `S1 保存后卡片出现书签图标（蓝色像素较基线增加 ≥${BOOKMARK_BLUE_DELTA_MIN}）`,
    `base=${baseBlue} after=${afterS1Blue}`,
  );
  await snap("s1-bookmark-icon-visible");

  report.section("S2 回填与取消分支（步骤 S2、F2）");
  await openEditDialog("账号 A");
  report.check(
    ui.findAttr(await api.uiTree(), "title", "取消书签") !== null,
    "S2 重开编辑对话框：书签按钮 title 为「取消书签」（已收藏态回填）",
  );
  const refillTokens = tagTokens(await api.uiTree());
  report.check(
    refillTokens.filter((token) => token === TAG_IMPORTANT).length === 1,
    "S2 重开编辑对话框：标签「重要」回填恰好一次",
    refillTokens.join("|"),
  );
  await snap("s2-reopen-refilled");
  await cancelEditDialog();
  await deselect();
  const afterS2Blue = await cardBlueCount(cardOf(await api.uiTree(), "账号 A").bounds);
  report.check(
    afterS2Blue >= baseBlue + BOOKMARK_BLUE_DELTA_MIN,
    "F2 点击「取消」不写库：书签保持收藏态（蓝色像素仍高于基线）",
    `base=${baseBlue} after=${afterS2Blue}`,
  );

  report.section("S3 标签云分组列表与面板互斥（步骤 S3）");
  await ensureTagCloud(true);
  const cloudTree = await api.uiTree();
  report.check(
    ui.findButton(cloudTree, TAG_IMPORTANT, { exact: true }) !== null,
    "S3 标签云面板出现标签按钮「重要」",
  );
  report.check(!ui.hasText(cloudTree, "暂无标签"), "S3 有标签后空态文案消失");
  await snap("s3-tag-cloud");
  // 互斥：打开模板面板 → 标签云关闭
  await clickToolbar("新建节点");
  await waitForCondition(
    async () => ui.findAttr(await api.uiTree(), "title", "拖拽创建数据节点") !== null,
    { timeout: 8000, interval: 200, label: "模板面板展开" },
  );
  await sleep(450);
  report.check(
    !tagCloudOpen(await api.uiTree()) &&
      ui.findButton(await api.uiTree(), TAG_IMPORTANT, { exact: true }) === null,
    "S3 展开模板面板后标签云关闭（面板互斥）",
  );
  // 反向互斥：点击标签云 → 模板面板关闭
  await clickToolbar("标签云");
  await waitForCondition(async () => tagCloudOpen(await api.uiTree()), {
    timeout: 8000,
    interval: 200,
    label: "标签云面板重新打开",
  });
  await waitForCondition(
    async () => ui.findAttr(await api.uiTree(), "title", "拖拽创建数据节点") === null,
    { timeout: 6000, interval: 200, label: "模板面板关闭" },
  );
  report.check(true, "S3 重新点击标签云后模板面板关闭（反向互斥）");
  // 点击标签打开分组列表
  await ui.clickNode(await waitTagButton(TAG_IMPORTANT));
  await waitDialogByTitle(`标签：${TAG_IMPORTANT}`);
  await sleep(600);
  const tagDialog = dialogByTitle(await api.uiTree(), `标签：${TAG_IMPORTANT}`);
  const tagDialogText = ui.subtreeText(tagDialog);
  report.check(
    tagDialogText.includes("账号 A") && tagDialogText.includes("根画布"),
    "S3 「标签：重要」分组列表含「账号 A」与本地化分组名「根画布」",
    tagDialogText.replace(/\s+/g, " ").slice(0, 120),
  );
  report.check(ui.findAttr(await api.uiTree(), "title", "编辑节点") !== null, "S3 列表项含「编辑节点」按钮");
  report.check(
    ui.findAttr(await api.uiTree(), "title", "跳转到节点") !== null,
    "S3 列表项含「跳转到节点」按钮",
  );
  await snap("s3-tag-dialog");
  await api.postInput([api.keyClick(["Escape"])]);
  await waitDialogByTitleGone(`标签：${TAG_IMPORTANT}`);
  await sleep(700);
  report.check(
    tagCloudOpen(await api.uiTree()) &&
      ui.findButton(await api.uiTree(), TAG_IMPORTANT, { exact: true }) !== null,
    "S3 分组列表关闭后标签云仍在并重新加载标签",
  );
  await snap("s3-tag-cloud-reloaded");
  await ensureTagCloud(false);

  report.section("S4 书签列表编辑与画布同步（步骤 S4、F3）");
  await openBookmarkList();
  const bookmarkDialog = dialogByTitle(await api.uiTree(), "书签");
  const bookmarkDialogText = ui.subtreeText(bookmarkDialog);
  report.check(
    bookmarkDialogText.includes("账号 A") && bookmarkDialogText.includes("根画布"),
    "S4 书签列表含「账号 A」与分组名「根画布」",
    bookmarkDialogText.replace(/\s+/g, " ").slice(0, 120),
  );
  await snap("s4-bookmark-list");
  // 嵌套编辑取消书签
  const editButton = await waitForCondition(
    async () =>
      listActionForNode(await api.uiTree(), "账号 A", "编辑节点"),
    { timeout: 6000, interval: 200, label: "「账号 A」行的编辑按钮" },
  );
  await ui.clickNode(editButton);
  await waitDialogByTitle("编辑节点");
  await sleep(600);
  report.check(
    ui.findAttr(await api.uiTree(), "title", "取消书签") !== null,
    "S4 嵌套编辑对话框为已收藏态（title「取消书签」）",
  );
  await clickBookmarkToggle("取消书签", "加入书签");
  await confirmEditDialog();
  const emptyList = await waitTextStableOrNull("暂无书签节点", { timeout: 8000 });
  report.check(emptyList !== null, "S4 编辑保存后书签列表重新加载为空态「暂无书签节点」");
  await snap("s4-bookmark-list-empty");
  await api.postInput([api.keyClick(["Escape"])]);
  await waitDialogByTitleGone("书签");
  await sleep(800);
  await deselect();
  const afterUnbookmarkBlue = await cardBlueCount(cardOf(await api.uiTree(), "账号 A").bounds);
  console.log(`  (pixel) after unbookmark blue = ${afterUnbookmarkBlue}`);
  report.check(
    afterUnbookmarkBlue <= baseBlue + BOOKMARK_BLUE_RESIDUAL_MAX,
    `S4 卡片书签图标消失（蓝色像素回落到基线 +${BOOKMARK_BLUE_RESIDUAL_MAX} 以内，nodeUpdated 同步生效）`,
    `base=${baseBlue} after=${afterUnbookmarkBlue}`,
  );
  await snap("s4-bookmark-icon-gone");
  await openEditDialog("账号 A");
  report.check(
    ui.findAttr(await api.uiTree(), "title", "加入书签") !== null,
    "S4 重开编辑对话框书签按钮为「加入书签」（画布同步后的数据一致）",
  );
  const keptTokens = tagTokens(await api.uiTree());
  report.check(
    keptTokens.filter((token) => token === TAG_IMPORTANT).length === 1,
    "S4 取消书签不影响标签：combobox 仍回填「重要」",
    keptTokens.join("|"),
  );
  await cancelEditDialog();

  report.section("S5 备注 B 加标签与账号 A 重新收藏（步骤 S5）");
  await editBookmarkAndTag("备注 B", { tag: TAG_IMPORTANT });
  report.check(
    cardOf(await api.uiTree(), "备注 B") !== null,
    "S5 「备注 B」保存后仍在根画布",
  );
  await editBookmarkAndTag("账号 A", { bookmark: true });
  await deselect();
  const afterS5Blue = await cardBlueCount(cardOf(await api.uiTree(), "账号 A").bounds);
  report.check(
    afterS5Blue >= baseBlue + BOOKMARK_BLUE_DELTA_MIN,
    `S5 重新收藏后卡片书签图标再次出现（蓝色像素较基线增加 ≥${BOOKMARK_BLUE_DELTA_MIN}）`,
    `base=${baseBlue} after=${afterS5Blue}`,
  );
  await snap("s5-rebookmarked");

  report.section("S6 全局搜索标签匹配（步骤 S6）");
  await clickToolbar("搜索");
  await waitForCondition(async () => searchInput(await api.uiTree()), {
    timeout: 6000,
    interval: 200,
    label: "搜索输入框",
  });
  await ui.clickNode(searchInput(await api.uiTree()));
  await api.postInput([api.typeText(TAG_IMPORTANT)]);
  const candidates = await waitForCondition(
    async () => {
      const items = candidateItems(await api.uiTree());
      return items.length === 2 ? items : null;
    },
    { timeout: 8000, interval: 250, label: "搜索候选出现" },
  );
  const candidateTitles = candidates
    .map((item) => {
      const hit = ui
        .flatten([item])
        .find(({ node }) => node.tag === "#text" && (node.text ?? "").trim() !== "");
      return (hit?.node.text ?? "").trim();
    })
    .sort();
  report.check(
    candidateTitles.join(",") === ["备注 B", "账号 A"].sort().join(","),
    "S6 搜索「重要」经标签匹配命中「账号 A」与「备注 B」",
    candidateTitles.join(","),
  );
  const candidateSubtitles = candidates.map((item) => ui.subtreeText(item));
  report.check(
    candidateSubtitles.every((text) => text.includes("根画布")),
    "S6 候选副标题中根画布显示本地化「根画布」（fixture 库内名为 root）",
    candidateSubtitles.join(" | "),
  );
  await snap("s6-search-tag-match");
  await clickToolbar("关闭搜索");
  await sleep(700);

  report.section("S7 子画布与标签字号分档（步骤 S7）");
  await dragCreateNode("拖拽创建画布数据节点", CHILD_DROP);
  await waitForCondition(async () => cardOf(await api.uiTree(), "新画布"), {
    timeout: 10000,
    interval: 250,
    label: "「新画布」卡片",
  });
  await sleep(600);
  await ui.clickText("新画布", { count: 2 });
  await waitForCondition(
    async () => {
      const crumbs = crumbText(await api.uiTree());
      return crumbs[crumbs.length - 1] === "新画布" ? true : null;
    },
    { timeout: 12000, interval: 250, label: "进入子画布" },
  );
  await sleep(900);
  report.check(
    crumbText(await api.uiTree()).join("/") === "画布宇宙/根画布/新画布",
    "S7 双击「新画布」进入子画布（面包屑为「画布宇宙 / 根画布 / 新画布」）",
    crumbText(await api.uiTree()).join("/"),
  );
  await snap("s7-child-canvas");
  await dragCreateNode("拖拽创建数据节点", CHILD_NODE_DROP);
  await waitForCondition(async () => cardOf(await api.uiTree(), "新节点"), {
    timeout: 10000,
    interval: 250,
    label: "「新节点」卡片",
  });
  await sleep(600);
  await editBookmarkAndTag("新节点", { bookmark: true, tag: TAG_SECONDARY });
  await ui.clickText("根画布");
  await waitForCondition(
    async () => {
      const crumbs = crumbText(await api.uiTree());
      return crumbs[crumbs.length - 1] === "根画布" ? true : null;
    },
    { timeout: 12000, interval: 250, label: "返回根画布" },
  );
  await sleep(800);
  await ensureTagCloud(true);
  await waitTagButton(TAG_SECONDARY);
  await sleep(500);
  const cloudTwoTree = await api.uiTree();
  const importantButton = ui.findButton(cloudTwoTree, TAG_IMPORTANT, { exact: true });
  const secondaryButton = ui.findButton(cloudTwoTree, TAG_SECONDARY, { exact: true });
  report.check(
    importantButton !== null && secondaryButton !== null,
    "S7 标签云同时显示「重要」与「次要」",
  );
  report.check(
    importantButton !== null &&
      secondaryButton !== null &&
      importantButton.bounds.height >= secondaryButton.bounds.height + TAG_FONT_HEIGHT_DELTA_MIN,
    `S7 字号按节点数分档：「重要」（计数 2）按钮高度明显大于「次要」（计数 1，差 ≥${TAG_FONT_HEIGHT_DELTA_MIN} 物理像素）`,
    `重要=${importantButton?.bounds.height} 次要=${secondaryButton?.bounds.height}`,
  );
  await snap("s7-tag-cloud-two-tags");
  await ensureTagCloud(false);

  report.section("S8 书签列表分组与跳转居中（步骤 S8）");
  await openBookmarkList();
  const groupDialog = dialogByTitle(await api.uiTree(), "书签");
  const groupText = ui.subtreeText(groupDialog);
  report.check(
    ["根画布", "新画布", "账号 A", "新节点"].every((text) => groupText.includes(text)),
    "S8 书签列表包含「根画布」「新画布」两个分组及两个书签节点（账号 A、新节点）",
    groupText.replace(/\s+/g, " ").slice(0, 160),
  );
  report.check(
    !groupText.includes("备注 B"),
    "S8 仅有标签未收藏的「备注 B」不出现在书签列表",
  );
  const groupNames = ["根画布", "新画布"];
  const groupTops = Object.fromEntries(
    groupNames.map((name) => [name, textNode([groupDialog], name)?.bounds.top ?? Infinity]),
  );
  const actualOrder = [...groupNames].sort((a, b) => groupTops[a] - groupTops[b]);
  const expectedOrder = [...groupNames].sort((a, b) => a.localeCompare(b));
  report.check(
    actualOrder.join(",") === expectedOrder.join(","),
    "S8 组间按画布名升序排列（与 localeCompare 结果一致）",
    `actual=${actualOrder.join(",")} expected=${expectedOrder.join(",")}`,
  );
  await snap("s8-bookmark-groups");
  const jumpButton = await waitForCondition(
    async () => listActionForNode(await api.uiTree(), "新节点", "跳转到节点"),
    { timeout: 6000, interval: 200, label: "「新节点」行的跳转按钮" },
  );
  await ui.clickNode(jumpButton);
  await waitForCondition(
    async () => {
      const crumbs = crumbText(await api.uiTree());
      return crumbs[crumbs.length - 1] === "新画布" ? true : null;
    },
    { timeout: 12000, interval: 250, label: "跳转到子画布" },
  );
  report.check(
    await waitDialogByTitleGone("书签", { timeout: 6000 })
      .then(() => true)
      .catch(() => false),
    "S8 跳转后书签列表对话框关闭",
  );
  const stableNode = await waitNodePositionStable("新节点");
  const center = ui.boundsCenter(stableNode);
  const region = { left: 0, top: 45, width: info.width, height: info.height - 45 };
  const regionCenter = {
    x: region.left + region.width / 2,
    y: region.top + region.height / 2,
  };
  report.check(
    Math.abs(center.x - regionCenter.x) <= region.width * CENTER_TOLERANCE &&
      Math.abs(center.y - regionCenter.y) <= region.height * CENTER_TOLERANCE,
    `S8 跳转后「新节点」居中（画布区域中心 ±${CENTER_TOLERANCE * 100}%）`,
    `center=(${center.x.toFixed(0)},${center.y.toFixed(0)}) regionCenter=(${regionCenter.x.toFixed(0)},${regionCenter.y.toFixed(0)})`,
  );
  await snap("s8-jump-centered");

  report.section("S9 日志的标签与书签详情（步骤 S9）");
  await openLogDialog();
  report.check(logDialog(await api.uiTree()) !== null, "S9 打开日志对话框");
  await filterLogs("账号");
  const accountDetail1 = await waitTextStableOrNull(
    "修改节点“账号 A”的标签：新增 重要；移除 —",
    { timeout: 8000 },
  );
  report.check(accountDetail1 !== null, "S9 存在标签日志详情「修改节点“账号 A”的标签：新增 重要；移除 —」");
  report.check(ui.hasText(await api.uiTree(), "修改节点标签"), "S9 标签日志名称显示「修改节点标签」");
  const accountDetail2 = await waitTextStableOrNull("将节点“账号 A”加入书签", { timeout: 8000 });
  report.check(accountDetail2 !== null, "S9 存在书签日志详情「将节点“账号 A”加入书签」");
  const accountDetail3 = await waitTextStableOrNull("取消节点“账号 A”的书签", { timeout: 8000 });
  report.check(accountDetail3 !== null, "S9 存在书签日志详情「取消节点“账号 A”的书签」");
  report.check(ui.hasText(await api.uiTree(), "修改节点书签"), "S9 书签日志名称显示「修改节点书签」");
  await snap("s9-log-account");
  await filterLogs("备注");
  const noteDetail = await waitTextStableOrNull(
    "修改节点“备注 B”的标签：新增 重要；移除 —",
    { timeout: 8000 },
  );
  report.check(noteDetail !== null, "S9 存在标签日志详情「修改节点“备注 B”的标签：新增 重要；移除 —」");
  await snap("s9-log-note");
  await filterLogs("新节点");
  const childDetail = await waitTextStableOrNull(
    "修改节点“新节点”的标签：新增 次要；移除 —",
    { timeout: 8000 },
  );
  report.check(childDetail !== null, "S9 存在标签日志详情「修改节点“新节点”的标签：新增 次要；移除 —」");
  const childBookmark = await waitTextStableOrNull("将节点“新节点”加入书签", { timeout: 8000 });
  report.check(childBookmark !== null, "S9 存在书签日志详情「将节点“新节点”加入书签」");
  await snap("s9-log-child");
  const logDialogNode = logDialog(await api.uiTree());
  await ui.clickNode(ui.findButton(await api.uiTree(), "关闭", { root: logDialogNode, exact: true }));
  await waitDialogByTitleGone("日志");
  await sleep(500);
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
