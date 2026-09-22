// case_016 快捷键复制粘贴节点。
//
// 流程：复制 base fixture → 启动应用 → 解锁（lastScene 恢复根画布）
// → S1 画布数据节点过滤 + 空剪贴板无操作（模板面板拖拽创建「新画布」并选中复制粘贴）
// → S2 单选复制粘贴（副本落于鼠标位置附近且自动选中）
// → S3 多选复制粘贴（Ctrl+点击两个源节点，副本保持相对布局）
// → S4 换落点重复粘贴（避免副本完全重叠被 UI 树遮挡过滤）
// → S5 门控-对话框、S6 门控-模板面板、S7 门控-回收站面板、S8 门控-输入框焦点、
//   S9 门控-边右键菜单（阴性断言按「打开前集合 → 关闭遮罩后集合一致」表达）
// → S10 跨画布粘贴（子画布副本归属子画布，返回根画布集合不变）
// → S11 源删除后粘贴（逻辑删除与物理删除均跳过、无错误提示）
// → S12 回归悬浮「复制节点」按钮（副本落于画布容器中心）→ 输出 PASS/FAIL 报告。
//
// 脚本规范要点：
// - 断言优先使用 UI 树（role / name / attrs / bounds / states）；vue-flow 节点卡片不写
//   aria-selected，选中态用截图像素断言（卡片上边框为 --v-theme-primary 蓝色系，沿用
//   case_004 的 isSelectionBorder 判据）；
// - 节点卡片与边元素都带 role=group 与 data-id，卡片集合按 aria-roledescription="node" 过滤；
// - 完全重叠的卡片会因遮挡过滤从 UI 树剔除，故重复粘贴一律换落点（S4）；
// - 对话框与边右键菜单的遮罩会把画布卡片从 UI 树过滤，门控断言在关闭遮罩后比对集合；
// - 阴性断言用「集合相等 + 静止复查」的等待函数表达，不以固定 sleep 单独判定；
// - Ctrl+C / Ctrl+V 用 key_click(["Control","c"|"v"]) 注入；Ctrl+点击多选必须在同一
//   /input 请求内 key_press → 移动 → 点击 → key_release（请求间按键不保持）；
// - 应用生命周期由 withApp 包装，保证无论成败都经 POST /shutdown 收尾；
// - 关键步骤截图存 output；流程中断时额外截取 fatal 截图并写入 report.json。
//
// 运行方式：在项目根目录执行 `node e2e\script\case_016\case_016.js`

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

/** 画布数据节点（子画布）的拖拽创建落点（窗口物理像素） */
const CHILD_DROP = { x: 350, y: 400 };
/** S2 单选粘贴的鼠标落点（画布右下空白区） */
const P1 = { x: 900, y: 950 };
/** S3 多选粘贴的鼠标落点（画布左下空白区） */
const P2 = { x: 400, y: 950 };
/** S4 重复粘贴的鼠标落点（画布左上空白区，与既有卡片互不重叠） */
const P4 = { x: 370, y: 580 };
/** S10 子画布内的粘贴落点 */
const P3 = { x: 800, y: 600 };
/** 点击空白处取消选中并复位的坐标（窗口物理像素） */
const BLANK_POINT = { x: 200, y: 300 };
/** 悬停前的清场坐标（窗口物理像素） */
const NEUTRAL_POINT = { x: 1200, y: 300 };

/** 连接桩检索的卡片外扩范围（窗口物理像素，沿用 case_006） */
const HANDLE_SEARCH_MARGIN = 45;
/** 边右键探测的卡片危险区外扩（窗口物理像素，沿用 case_006） */
const CARD_SIDE_MARGIN = 12;
/** 边右键探测的卡片顶部危险区外扩（窗口物理像素，沿用 case_006） */
const CARD_TOP_MARGIN = 14;

/** 副本中心与鼠标落点的距离上限（物理像素；20px 网格取整误差 ≤15 + 测量余量） */
const DROP_DISTANCE_MAX = 60;
/** 副本相对布局的逐轴位移差上限（物理像素；逐轴取整误差 ≤1.5 + 测量余量） */
const LAYOUT_DELTA_MAX = 4;
/** 副本与视口中心的最小距离（物理像素；用于排除「鼠标不在容器内回退中心」语义） */
const CENTER_EXCLUSION_MIN = 150;
/** 复制按钮回归的窗口中心容差（物理像素，沿用 case_004 契约） */
const COPY_BUTTON_CENTER_TOLERANCE = { x: 200, y: 250 };

const report = new Report("case_016 快捷键复制粘贴节点");

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
 * 按 data-id 查找节点卡片。
 * @param {object[]} tree UI 树顶层节点数组。
 * @param {string} id 节点 id。
 * @returns {object|null} 卡片；未命中时为 null。
 */
function cardById(tree, id) {
  return cards(tree).find((c) => c.attrs["data-id"] === id) ?? null;
}

/**
 * 按标题文本查找最近的节点卡片（同名卡片取与首个标题文本节点最近者）。
 * @param {object[]} tree UI 树顶层节点数组。
 * @param {string} text 节点标题。
 * @returns {object|null} 卡片；未命中时为 null。
 */
function cardOf(tree, text) {
  const titles = ui
    .flatten(tree)
    .filter(({ node }) => node.tag === "#text" && (node.text ?? "").trim() === text);
  if (titles.length === 0) {
    return null;
  }
  const title = titles[0].node;
  const candidates = cards(tree).filter((card) =>
    ui
      .flatten([card])
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
 * 读取卡片子树中的首个可见文本（即卡片标题）。
 * @param {object} card 卡片节点。
 * @returns {string} 标题文本；无文本时为空串。
 */
function cardTitle(card) {
  const hit = ui
    .flatten([card])
    .find(({ node }) => node.tag === "#text" && (node.text ?? "").trim() !== "");
  return hit ? hit.node.text.trim() : "";
}

/**
 * 返回当前画布全部卡片 id 的有序列表（用于集合相等比较）。
 * @param {object[]} tree UI 树顶层节点数组。
 * @returns {string[]} 升序排列的卡片 id 数组。
 */
function cardKeys(tree) {
  return cards(tree)
    .map((c) => c.attrs["data-id"])
    .sort();
}

/**
 * 返回卡片 id 集合。
 * @param {object[]} tree UI 树顶层节点数组。
 * @returns {Set<string>} 卡片 id 集合。
 */
function cardIdSet(tree) {
  return new Set(cards(tree).map((c) => c.attrs["data-id"]));
}

/**
 * 返回当前画布中不在基准集合内的新卡片。
 * @param {Set<string>} beforeSet 基准卡片 id 集合。
 * @param {object[]} tree 当前 UI 树顶层节点数组。
 * @returns {object[]} 新卡片数组。
 */
function newCards(beforeSet, tree) {
  return cards(tree).filter((c) => !beforeSet.has(c.attrs["data-id"]));
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
 * 返回面包屑条目（顶部条内 listitem），按 left 升序。
 * @param {object[]} tree UI 树顶层节点数组。
 * @returns {object[]} 面包屑条目数组。
 */
function crumbs(tree) {
  return ui
    .flatten(tree)
    .map((entry) => entry.node)
    .filter((node) => node.role === "listitem" && node.bounds.top < 140)
    .sort((a, b) => a.bounds.left - b.bounds.left);
}

/**
 * 返回面包屑各级文案数组（根画布在首、当前画布在尾）。
 * @param {object[]} tree UI 树顶层节点数组。
 * @returns {string[]} 面包屑文案数组。
 */
function crumbText(tree) {
  return crumbs(tree).map((item) => ui.subtreeText(item).trim());
}

/**
 * 查找全局搜索输入框（placeholder 为「搜索节点…」）。
 * @param {object[]} tree UI 树顶层节点数组。
 * @returns {object|null} 输入框节点；未命中时为 null。
 */
function searchInput(tree) {
  return (
    ui
      .flatten(tree)
      .map((entry) => entry.node)
      .find((node) => node.editable && node.attrs?.placeholder === "搜索节点…") ?? null
  );
}

// ---------- 等待 ----------

/**
 * 等待当前画布卡片数量达到期望值。
 * @param {number} expected 期望数量。
 * @param {object} [options] 可选参数。
 * @param {number} [options.timeout=8000] 超时毫秒数。
 * @returns {Promise<boolean>} 达到期望时为 true；超时为 false。
 */
async function waitCardCount(expected, { timeout = 8000 } = {}) {
  return waitForCondition(
    async () => (cards(await api.uiTree()).length === expected ? true : null),
    { timeout, interval: 200, label: `card count == ${expected}` },
  )
    .then(() => true)
    .catch(() => false);
}

/**
 * 等待当前画布新增指定数量的卡片。
 * @param {Set<string>} beforeSet 基准卡片 id 集合。
 * @param {number} delta 期望新增数量。
 * @param {object} [options] 可选参数。
 * @param {number} [options.timeout=8000] 超时毫秒数。
 * @returns {Promise<boolean>} 达到期望时为 true；超时为 false。
 */
async function waitNewCardCount(beforeSet, delta, { timeout = 8000 } = {}) {
  return waitForCondition(
    async () => (newCards(beforeSet, await api.uiTree()).length === delta ? true : null),
    { timeout, interval: 200, label: `${delta} new cards` },
  )
    .then(() => true)
    .catch(() => false);
}

/**
 * 等待当前画布卡片集合等于期望集合，并在静止期后复查仍相等。
 * 用于阴性断言（快捷键未生效时集合保持不变）。
 * @param {string} expectedKey cardKeys 生成的期望集合键（逗号分隔）。
 * @param {object} [options] 可选参数。
 * @param {number} [options.timeout=8000] 超时毫秒数。
 * @param {number} [options.settleMs=600] 首次命中后的静止等待毫秒数。
 * @returns {Promise<boolean>} 静止复查后仍相等时为 true。
 */
async function waitCardSetStable(expectedKey, { timeout = 8000, settleMs = 600 } = {}) {
  const hit = await waitForCondition(
    async () => (cardKeys(await api.uiTree()).join(",") === expectedKey ? true : null),
    { timeout, interval: 250, label: `card set stable` },
  )
    .then(() => true)
    .catch(() => false);
  if (!hit) {
    return false;
  }
  await sleep(settleMs);
  return cardKeys(await api.uiTree()).join(",") === expectedKey;
}

// ---------- 像素 ----------

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
 * 截取指定卡片上边框的像素样本（卡片左起 1/4 处）。
 * @param {object[]} cardsToSample 卡片数组。
 * @returns {Promise<object[]>} 像素分量数组。
 */
async function borderPixels(cardsToSample) {
  const png = image.decodePng(await api.screenshot());
  return cardsToSample.map((card) =>
    pixelAt(
      png,
      Math.round(card.bounds.left + card.bounds.width / 4),
      Math.round(card.bounds.top) + 1,
    ),
  );
}

/**
 * 计算两个坐标的欧氏距离。
 * @param {{x: number, y: number}} a 坐标 A。
 * @param {{x: number, y: number}} b 坐标 B。
 * @returns {number} 距离（物理像素）。
 */
function dist(a, b) {
  return Math.hypot(a.x - b.x, a.y - b.y);
}

// ---------- 通用操作 ----------

/**
 * 等待首页就绪并确保界面语言为中文：首页未出现中文名称输入框时，
 * 通过右上角「切换语言」菜单切回「简体中文」。
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
 * 点击画布空白处取消选中并复位 hover 状态。
 * @returns {Promise<void>} 无返回值。
 */
async function deselect() {
  await api.postInput([api.mouseMove(BLANK_POINT.x, BLANK_POINT.y), api.mouseClick("left", 1)]);
  await sleep(650);
}

/**
 * 把鼠标移到清场坐标并等待 hover 状态（工具条/提示）退场。
 * @returns {Promise<void>} 无返回值。
 */
async function clearHover() {
  await api.postInput([api.mouseMove(NEUTRAL_POINT.x, NEUTRAL_POINT.y)]);
  await sleep(450);
}

/**
 * 点击卡片中心（可双击）。
 * @param {object} card 卡片节点（坐标应为当前界面下的新鲜值）。
 * @param {number} [count=1] 点击次数。
 * @returns {Promise<void>} 无返回值。
 */
async function clickCard(card, count = 1) {
  await ui.clickNode(card, { count });
  await sleep(450);
}

/**
 * 按住 Ctrl 点击卡片（多选；按下与松开必须在同一请求内完成）。
 * @param {object} card 卡片节点（坐标应为当前界面下的新鲜值）。
 * @returns {Promise<void>} 无返回值。
 */
async function ctrlClickCard(card) {
  const center = ui.boundsCenter(card);
  await api.postInput([
    api.keyPress(["Control"]),
    api.mouseMove(center.x, center.y),
    api.mouseClick("left", 1),
    api.keyRelease(["Control"]),
  ]);
  await sleep(450);
}

/**
 * 发送 Ctrl+C（复制选中节点到应用内存剪贴板）。
 * @returns {Promise<void>} 无返回值。
 */
async function ctrlC() {
  await api.postInput([api.keyClick(["Control", "c"])]);
  await sleep(350);
}

/**
 * 发送 Ctrl+V（可选先把鼠标移到粘贴落点）。
 * @param {{x: number, y: number}|null} [point=null] 鼠标落点；null 表示不移动。
 * @returns {Promise<void>} 无返回值。
 */
async function ctrlV(point = null) {
  const commands = [];
  if (point) {
    commands.push(api.mouseMove(point.x, point.y), api.wait(200));
  }
  commands.push(api.keyClick(["Control", "v"]));
  await api.postInput(commands);
}

/**
 * 悬停卡片中心（先清场避免其它卡片的悬浮按钮残留）。
 * @param {string} cardId 目标卡片 id。
 * @returns {Promise<void>} 无返回值。
 */
async function hoverCardById(cardId) {
  await clearHover();
  const card = cardById(await api.uiTree(), cardId);
  if (!card) {
    throw new Error(`hoverCardById: card ${cardId} not found`);
  }
  const center = ui.boundsCenter(card);
  await api.postInput([api.mouseMove(center.x, center.y)]);
  await sleep(550);
}

/**
 * 在 UI 树中查找目标卡片正上方的悬浮操作按钮。
 * @param {object[]} tree UI 树顶层节点数组。
 * @param {string} cardId 目标卡片 id。
 * @param {string} actionTitle 按钮 title。
 * @returns {object|null} 按钮节点；未命中时为 null。
 */
function findActionNearCard(tree, cardId, actionTitle) {
  const card = cardById(tree, cardId);
  if (!card) {
    return null;
  }
  return (
    ui
      .flatten(tree)
      .map((entry) => entry.node)
      .find(
        (n) =>
          n.attrs?.title === actionTitle &&
          n.bounds.left + n.bounds.width >= card.bounds.left - 40 &&
          n.bounds.left <= card.bounds.left + card.bounds.width + 40 &&
          n.bounds.top >= card.bounds.top - 130 &&
          n.bounds.top + n.bounds.height <= card.bounds.top + 10,
      ) ?? null
  );
}

/**
 * 悬停指定卡片并点击其悬浮操作按钮。
 * @param {string} cardId 目标卡片 id。
 * @param {string} actionTitle 按钮 title（如「删除节点」「复制节点」）。
 * @returns {Promise<void>} 无返回值。
 */
async function clickCardAction(cardId, actionTitle) {
  await hoverCardById(cardId);
  await waitForCondition(
    async () => findActionNearCard(await api.uiTree(), cardId, actionTitle),
    { timeout: 6000, interval: 200, label: `button "${actionTitle}" near ${cardId}` },
  );
  await sleep(250);
  const fresh = await api.uiTree();
  const button = findActionNearCard(fresh, cardId, actionTitle);
  if (!button) {
    throw new Error(`clickCardAction: button "${actionTitle}" lost for card ${cardId}`);
  }
  await ui.clickNode(button);
}

/**
 * 展开模板面板（已展开时直接返回）。
 * @returns {Promise<void>} 无返回值。
 */
async function openTemplatePanel() {
  const tree = await api.uiTree();
  if (ui.findAttr(tree, "title", "拖拽创建数据节点")) {
    return;
  }
  const button = await waitForCondition(
    async () => ui.findAttr(await api.uiTree(), "title", "新建节点"),
    { timeout: 8000, interval: 200, label: "「新建节点」按钮" },
  );
  await ui.clickNode(button);
  await waitForCondition(
    async () => ui.findAttr(await api.uiTree(), "title", "拖拽创建数据节点"),
    { timeout: 8000, interval: 200, label: "模板面板展开" },
  );
  await sleep(500);
}

/**
 * 收起模板面板并等待手柄消失。
 * @returns {Promise<void>} 无返回值。
 */
async function closeTemplatePanel() {
  const button = await waitForCondition(
    async () => ui.findAttr(await api.uiTree(), "title", "新建节点"),
    { timeout: 8000, interval: 200, label: "「新建节点」按钮" },
  );
  await ui.clickNode(button);
  await waitForCondition(
    async () => ui.findAttr(await api.uiTree(), "title", "拖拽创建数据节点") === null,
    { timeout: 6000, interval: 200, label: "模板面板关闭" },
  );
  await sleep(400);
}

/**
 * 从模板面板拖拽指定手柄到画布落点创建节点，并等待面板自动关闭。
 * @param {string} handleTitle 拖拽手柄的 title。
 * @param {{x: number, y: number}} drop 落点（窗口物理像素）。
 * @returns {Promise<void>} 无返回值。
 */
async function dragCreateNode(handleTitle, drop) {
  await openTemplatePanel();
  const handle = await waitForCondition(
    async () => ui.findAttr(await api.uiTree(), "title", handleTitle),
    { timeout: 8000, interval: 200, label: `模板手柄「${handleTitle}」` },
  );
  await api.postInput([api.mouseDrag(ui.boundsCenter(handle), drop)]);
  await waitForCondition(
    async () => ui.findAttr(await api.uiTree(), "title", handleTitle) === null,
    { timeout: 6000, interval: 200, label: "模板面板自动关闭" },
  );
  await sleep(800);
}

/**
 * 确保回收站面板处于打开状态（已打开时不重复点击，避免 toggle 关闭）。
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
 * 关闭回收站面板（未打开时不操作）。
 * @returns {Promise<void>} 无返回值。
 */
async function closeRecycleBinPanel() {
  const tree = await api.uiTree();
  if (ui.findButton(tree, "清空回收站")) {
    await ui.clickNode(ui.findAttr(tree, "title", "回收站"));
    await waitForCondition(
      async () => !ui.findButton(await api.uiTree(), "清空回收站"),
      { timeout: 6000, interval: 200, label: "回收站面板关闭" },
    );
    await sleep(400);
  }
}

// ---------- 边右键菜单辅助（沿用 case_006 的贝塞尔几何与危险区逻辑） ----------

/**
 * 收集指定卡片附近的连接桩元素。
 * @param {object[]} tree UI 树顶层节点数组。
 * @param {object} card 节点卡片。
 * @returns {object[]} 连接桩节点数组。
 */
function handlesNear(tree, card) {
  const margin = HANDLE_SEARCH_MARGIN;
  return ui
    .flatten(tree)
    .map((entry) => entry.node)
    .filter(
      (node) =>
        node.attrs?.["data-handlepos"] &&
        node.bounds &&
        node.bounds.width > 0 &&
        node.bounds.left + node.bounds.width > card.bounds.left - margin &&
        node.bounds.left < card.bounds.left + card.bounds.width + margin &&
        node.bounds.top + node.bounds.height > card.bounds.top - margin &&
        node.bounds.top < card.bounds.top + card.bounds.height + margin,
    );
}

/**
 * 取指定卡片指定方向的连接桩。
 * @param {object[]} tree UI 树顶层节点数组。
 * @param {object} card 节点卡片。
 * @param {string} pos 连接桩方向（top/right/bottom/left）。
 * @returns {object|null} 连接桩；未命中时为 null。
 */
function handleOf(tree, card, pos) {
  return handlesNear(tree, card).filter((h) => h.attrs["data-handlepos"] === pos)[0] ?? null;
}

/**
 * 按应用几何公式构造三次贝塞尔曲线的四个点（控制点沿 handle 轴向偏移 |dx|/2）。
 * @param {{x: number, y: number}} p0 源连接桩中心。
 * @param {{x: number, y: number}} p1 目标连接桩中心。
 * @param {string} sh 源 handle 方向。
 * @param {string} th 目标 handle 方向。
 * @returns {{p0: object, cp1: object, cp2: object, p1: object}} 贝塞尔几何点。
 */
function bezPoints(p0, p1, sh, th) {
  const d = Math.abs(p1.x - p0.x) * 0.5;
  const cp1 = { x: p0.x, y: p0.y };
  if (sh === "right") cp1.x += d;
  else if (sh === "left") cp1.x -= d;
  else if (sh === "bottom") cp1.y += d;
  else if (sh === "top") cp1.y -= d;
  const cp2 = { x: p1.x, y: p1.y };
  if (th === "right") cp2.x += d;
  else if (th === "left") cp2.x -= d;
  else if (th === "bottom") cp2.y += d;
  else if (th === "top") cp2.y -= d;
  return { p0, cp1, cp2, p1 };
}

/**
 * 计算三次贝塞尔曲线在参数 t 处的坐标。
 * @param {{p0: object, cp1: object, cp2: object, p1: object}} pts 贝塞尔几何点。
 * @param {number} t 参数（0..1）。
 * @returns {{x: number, y: number}} 坐标（窗口物理像素）。
 */
function bezAt(pts, t) {
  const { p0, cp1, cp2, p1 } = pts;
  const mt = 1 - t;
  const a = mt * mt * mt;
  const b = 3 * mt * mt * t;
  const c = 3 * mt * t * t;
  const d = t * t * t;
  return {
    x: a * p0.x + b * cp1.x + c * cp2.x + d * p1.x,
    y: a * p0.y + b * cp1.y + c * cp2.y + d * p1.y,
  };
}

/**
 * 计算指定两端卡片与连接桩方向对应的边贝塞尔几何点。
 * @param {object[]} tree UI 树顶层节点数组。
 * @param {{sourceId: string, sourcePos: string, targetId: string, targetPos: string}} spec 边端点描述。
 * @returns {object} 贝塞尔几何点。
 */
function edgeMidpoint(tree, spec) {
  const sCard = cardById(tree, spec.sourceId);
  const tCard = cardById(tree, spec.targetId);
  if (!sCard || !tCard) {
    throw new Error(`edgeMidpoint: card not found ${JSON.stringify(spec)}`);
  }
  const sH = handleOf(tree, sCard, spec.sourcePos);
  const tH = handleOf(tree, tCard, spec.targetPos);
  if (!sH || !tH) {
    throw new Error(`edgeMidpoint: handle not found ${JSON.stringify(spec)}`);
  }
  return bezPoints(ui.boundsCenter(sH), ui.boundsCenter(tH), spec.sourcePos, spec.targetPos);
}

/**
 * 生成全部节点卡片的右键危险区（卡片外扩）。
 * @param {object[]} tree UI 树顶层节点数组。
 * @returns {{left: number, top: number, right: number, bottom: number}[]} 危险区矩形数组。
 */
function dangerRects(tree) {
  return cards(tree).map((card) => ({
    left: card.bounds.left - CARD_SIDE_MARGIN,
    top: card.bounds.top - CARD_TOP_MARGIN,
    right: card.bounds.left + card.bounds.width + CARD_SIDE_MARGIN,
    bottom: card.bounds.top + card.bounds.height + CARD_SIDE_MARGIN,
  }));
}

/**
 * 判断点是否位于全部危险区之外。
 * @param {{x: number, y: number}} point 点坐标。
 * @param {{left: number, top: number, right: number, bottom: number}[]} rects 危险区数组。
 * @returns {boolean} 位于全部危险区之外时为 true。
 */
function pointClear(point, rects) {
  return rects.every(
    (r) => point.x < r.left || point.x > r.right || point.y < r.top || point.y > r.bottom,
  );
}

/**
 * 在边曲线上挑选右键探测点：优先取曲线上远离全部节点卡片的点，无净空点时取净空最大者。
 * @param {object} pts 贝塞尔几何点。
 * @param {{left: number, top: number, right: number, bottom: number}[]} rects 危险区数组。
 * @returns {{point: {x: number, y: number}, t: number}} 探测点信息。
 */
function pickProbePoint(pts, rects) {
  const candidates = [
    0.5, 0.46, 0.54, 0.42, 0.58, 0.38, 0.62, 0.34, 0.66, 0.3, 0.7, 0.26, 0.74, 0.22, 0.78,
    0.18, 0.82,
  ];
  let best = null;
  for (const t of candidates) {
    const point = bezAt(pts, t);
    const clearance = Math.min(
      ...rects.map((r) => Math.max(r.left - point.x, point.x - r.right, r.top - point.y, point.y - r.bottom)),
    );
    if (pointClear(point, rects)) {
      return { point, t, clearance };
    }
    if (!best || clearance > best.clearance) {
      best = { point, t, clearance };
    }
  }
  return best;
}

/**
 * 判断边右键菜单是否可见（以菜单项「编辑边」为锚点）。
 * @returns {Promise<boolean>} 可见时为 true。
 */
async function edgeMenuVisible() {
  return ui.findByText(await api.uiTree(), "编辑边", { exact: true }) !== null;
}

/**
 * 在边的曲线上弹出右键菜单（带重试，规避后端写入与渲染延迟）。
 * @param {{sourceId: string, sourcePos: string, targetId: string, targetPos: string}} spec 边端点描述。
 * @returns {Promise<boolean>} 成功弹出时为 true；超时为 false。
 */
async function openEdgeMenu(spec) {
  return waitForCondition(
    async () => {
      const tree = await api.uiTree();
      const pts = edgeMidpoint(tree, spec);
      const pick = pickProbePoint(pts, dangerRects(tree));
      await clearHover();
      await api.postInput([api.mouseMove(pick.point.x, pick.point.y)]);
      await sleep(380);
      await api.postInput([api.mouseMove(pick.point.x, pick.point.y), api.mouseClick("right", 1)]);
      await sleep(450);
      if (await edgeMenuVisible()) {
        console.log(
          `  (edge menu) t=${pick.t.toFixed(2)} @ (${Math.round(pick.point.x)},${Math.round(pick.point.y)})`,
        );
        return true;
      }
      return null;
    },
    { timeout: 10000, interval: 500, label: "边右键菜单打开" },
  )
    .then(() => true)
    .catch(() => false);
}

/**
 * 关闭边右键菜单（Esc）并等待其消失。
 * @returns {Promise<void>} 无返回值。
 */
async function closeEdgeMenu() {
  if (!(await edgeMenuVisible())) {
    return;
  }
  await api.postInput([api.keyClick(["Escape"])]);
  await waitForCondition(async () => !(await edgeMenuVisible()), {
    timeout: 4000,
    interval: 150,
    label: "边右键菜单关闭",
  });
  await sleep(400);
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
  const unlocked = await api.uiTree();
  const accountCard = cardOf(unlocked, "账号 A");
  const noteCard = cardOf(unlocked, "备注 B");
  report.check(accountCard !== null && noteCard !== null, "解锁后根画布出现「账号 A」与「备注 B」");
  report.check(
    crumbText(unlocked).join("/") === "画布宇宙/根画布",
    "按 lastScene 恢复场景（面包屑为「画布宇宙 / 根画布」）",
    crumbText(unlocked).join("/"),
  );
  const accountId = accountCard.attrs["data-id"];
  const noteId = noteCard.attrs["data-id"];
  await snap("unlocked-root-canvas");

  report.section("S1 画布数据节点过滤与空剪贴板无操作（步骤 S1、F7）");
  await dragCreateNode("拖拽创建画布数据节点", CHILD_DROP);
  const childCard = await waitForCondition(async () => cardOf(await api.uiTree(), "新画布"), {
    timeout: 10000,
    interval: 250,
    label: "「新画布」卡片",
  });
  report.check(childCard !== null, "S1 模板面板拖拽创建画布数据节点「新画布」");
  const childId = childCard.attrs["data-id"];
  const s1BeforeKey = cardKeys(await api.uiTree()).join(",");
  await clickCard(cardById(await api.uiTree(), childId));
  const s1Pixels = await borderPixels([cardById(await api.uiTree(), childId)]);
  report.check(
    s1Pixels.length === 1 && isSelectionBorder(s1Pixels[0]),
    "S1 单击选中「新画布」画布数据节点（卡片上边框为选中主色）",
    JSON.stringify(s1Pixels[0]),
  );
  await ctrlC();
  await ctrlV(P1);
  const s1Stable = await waitCardSetStable(s1BeforeKey);
  const s1Tree = await api.uiTree();
  report.check(
    s1Stable,
    "S1 画布数据节点被过滤且剪贴板为空：Ctrl+V 后卡片集合不变（无操作）",
    `count=${cards(s1Tree).length}`,
  );
  report.check(
    !ui.hasText(s1Tree, "不允许操作画布数据节点"),
    "S1 无「不允许操作画布数据节点」错误提示（前端已过滤，未到达后端拒绝分支）",
  );
  await snap("s1-canvas-data-node-filtered");

  report.section("S2 单选复制粘贴（步骤 S2）");
  await deselect();
  await clickCard(cardById(await api.uiTree(), accountId));
  const s2SourcePixels = await borderPixels([cardById(await api.uiTree(), accountId)]);
  report.check(
    s2SourcePixels.length === 1 && isSelectionBorder(s2SourcePixels[0]),
    "S2 单击选中「账号 A」源卡片（卡片上边框为选中主色）",
    JSON.stringify(s2SourcePixels[0]),
  );
  await ctrlC();
  const s2Before = cardIdSet(await api.uiTree());
  await ctrlV(P1);
  const s2Added = await waitNewCardCount(s2Before, 1);
  await sleep(600);
  const s2Tree = await api.uiTree();
  const s2NewIds = newCards(s2Before, s2Tree).map((c) => c.attrs["data-id"]);
  report.check(s2Added && s2NewIds.length === 1, "S2 Ctrl+V 后根画布新增 1 个卡片", `count=${cards(s2Tree).length}`);
  const s2Copy = s2NewIds.length === 1 ? cardById(s2Tree, s2NewIds[0]) : null;
  report.check(s2Copy !== null && cardTitle(s2Copy) === "账号 A", "S2 新增卡片标题为「账号 A」", s2Copy ? cardTitle(s2Copy) : "none");
  if (s2Copy) {
    const center = ui.boundsCenter(s2Copy);
    report.check(
      dist(center, P1) <= DROP_DISTANCE_MAX,
      `S2 副本中心落在鼠标位置附近（≤${DROP_DISTANCE_MAX} 物理像素，20px 网格取整误差内）`,
      `dist=${Math.round(dist(center, P1))}`,
    );
    report.check(
      dist(center, { x: info.width / 2, y: info.height / 2 }) >= CENTER_EXCLUSION_MIN,
      `S2 副本远离视口中心（≥${CENTER_EXCLUSION_MIN} 物理像素，排除回退中心的语义）`,
      `dist=${Math.round(dist(center, { x: info.width / 2, y: info.height / 2 }))}`,
    );
    const copyPixels = await borderPixels([s2Copy]);
    report.check(
      copyPixels.length === 1 && isSelectionBorder(copyPixels[0]),
      "S2 粘贴后新副本自动选中（卡片上边框为选中主色）",
      JSON.stringify(copyPixels[0]),
    );
  }
  await snap("s2-single-paste");

  report.section("S3 多选复制粘贴与相对布局（步骤 S3）");
  await deselect();
  const s3SourceTree = await api.uiTree();
  const srcAccountBounds = cardById(s3SourceTree, accountId).bounds;
  const srcNoteBounds = cardById(s3SourceTree, noteId).bounds;
  await ctrlClickCard(cardById(await api.uiTree(), accountId));
  await ctrlClickCard(cardById(await api.uiTree(), noteId));
  const s3SelectedTree = await api.uiTree();
  const s3SelectedPixels = await borderPixels([
    cardById(s3SelectedTree, accountId),
    cardById(s3SelectedTree, noteId),
  ]);
  report.check(
    s3SelectedPixels.length === 2 && s3SelectedPixels.every(isSelectionBorder),
    "S3 Ctrl+点击选中「账号 A」与「备注 B」两个源节点（两卡片上边框均为选中主色）",
    JSON.stringify(s3SelectedPixels),
  );
  await ctrlC();
  const s3Before = cardIdSet(s3SelectedTree);
  await ctrlV(P2);
  const s3Added = await waitNewCardCount(s3Before, 2);
  await sleep(600);
  const s3Tree = await api.uiTree();
  const s3NewCards = newCards(s3Before, s3Tree);
  report.check(s3Added && s3NewCards.length === 2, "S3 Ctrl+V 后根画布新增 2 个卡片", `count=${cards(s3Tree).length}`);
  if (s3NewCards.length === 2) {
    const byTitle = new Map(s3NewCards.map((card) => [cardTitle(card), card]));
    report.check(
      byTitle.has("账号 A") && byTitle.has("备注 B"),
      "S3 两个副本标题分别为「账号 A」与「备注 B」",
      [...byTitle.keys()].join("/"),
    );
    if (byTitle.has("账号 A") && byTitle.has("备注 B")) {
      const copyAccount = byTitle.get("账号 A").bounds;
      const copyNote = byTitle.get("备注 B").bounds;
      const srcDx = srcNoteBounds.left - srcAccountBounds.left;
      const srcDy = srcNoteBounds.top - srcAccountBounds.top;
      const copyDx = copyNote.left - copyAccount.left;
      const copyDy = copyNote.top - copyAccount.top;
      report.check(
        Math.abs(copyDx - srcDx) <= LAYOUT_DELTA_MAX &&
          Math.abs(copyDy - srcDy) <= LAYOUT_DELTA_MAX,
        `S3 副本相对布局与源节点一致（位移差 ≤${LAYOUT_DELTA_MAX} 物理像素）`,
        `src=(${srcDx},${srcDy}) copy=(${copyDx},${copyDy})`,
      );
      const left = Math.min(copyAccount.left, copyNote.left);
      const right = Math.max(copyAccount.left + copyAccount.width, copyNote.left + copyNote.width);
      const top = Math.min(copyAccount.top, copyNote.top);
      const bottom = Math.max(copyAccount.top + copyAccount.height, copyNote.top + copyNote.height);
      const boxCenter = { x: (left + right) / 2, y: (top + bottom) / 2 };
      report.check(
        dist(boxCenter, P2) <= DROP_DISTANCE_MAX,
        `S3 副本包围盒中心落在鼠标位置附近（≤${DROP_DISTANCE_MAX} 物理像素）`,
        `dist=${Math.round(dist(boxCenter, P2))}`,
      );
    }
    const copyPixels = await borderPixels(s3NewCards);
    report.check(
      copyPixels.length === 2 && copyPixels.every(isSelectionBorder),
      "S3 两副本均呈选中态（上边框均为选中主色）",
      JSON.stringify(copyPixels),
    );
  }
  await snap("s3-multi-paste");

  report.section("S4 换落点重复粘贴（步骤 S4）");
  const s4Before = cardIdSet(s3Tree);
  await ctrlV(P4);
  const s4Added = await waitNewCardCount(s4Before, 2);
  await sleep(600);
  const s4Tree = await api.uiTree();
  const s4NewCards = newCards(s4Before, s4Tree);
  report.check(s4Added && s4NewCards.length === 2, "S4 再次 Ctrl+V 后根画布再新增 2 个卡片", `count=${cards(s4Tree).length}`);
  if (s4NewCards.length === 2) {
    const byTitle = new Map(s4NewCards.map((card) => [cardTitle(card), card]));
    report.check(
      byTitle.has("账号 A") && byTitle.has("备注 B"),
      "S4 两个副本标题分别为「账号 A」与「备注 B」",
      [...byTitle.keys()].join("/"),
    );
    if (byTitle.has("账号 A") && byTitle.has("备注 B")) {
      const copyAccount = byTitle.get("账号 A").bounds;
      const copyNote = byTitle.get("备注 B").bounds;
      const left = Math.min(copyAccount.left, copyNote.left);
      const right = Math.max(copyAccount.left + copyAccount.width, copyNote.left + copyNote.width);
      const top = Math.min(copyAccount.top, copyNote.top);
      const bottom = Math.max(copyAccount.top + copyAccount.height, copyNote.top + copyNote.height);
      const boxCenter = { x: (left + right) / 2, y: (top + bottom) / 2 };
      report.check(
        dist(boxCenter, P4) <= DROP_DISTANCE_MAX,
        `S4 副本包围盒中心落在新鼠标落点附近（≤${DROP_DISTANCE_MAX} 物理像素）`,
        `dist=${Math.round(dist(boxCenter, P4))}`,
      );
    }
    const copyPixels = await borderPixels(s4NewCards);
    report.check(
      copyPixels.length === 2 && copyPixels.every(isSelectionBorder),
      "S4 两副本均呈选中态（上边框均为选中主色）",
      JSON.stringify(copyPixels),
    );
  }
  await snap("s4-repeat-paste");

  report.section("S5 门控-对话框（F1）");
  await deselect();
  const s5BaseKey = cardKeys(await api.uiTree()).join(",");
  await clickCard(cardById(await api.uiTree(), noteId), 2);
  await ui.waitForDialog("编辑节点", { timeout: 8000 });
  await sleep(600);
  let s5DialogTree = await api.uiTree();
  report.check(ui.findDialog(s5DialogTree, "编辑节点") !== null, "S5 双击「备注 B」打开「编辑节点」对话框（门控前置状态）");
  /** 判断给定 UI 树中是否存在处于聚焦态的可编辑控件 */
  const isAnyEditableFocused = (tree) =>
    ui.flatten(tree).some(({ node }) => node.editable && node.states?.focused === true);
  if (isAnyEditableFocused(s5DialogTree)) {
    await ui.clickText("编辑节点");
    await sleep(400);
    s5DialogTree = await api.uiTree();
  }
  report.check(!isAnyEditableFocused(s5DialogTree), "S5 按键前焦点不在任何输入框（进入对话框门控分支）");
  await ctrlC();
  await ctrlV(null);
  await sleep(900);
  s5DialogTree = await api.uiTree();
  report.check(ui.findDialog(s5DialogTree, "编辑节点") !== null, "S5 Ctrl+C/V 后对话框保持打开（快捷键被门控放行）");
  await snap("s5-dialog-gate");
  const s5DialogNow = ui.findDialog(await api.uiTree(), "编辑节点");
  await ui.clickNode(ui.findButton(await api.uiTree(), "取消", { root: s5DialogNow }));
  await ui.waitForDialogGone("编辑节点", { timeout: 8000 });
  await sleep(700);
  const s5Stable = await waitCardSetStable(s5BaseKey);
  const s5Tree = await api.uiTree();
  report.check(
    s5Stable,
    "S5 关闭对话框后卡片集合与打开前一致（对话框门控生效，未粘贴节点）",
    `count=${cards(s5Tree).length}`,
  );
  report.check(
    !ui.hasText(s5Tree, "未找到节点") && !ui.hasText(s5Tree, "不允许操作"),
    "S5 门控放行期间无节点复制粘贴相关错误提示",
  );

  report.section("S6 门控-模板面板（F2）");
  const s6BaseKey = cardKeys(await api.uiTree()).join(",");
  await openTemplatePanel();
  report.check(
    ui.findAttr(await api.uiTree(), "title", "拖拽创建数据节点") !== null,
    "S6 模板面板展开（门控前置状态）",
  );
  await ctrlC();
  await ctrlV(null);
  const s6Stable = await waitCardSetStable(s6BaseKey);
  report.check(s6Stable, "S6 模板面板打开时 Ctrl+C/V 无效（卡片集合不变）", `count=${cards(await api.uiTree()).length}`);
  await snap("s6-template-panel-gate");
  await closeTemplatePanel();

  report.section("S7 门控-回收站面板（F3）");
  const s7BaseKey = cardKeys(await api.uiTree()).join(",");
  const s7Button = await waitForCondition(
    async () => ui.findAttr(await api.uiTree(), "title", "回收站"),
    { timeout: 8000, interval: 200, label: "「回收站」按钮" },
  );
  await ui.clickNode(s7Button);
  await waitForCondition(async () => ui.findButton(await api.uiTree(), "清空回收站"), {
    timeout: 6000,
    interval: 200,
    label: "回收站面板打开",
  });
  await sleep(400);
  report.check(true, "S7 回收站面板展开（门控前置状态）");
  await ctrlC();
  await ctrlV(null);
  const s7Stable = await waitCardSetStable(s7BaseKey);
  report.check(s7Stable, "S7 回收站面板打开时 Ctrl+C/V 无效（卡片集合不变）", `count=${cards(await api.uiTree()).length}`);
  await snap("s7-recycle-panel-gate");
  await closeRecycleBinPanel();

  report.section("S8 门控-输入框焦点（F4）");
  await clearHover();
  const searchButton = await waitForCondition(
    async () => ui.findAttr(await api.uiTree(), "title", "搜索"),
    { timeout: 8000, interval: 200, label: "「搜索」按钮" },
  );
  await ui.clickNode(searchButton);
  const s8Input = await waitForCondition(async () => searchInput(await api.uiTree()), {
    timeout: 6000,
    interval: 200,
    label: "搜索输入框",
  });
  report.check(s8Input.states?.focused === true, "S8 全局搜索框展开并自动聚焦（门控前置状态）");
  const s8BaseKey = cardKeys(await api.uiTree()).join(",");
  await ctrlC();
  await ctrlV(null);
  const s8Stable = await waitCardSetStable(s8BaseKey);
  report.check(
    s8Stable,
    "S8 输入框聚焦时 Ctrl+C/V 不触发节点复制粘贴（卡片集合不变；原生文本粘贴不作失败判定）",
    `count=${cards(await api.uiTree()).length}`,
  );
  await snap("s8-input-focus-gate");
  await ui.clickNode(ui.findAttr(await api.uiTree(), "title", "关闭搜索"));
  await sleep(700);

  report.section("S9 门控-边右键菜单（F5）");
  const s9BaseKey = cardKeys(await api.uiTree()).join(",");
  const menuOpened = await openEdgeMenu({
    sourceId: accountId,
    sourcePos: "right",
    targetId: noteId,
    targetPos: "left",
  });
  report.check(menuOpened, "S9 在 fixture 边曲线上右键弹出「编辑边」菜单（门控前置状态）");
  await ctrlC();
  await ctrlV(null);
  await sleep(900);
  await snap("s9-edge-menu-gate");
  await closeEdgeMenu();
  await sleep(700);
  const s9Stable = await waitCardSetStable(s9BaseKey);
  report.check(s9Stable, "S9 边右键菜单打开时 Ctrl+C/V 无效（关闭菜单后卡片集合不变）", `count=${cards(await api.uiTree()).length}`);

  report.section("S10 跨画布粘贴（步骤 S10）");
  await deselect();
  await clickCard(cardById(await api.uiTree(), accountId));
  await ctrlC();
  const rootBaseKey = cardKeys(await api.uiTree()).join(",");
  await clickCard(cardById(await api.uiTree(), childId), 2);
  await waitForCondition(
    async () => {
      const tree = await api.uiTree();
      const items = crumbText(tree);
      return items.length > 0 && items[items.length - 1] === "新画布" ? tree : null;
    },
    { timeout: 12000, interval: 250, label: "进入子画布" },
  );
  await sleep(800);
  const childTree = await api.uiTree();
  report.check(
    crumbText(childTree).join("/") === "画布宇宙/根画布/新画布",
    "S10 双击「新画布」进入子画布（面包屑为「画布宇宙 / 根画布 / 新画布」）",
    crumbText(childTree).join("/"),
  );
  report.check(cards(childTree).length === 0, "S10 子画布初始为空（跨画布粘贴的前置状态）");
  await ctrlV(P3);
  const childAdded = await waitCardCount(1);
  await sleep(700);
  const childAfterTree = await api.uiTree();
  const childCards = cards(childAfterTree);
  report.check(childAdded && childCards.length === 1, "S10 子画布 Ctrl+V 后出现 1 个副本", `count=${childCards.length}`);
  if (childCards.length === 1) {
    report.check(cardTitle(childCards[0]) === "账号 A", "S10 子画布副本标题为「账号 A」", cardTitle(childCards[0]));
    const center = ui.boundsCenter(childCards[0]);
    report.check(
      dist(center, P3) <= DROP_DISTANCE_MAX,
      `S10 子画布副本中心落在鼠标位置附近（≤${DROP_DISTANCE_MAX} 物理像素）`,
      `dist=${Math.round(dist(center, P3))}`,
    );
    const copyPixels = await borderPixels([childCards[0]]);
    report.check(
      copyPixels.length === 1 && isSelectionBorder(copyPixels[0]),
      "S10 子画布副本呈选中态（卡片上边框为选中主色）",
      JSON.stringify(copyPixels[0]),
    );
  }
  await snap("s10-child-canvas-paste");
  await ui.clickText("根画布");
  await waitForCondition(
    async () => {
      const tree = await api.uiTree();
      const items = crumbText(tree);
      return items.length > 0 && items[items.length - 1] === "根画布" && ui.hasText(tree, "账号 A")
        ? tree
        : null;
    },
    { timeout: 12000, interval: 250, label: "返回根画布" },
  );
  await sleep(800);
  const backStable = await waitCardSetStable(rootBaseKey);
  const backTree = await api.uiTree();
  report.check(
    backStable,
    "S10 返回根画布后卡片集合与进入前完全一致（原件仍在、副本归属子画布）",
    `count=${cards(backTree).length}`,
  );
  report.check(cardById(backTree, accountId) !== null, "S10 根画布原「账号 A」卡片仍在");
  await snap("s10-back-to-root");

  report.section("S11 源删除后粘贴（逻辑删除与物理删除，F6）");
  await deselect();
  await clickCard(cardById(await api.uiTree(), noteId));
  await ctrlC();
  await clickCardAction(noteId, "删除节点");
  await waitForCondition(async () => cardById(await api.uiTree(), noteId) === null, {
    timeout: 10000,
    interval: 250,
    label: "「备注 B」逻辑删除",
  });
  await sleep(800);
  const s11DeletedTree = await api.uiTree();
  report.check(!cardById(s11DeletedTree, noteId), "S11 「备注 B」逻辑删除后从画布消失");
  report.check(
    badgeTexts(s11DeletedTree).includes("1"),
    "S11 回收站徽标为「1」",
    JSON.stringify(badgeTexts(s11DeletedTree)),
  );
  const s11AfterDeleteKey = cardKeys(s11DeletedTree).join(",");
  await ctrlV(P1);
  const s11LogicalStable = await waitCardSetStable(s11AfterDeleteKey);
  const s11Tree = await api.uiTree();
  report.check(
    s11LogicalStable,
    "S11 逻辑删除源节点后 Ctrl+V 跳过该节点（卡片集合不变）",
    `count=${cards(s11Tree).length}`,
  );
  report.check(
    !ui.hasText(s11Tree, "未找到节点") && !ui.hasText(s11Tree, "不允许操作"),
    "S11 逻辑删除后的跳过无错误提示（NoNodeWithSuchId 被静默跳过）",
  );
  await snap("s11-logical-delete-skip");
  await ensureRecycleBinPanel();
  const physicalButton = await waitForCondition(
    async () => ui.findAttr(await api.uiTree(), "title", "永久删除"),
    { timeout: 6000, interval: 200, label: "「永久删除」按钮" },
  );
  await ui.clickNode(physicalButton);
  await ui.waitForDialog("永久删除节点", { timeout: 8000 });
  await sleep(400);
  const physicalDialog = ui.findDialog(await api.uiTree(), "永久删除节点");
  await ui.clickNode(ui.findButton(await api.uiTree(), "确认", { root: physicalDialog }));
  await sleep(1400);
  await closeRecycleBinPanel();
  await sleep(400);
  const physicalTree = await api.uiTree();
  report.check(badgeTexts(physicalTree).length === 0, "S11 永久删除「备注 B」后回收站徽标消失");
  const s11AfterPhysicalKey = cardKeys(physicalTree).join(",");
  await ctrlV(P1);
  const s11PhysicalStable = await waitCardSetStable(s11AfterPhysicalKey);
  const s11PhysicalTree = await api.uiTree();
  report.check(
    s11PhysicalStable,
    "S11 永久删除源节点后再次 Ctrl+V 仍跳过（卡片集合不变）",
    `count=${cards(s11PhysicalTree).length}`,
  );
  report.check(
    !ui.hasText(s11PhysicalTree, "未找到节点") && !ui.hasText(s11PhysicalTree, "不允许操作"),
    "S11 永久删除后的跳过同样无错误提示",
  );
  await snap("s11-physical-delete-skip");

  report.section("S12 回归-悬浮复制按钮（步骤 S12）");
  await clearHover();
  const s12Base = cardIdSet(await api.uiTree());
  const s12SourceCard = cardById(await api.uiTree(), accountId);
  await clickCardAction(accountId, "复制节点");
  const s12Added = await waitNewCardCount(s12Base, 1);
  await sleep(800);
  const s12Tree = await api.uiTree();
  const s12NewIds = newCards(s12Base, s12Tree).map((c) => c.attrs["data-id"]);
  report.check(s12Added && s12NewIds.length === 1, "S12 悬浮「复制节点」按钮后新增 1 个卡片", `count=${cards(s12Tree).length}`);
  if (s12NewIds.length === 1) {
    const copy = cardById(s12Tree, s12NewIds[0]);
    report.check(cardTitle(copy) === "账号 A", "S12 副本标题为「账号 A」", cardTitle(copy));
    const center = ui.boundsCenter(copy);
    const windowCenter = { x: info.width / 2, y: info.height / 2 };
    report.check(
      Math.abs(center.x - windowCenter.x) <= COPY_BUTTON_CENTER_TOLERANCE.x &&
        Math.abs(center.y - windowCenter.y) <= COPY_BUTTON_CENTER_TOLERANCE.y,
      `S12 副本落于视口中心区域（横向 ±${COPY_BUTTON_CENTER_TOLERANCE.x}、纵向 ±${COPY_BUTTON_CENTER_TOLERANCE.y} 物理像素）`,
      `center=(${Math.round(center.x)},${Math.round(center.y)}) window=(${windowCenter.x},${windowCenter.y})`,
    );
    report.check(
      dist(center, ui.boundsCenter(s12SourceCard)) > CENTER_EXCLUSION_MIN,
      `S12 副本远离源卡片（>${CENTER_EXCLUSION_MIN} 物理像素，保持容器中心语义而非鼠标位置语义）`,
      `dist=${Math.round(dist(center, ui.boundsCenter(s12SourceCard)))}`,
    );
  }
  await snap("s12-copy-button");
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
