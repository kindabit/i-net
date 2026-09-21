// case_007 跨画布结构与迁移。
//
// 流程：复制 base fixture → 启动应用 → 解锁（lastScene 恢复根画布）
// → 根画布创建画布数据节点「工作区」「参考区」（重命名同步画布名）
// → 逐层钻入创建「子级一」「子级二」构成 4 层画布链，验证面包屑折叠为「…」菜单
// → 省略号菜单跳转「工作区」；返回根画布
// → 账号 A → 工作区 跨画布连接（子画布出现入向影子 + 左侧虚线虚拟边）
// → 点击虚拟边跳回产生边所在画布并居中该边
// → 工作区 → 参考区 连接（工作区出现出向影子「参考区」）→ 工作区创建「节点 C」并连到出向影子
// → 根画布反向替换：断连确认框（取消 F4 / 确认替换）→ 验证方向反转与影子级联
// → 根画布创建「迁移节点」→ Alt 拖拽迁移到「工作区」（绿色落点高亮 + 视口中心落位）
// → 工作区 Alt 拖拽迁移到面包屑「根画布」片段（绿色片段高亮）
// → 三条非法迁移失败路径：含外部边节点集 / 影子节点 / 画布数据节点（红色落点高亮 + 提示）
// → 输出 PASS/FAIL 报告。
//
// 脚本规范要点：
// - Alt 组合拖拽必须整段放在同一个 /input 请求里：每个请求结束会析构原生输入句柄并释放
//   仍按下的按键，跨请求保持 Alt 会让状态机被重置（表现为普通画布内移动）。
//   配方：keyPress(Alt) → 移动/按下/分阶段拖动 → 松手前停顿 → mouseRelease → keyRelease(Alt)。
// - 落点高亮断言用并发截图：在 /input 队列执行期间（拖动停在落点上）另行请求 /screenshot，
//   对目标卡片外圈统计成功/错误主色像素、对面包屑片段统计绿色/红色底纹像素。
// - 影子虚拟边是 UI 树中的 role=button 节点（name 为对应方向的 hint 文案，bounds 为 40×10 CSS 像素），
//   定位与点击一律基于 UI 树：按 hint 文案取按钮节点，并按 bounds 相对卡片位置区分入向（卡片左侧）/
//   出向（卡片右侧）；点击后路由携带 edgeId 跳转到产生边所在画布并居中该边（入向与出向各自验证）。
// - 节点迁移目标由 DOM 命中顺序决定：被拖拽的画布数据节点自身会先命中，因此 F3 的红色高亮
//   出现在被拖拽节点上（见修订记录），断言按「红色高亮 + F3 文案 + 不迁移」执行。
// - 非法迁移按画布内移动持久化：先断言提示与节点仍在原画布，再继续后续步骤。
// - 拖拽落点会遮挡目标卡片/面包屑（拖拽中的卡片始终跟随光标）：目标 bounds 必须在拖动前采集；
//   被拖拽节点落到面包屑上后会挡住面包屑，后续步骤先把节点挪回空白区。
// - 全部等待使用 lib 等待函数；应用生命周期由 withApp 包装，保证无论成败都经 POST /shutdown 收尾。
//
// 运行方式：在项目根目录执行 `node e2e\script\case_007\case_007.js`

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

/** 根画布中各节点的拖拽创建落点（窗口物理像素） */
const ROOT_WORKSPACE_DROP = { x: 700, y: 900 };
const ROOT_REFERENCE_DROP = { x: 1120, y: 950 };
const ROOT_MIGRATE_DROP = { x: 350, y: 900 };
/** 子画布内创建画布数据节点/数据节点的落点（窗口物理像素） */
const CHILD_DROP = { x: 660, y: 480 };
const NODE_C_DROP = { x: 640, y: 750 };
/** F1 之后停放「账号 A」的空白落点（窗口物理像素） */
const ACCOUNT_A_PARK = { x: 350, y: 300 };
/** F2 之后停放影子节点的空白落点（窗口物理像素） */
const SHADOW_PARK = { x: 400, y: 760 };
/** 清场坐标（画布内空白区，窗口物理像素） */
const NEUTRAL_POINT = { x: 1420, y: 300 };
/** 取消选中用的画布空白点（窗口物理像素） */
const BLANK_POINT = { x: 300, y: 700 };

/** 落点高亮主色像素数下限：卡片外圈（success/error 描边）与面包屑片段（success/error 底纹） */
const RING_TINT_MIN = 300;
const CRUMB_TINT_MIN = 200;
/** 定位居中的容差：窗口尺寸的 10% */
const CENTER_TOLERANCE_RATIO = 0.1;
/** 迁移落位容差：窗口尺寸的 20% */
const RELOCATE_TOLERANCE_RATIO = 0.2;

/** 影子虚拟边的 UI 树锚点：hint 文案（i18n 的 database.canvas.shadow-*-hint） */
const SHADOW_INFLOW_HINT = "入度来自画布之外，点击定位至产生该影子的边";
const SHADOW_OUTFLOW_HINT = "出度指向画布之外，点击定位至产生该影子的边";
/** 数据节点固定尺寸（CSS 像素，见 src/node-size.ts），用于由卡片实测宽度换算物理像素比例 */
const NODE_CSS_WIDTH = 160;
/** 影子虚拟边的 CSS 尺寸（DataNode.vue 中 svg 的 width/height = 2.5rem × 0.625rem） */
const VIRTUAL_EDGE_CSS_WIDTH = 40;
const VIRTUAL_EDGE_CSS_HEIGHT = 10;
/** 虚拟边 bounds 尺寸断言的容差（物理像素） */
const VIRTUAL_EDGE_SIZE_TOLERANCE = 2;

const report = new Report("case_007 跨画布结构与迁移");

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
 * 把已解码的 PNG 保存到 output 目录（用于并发截取的拖动中画面）。
 * @param {import("pngjs").PNG} png 已解码的 PNG。
 * @param {string} label 截图标签。
 * @returns {string} 截图文件路径。
 */
function savePng(png, label) {
  shotIndex += 1;
  const file = path.join(OUTPUT_DIR, `${String(shotIndex).padStart(2, "0")}-${label}.png`);
  image.writePng(file, png);
  console.log(`  (screenshot) ${file}`);
  return file;
}

/**
 * 深度展开 UI 树为节点数组。
 * @param {object[]} tree UI 树顶层节点数组。
 * @returns {object[]} 全部节点数组（先序）。
 */
function flattenNodes(tree) {
  return ui.flatten(tree).map(({ node }) => node);
}

/**
 * 返回 UI 树中文本精确匹配的 #text 节点，按 top、left 升序排序。
 * @param {object[]} tree UI 树顶层节点数组。
 * @param {string} text 目标文本。
 * @returns {object[]} 命中的文本节点数组。
 */
function textNodes(tree, text) {
  return flattenNodes(tree)
    .filter((node) => node.tag === "#text" && (node.text ?? "").trim() === text)
    .sort((a, b) => a.bounds.top - b.bounds.top || a.bounds.left - b.bounds.left);
}

/**
 * 返回 UI 树中全部 vue-flow 节点卡片（role=group、带 data-id 且 aria-roledescription=node）。
 * @param {object[]} tree UI 树顶层节点数组。
 * @returns {object[]} 节点卡片组数组。
 */
function cardGroups(tree) {
  return flattenNodes(tree).filter(
    (node) =>
      node.tag === "DIV" &&
      node.role === "group" &&
      node.attrs?.["data-id"] &&
      node.attrs?.["aria-roledescription"] === "node",
  );
}

/**
 * 查找指定标题的第 index 个节点卡片（取包含该文本的最近卡片）。
 * @param {object[]} tree UI 树顶层节点数组。
 * @param {string} text 节点标题文本。
 * @param {number} [index=0] 同名节点序号（按 top、left 升序）。
 * @returns {object|null} 命中的节点卡片；未命中时为 null。
 */
function cardOf(tree, text, index = 0) {
  const titles = textNodes(tree, text);
  if (titles.length <= index) return null;
  const title = titles[index];
  const candidates = cardGroups(tree).filter((group) =>
    ui.flatten([group]).some(({ node }) => node.tag === "#text" && (node.text ?? "").trim() === text),
  );
  candidates.sort(
    (a, b) =>
      Math.hypot(a.bounds.left - title.bounds.left, a.bounds.top - title.bounds.top) -
      Math.hypot(b.bounds.left - title.bounds.left, b.bounds.top - title.bounds.top),
  );
  return candidates[0] ?? null;
}

/**
 * 取指定卡片指定方向的连接桩（连接桩在卡片外扩范围内）。
 * @param {object[]} tree UI 树顶层节点数组。
 * @param {object} card 节点卡片。
 * @param {string} pos 连接桩方向（top/right/bottom/left）。
 * @returns {object|null} 命中的连接桩；未命中时为 null。
 */
function handleOf(tree, card, pos) {
  const margin = 45;
  const hits = flattenNodes(tree).filter(
    (node) =>
      node.attrs?.["data-handlepos"] === pos &&
      node.bounds &&
      node.bounds.width > 0 &&
      node.bounds.left + node.bounds.width > card.bounds.left - margin &&
      node.bounds.left < card.bounds.left + card.bounds.width + margin &&
      node.bounds.top + node.bounds.height > card.bounds.top - margin &&
      node.bounds.top < card.bounds.top + card.bounds.height + margin,
  );
  hits.sort(
    (a, b) =>
      Math.hypot(ui.boundsCenter(a).x - ui.boundsCenter(card).x, ui.boundsCenter(a).y - ui.boundsCenter(card).y) -
      Math.hypot(ui.boundsCenter(b).x - ui.boundsCenter(card).x, ui.boundsCenter(b).y - ui.boundsCenter(card).y),
  );
  return hits[0] ?? null;
}

/**
 * 返回 UI 树中的全部边元素（role=group、aria-roledescription=edge）。
 * @param {object[]} tree UI 树顶层节点数组。
 * @returns {object[]} 边元素数组。
 */
function edgeGroups(tree) {
  return flattenNodes(tree).filter((node) => node.attrs?.["aria-roledescription"] === "edge");
}

/**
 * 返回面包屑条目（顶部条内的 listitem），按 left 升序。
 * @param {object[]} tree UI 树顶层节点数组。
 * @returns {object[]} 面包屑条目数组。
 */
function crumbs(tree) {
  return flattenNodes(tree)
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
 * 判断面包屑最后一项（当前画布）是否为指定文案。
 * @param {object[]} tree UI 树顶层节点数组。
 * @param {string} name 目标画布文案。
 * @returns {boolean} 匹配时为 true。
 */
function atCanvas(tree, name) {
  const items = crumbText(tree);
  return items.length > 0 && items[items.length - 1] === name;
}

/**
 * 把鼠标移到清场坐标并等待 hover 状态（工具条/提示）退场。
 * @returns {Promise<void>} 无返回值。
 */
async function clearHover() {
  await api.postInput([api.mouseMove(NEUTRAL_POINT.x, NEUTRAL_POINT.y)]);
  await sleep(420);
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
 * 展开模板面板（已展开时直接返回）。
 * @returns {Promise<void>} 无返回值。
 */
async function openTemplatePanel() {
  const tree = await api.uiTree();
  if (ui.findAttr(tree, "title", "拖拽创建数据节点")) return;
  const button = await waitForCondition(async () => ui.findAttr(await api.uiTree(), "title", "新建节点"), {
    timeout: 8000,
    interval: 200,
    label: "「新建节点」按钮",
  });
  await ui.clickNode(button);
  await sleep(700);
}

/**
 * 从模板面板拖拽指定手柄到画布落点创建节点，并等待面板自动关闭。
 * @param {string} handleTitle 拖拽手柄的 title（「拖拽创建数据节点」或「拖拽创建画布数据节点」）。
 * @param {{x: number, y: number}} drop 落点（窗口物理像素）。
 * @returns {Promise<void>} 无返回值。
 */
async function dragCreateNode(handleTitle, drop) {
  await openTemplatePanel();
  const handle = await waitForCondition(async () => ui.findAttr(await api.uiTree(), "title", handleTitle), {
    timeout: 8000,
    interval: 200,
    label: `模板手柄「${handleTitle}」`,
  });
  await api.postInput([api.mouseDrag(ui.boundsCenter(handle), drop)]);
  await waitForCondition(
    async () => ui.findAttr(await api.uiTree(), "title", handleTitle) === null,
    { timeout: 6000, interval: 200, label: "模板面板自动关闭" },
  );
  await sleep(800);
}

/**
 * 判断节点元素是否位于指定卡片上方的悬浮工具条范围内。
 * @param {object} node UI 树节点。
 * @param {object} card 节点卡片。
 * @returns {boolean} 位于工具条范围内时为 true。
 */
function nearCard(node, card) {
  return (
    node.bounds &&
    node.bounds.left + node.bounds.width >= card.bounds.left - 40 &&
    node.bounds.left <= card.bounds.left + card.bounds.width + 40 &&
    node.bounds.top >= card.bounds.top - 130 &&
    node.bounds.top + node.bounds.height <= card.bounds.top + 10
  );
}

/**
 * 悬停指定节点卡片中心并读取其悬浮操作按钮（按 left 升序）。
 * @param {string} text 节点标题文本。
 * @param {number} [index=0] 同名节点序号。
 * @returns {Promise<{card: object, buttons: object[]}>} 卡片与按钮数组。
 */
async function hoverCardActions(text, index = 0) {
  await clearHover();
  const tree0 = await api.uiTree();
  const card0 = cardOf(tree0, text, index);
  if (!card0) throw new Error(`hoverCardActions: card "${text}"[${index}] not found`);
  const c = ui.boundsCenter(card0);
  await api.postInput([api.mouseMove(c.x, c.y)]);
  await sleep(550);
  const tree = await api.uiTree();
  const card = cardOf(tree, text, index);
  const buttons = flattenNodes(tree).filter((n) => n.attrs?.title && nearCard(n, card));
  buttons.sort((a, b) => a.bounds.left - b.bounds.left);
  return { card, buttons };
}

/**
 * 通过 hover 工具条的「编辑节点」按钮打开编辑对话框并把标题改为指定文本。
 * @param {string} cardText 当前节点标题。
 * @param {string} newTitle 新标题。
 * @param {number} [index=0] 同名节点序号。
 * @returns {Promise<void>} 无返回值。
 */
async function renameNode(cardText, newTitle, index = 0) {
  const { buttons } = await hoverCardActions(cardText, index);
  const edit = buttons.find((b) => b.attrs.title === "编辑节点");
  if (!edit) throw new Error(`renameNode: edit button not found for "${cardText}"`);
  await ui.clickNode(edit);
  await ui.waitForDialog("编辑节点", { timeout: 8000 });
  await sleep(600);
  await ui.typeIntoEditable("标题", newTitle);
  const tree = await api.uiTree();
  const dialog = ui.findDialog(tree, "编辑节点");
  const confirm = ui.findButton(tree, "确认", { root: dialog });
  if (!confirm) throw new Error("renameNode: confirm button not found");
  await ui.clickNode(confirm);
  await ui.waitForDialogGone("编辑节点", { timeout: 8000 });
  await ui.waitForText(newTitle, { timeout: 8000 });
  await sleep(600);
}

/**
 * 点击对话框内按钮（在对话框子树内查找，避免与嵌套对话框同名按钮混淆）。
 * @param {string} dialogTitle 对话框标题/内容片段。
 * @param {string} buttonText 按钮文本。
 * @param {object} [options] 可选参数。
 * @param {boolean} [options.exact=false] 是否精确匹配按钮子树文本。
 * @returns {Promise<void>} 无返回值。
 */
async function clickDialogButton(dialogTitle, buttonText, { exact = false } = {}) {
  const tree = await api.uiTree();
  const dialog = ui.findDialog(tree, dialogTitle);
  if (!dialog) throw new Error(`clickDialogButton: dialog "${dialogTitle}" not found`);
  const button = ui.findButton(tree, buttonText, { root: dialog, exact });
  if (!button) throw new Error(`clickDialogButton: button "${buttonText}" not found in "${dialogTitle}"`);
  await ui.clickNode(button);
}

/**
 * 双击画布数据节点进入其子画布，等待面包屑当前项变为该画布。
 * @param {string} canvasText 画布数据节点标题（同时也是目标画布名）。
 * @param {object} [options] 可选参数。
 * @param {string|null} [options.expectText=null] 子画布内应出现的文本（空画布传 null）。
 * @param {number} [options.timeout=15000] 超时毫秒数。
 * @returns {Promise<void>} 无返回值。
 */
async function enterSubCanvas(canvasText, { expectText = null, timeout = 15000 } = {}) {
  await ui.clickText(canvasText, { count: 2 });
  await waitForCondition(
    async () => {
      const t = await api.uiTree();
      return atCanvas(t, canvasText) && (expectText === null || ui.hasText(t, expectText)) ? t : null;
    },
    { timeout, interval: 300, label: `进入子画布「${canvasText}」` },
  );
  await sleep(1000);
}

/**
 * 点击面包屑「根画布」片段返回根画布。
 * @returns {Promise<void>} 无返回值。
 */
async function backToRoot() {
  await ui.clickText("根画布");
  await ui.waitForText("备注 B", { timeout: 15000 });
  await sleep(1200);
}

/**
 * 从起点拖到终点建立边（连接桩坐标需足够新鲜）。
 * @param {{x: number, y: number}} from 起点（窗口物理像素）。
 * @param {{x: number, y: number}} to 终点（窗口物理像素）。
 * @returns {Promise<void>} 无返回值。
 */
async function connect(from, to) {
  await api.postInput([api.mouseDrag(from, to)]);
  await sleep(1400);
}

/**
 * 悬停指定节点标题后取指定方向连接桩中心（悬停后重新读取 UI 树，保证坐标新鲜）。
 * @param {string} text 节点标题文本。
 * @param {string} pos 连接桩方向（top/right/bottom/left）。
 * @param {number} [index=0] 同名节点序号。
 * @returns {Promise<{center: {x: number, y: number}, card: object, handle: object}>} 连接桩中心与卡片/连接桩元素。
 */
async function handleCenterOf(text, pos, index = 0) {
  await clearHover();
  const tree = await api.uiTree();
  const node = textNodes(tree, text)[index];
  if (!node) throw new Error(`handleCenterOf: text "${text}"[${index}] not found`);
  const c = ui.boundsCenter(node);
  await api.postInput([api.mouseMove(c.x, c.y)]);
  await sleep(500);
  const fresh = await api.uiTree();
  const card = cardOf(fresh, text, index);
  if (!card) throw new Error(`handleCenterOf: card "${text}"[${index}] not found`);
  const h = handleOf(fresh, card, pos);
  if (!h) throw new Error(`handleCenterOf: handle ${pos} of "${text}"[${index}] not found`);
  return { center: ui.boundsCenter(h), card, handle: h };
}

/**
 * 在指定节点的卡片中心做一次普通画布内拖拽（无 Alt）。
 * @param {string} text 节点标题文本。
 * @param {{x: number, y: number}} to 目标落点（窗口物理像素）。
 * @returns {Promise<void>} 无返回值。
 */
async function moveNodeTo(text, to) {
  await clearHover();
  const tree = await api.uiTree();
  const card = cardOf(tree, text);
  if (!card) throw new Error(`moveNodeTo: card "${text}" not found`);
  await api.postInput([api.mouseDrag(ui.boundsCenter(card), to)]);
  await sleep(1200);
}

/**
 * 统计 PNG 指定区域内满足谓词的像素数。
 * @param {import("pngjs").PNG} png 整窗截图 PNG。
 * @param {{left: number, top: number, width: number, height: number}} region 统计区域。
 * @param {(r: number, g: number, b: number) => boolean} predicate 颜色谓词。
 * @returns {number} 命中像素数。
 */
function countPixels(png, region, predicate) {
  let count = 0;
  for (let y = Math.max(0, Math.floor(region.top)); y < Math.min(png.height, Math.ceil(region.top + region.height)); y++) {
    for (let x = Math.max(0, Math.floor(region.left)); x < Math.min(png.width, Math.ceil(region.left + region.width)); x++) {
      const i = (png.width * y + x) << 2;
      if (predicate(png.data[i], png.data[i + 1], png.data[i + 2])) count += 1;
    }
  }
  return count;
}

/**
 * 判断颜色是否为迁移“允许”高亮主色（success 描边/底纹）。
 * @param {number} r 红色分量。
 * @param {number} g 绿色分量。
 * @param {number} b 蓝色分量。
 * @returns {boolean} 命中时为 true。
 */
function isSuccessGreen(r, g, b) {
  return g - r >= 60 && g - b >= 50 && g >= 140;
}

/**
 * 判断颜色是否为迁移“禁止”高亮主色（error 描边/底纹）。
 * @param {number} r 红色分量。
 * @param {number} g 绿色分量。
 * @param {number} b 蓝色分量。
 * @returns {boolean} 命中时为 true。
 */
function isErrorRed(r, g, b) {
  return r - g >= 80 && r - b >= 70 && r >= 180;
}

/**
 * 统计卡片外圈（迁移落点描边所在环带）的成功/错误主色像素数。
 * @param {import("pngjs").PNG} png 整窗截图 PNG。
 * @param {{left: number, top: number, width: number, height: number}} bounds 卡片 bounds。
 * @returns {{green: number, red: number}} 绿色与红色像素数。
 */
function ringTint(png, bounds) {
  const outer = { left: bounds.left - 8, top: bounds.top - 8, width: bounds.width + 16, height: bounds.height + 16 };
  const inset = 6;
  const inner = {
    left: bounds.left + inset,
    top: bounds.top + inset,
    width: bounds.width - 2 * inset,
    height: bounds.height - 2 * inset,
  };
  let green = 0;
  let red = 0;
  for (let y = Math.max(0, Math.floor(outer.top)); y < Math.min(png.height, Math.ceil(outer.top + outer.height)); y++) {
    for (let x = Math.max(0, Math.floor(outer.left)); x < Math.min(png.width, Math.ceil(outer.left + outer.width)); x++) {
      if (x >= inner.left && x < inner.left + inner.width && y >= inner.top && y < inner.top + inner.height) continue;
      const i = (png.width * y + x) << 2;
      const [r, g, b] = [png.data[i], png.data[i + 1], png.data[i + 2]];
      if (isSuccessGreen(r, g, b)) green += 1;
      if (isErrorRed(r, g, b)) red += 1;
    }
  }
  return { green, red };
}

/**
 * 统计面包屑片段底纹的绿色/红色像素数（success/error 18% 透明底纹）。
 * @param {import("pngjs").PNG} png 整窗截图 PNG。
 * @param {{left: number, top: number, width: number, height: number}} bounds 片段 bounds。
 * @returns {{green: number, red: number}} 绿色与红色像素数。
 */
function crumbTint(png, bounds) {
  return {
    green: countPixels(png, bounds, (r, g, b) => g - r >= 8 && g - b >= 8),
    red: countPixels(png, bounds, (r, g, b) => r - g >= 8 && r - b >= 8),
  };
}

/**
 * 在 UI 树中定位影子节点的虚拟边按钮（role=button、name 为对应方向的 hint 文案）。
 * @param {object[]} tree UI 树顶层节点数组。
 * @param {"inflow"|"outflow"} direction 影子方向（入向/出向）。
 * @param {object|null} [card=null] 影子卡片；提供时取离卡片最近且位于对应侧旁的按钮。
 * @returns {object|null} 命中的虚拟边按钮节点；未命中时为 null。
 */
function virtualEdgeOf(tree, direction, card = null) {
  const hint = direction === "inflow" ? SHADOW_INFLOW_HINT : SHADOW_OUTFLOW_HINT;
  const hits = flattenNodes(tree).filter(
    (node) => node.role === "button" && (node.name ?? "").trim() === hint && node.bounds && node.bounds.width > 0,
  );
  if (hits.length === 0) return null;
  if (!card) return hits[0];
  const center = ui.boundsCenter(card);
  hits.sort(
    (a, b) =>
      Math.hypot(ui.boundsCenter(a).x - center.x, ui.boundsCenter(a).y - center.y) -
      Math.hypot(ui.boundsCenter(b).x - center.x, ui.boundsCenter(b).y - center.y),
  );
  const hit = hits[0];
  // 相对卡片位置过滤：入向按钮位于卡片左侧、出向按钮位于卡片右侧（margin 2px CSS）
  const onLeft = hit.bounds.left + hit.bounds.width <= card.bounds.left + 2;
  const onRight = hit.bounds.left >= card.bounds.left + card.bounds.width - 2;
  if (direction === "inflow" && !onLeft) return null;
  if (direction === "outflow" && !onRight) return null;
  return hit;
}

/**
 * 断言虚拟边按钮的 bounds 尺寸等于 40×10 CSS 像素：按节点卡片实测宽度与固定 CSS 宽度求物理像素比例。
 * @param {object} edgeNode 虚拟边按钮节点。
 * @param {object} card 影子卡片。
 * @param {string} label 断言描述前缀。
 * @returns {boolean} 尺寸断言是否通过。
 */
function checkVirtualEdgeGeometry(edgeNode, card, label) {
  const scale = card.bounds.width / NODE_CSS_WIDTH;
  const expectedWidth = VIRTUAL_EDGE_CSS_WIDTH * scale;
  const expectedHeight = VIRTUAL_EDGE_CSS_HEIGHT * scale;
  const ok =
    Math.abs(edgeNode.bounds.width - expectedWidth) <= VIRTUAL_EDGE_SIZE_TOLERANCE &&
    Math.abs(edgeNode.bounds.height - expectedHeight) <= VIRTUAL_EDGE_SIZE_TOLERANCE;
  return report.check(
    ok,
    `${label} bounds 尺寸为 40×10 CSS 像素（按卡片实测宽度换算物理像素）`,
    `bounds=${edgeNode.bounds.width}x${edgeNode.bounds.height} expected≈${Math.round(expectedWidth)}x${Math.round(expectedHeight)} scale=${scale}`,
  );
}

/**
 * 点击节点的可见区域中心（中心越出窗口时收敛到窗口内 8px），用于贴着窗口边缘的虚拟边按钮。
 * @param {object} node UI 树节点（bounds 为窗口物理像素）。
 * @param {{width: number, height: number}} viewportSize 窗口物理尺寸。
 * @returns {Promise<void>} 无返回值。
 */
async function clickNodeVisible(node, viewportSize) {
  const center = ui.boundsCenter(node);
  const x = Math.min(Math.max(center.x, 8), viewportSize.width - 8);
  const y = Math.min(Math.max(center.y, 8), viewportSize.height - 8);
  await api.postInput([api.mouseMove(x, y)]);
  await sleep(250);
  await api.postInput([api.mouseClick("left", 1)]);
}

/**
 * Alt 组合拖拽（整段单个 /input 请求）：
 * keyPress(Alt) → 移动起点 → mousePress → 抖动 → 分阶段移动 → 停在落点 → mouseRelease → keyRelease(Alt)。
 * 拖动保持期间轮询并发截图，按评分函数挑选最优帧（落点高亮只在光标停驻于落点期间渲染，
 * 而命令注入存在每条约百毫秒的固定开销，无法预知准确的保持时刻），并可读取拖动中的 UI 树。
 * @param {{x: number, y: number}} from 起点（窗口物理像素）。
 * @param {{x: number, y: number}} to 落点（窗口物理像素）。
 * @param {object} [options] 可选参数。
 * @param {string} [options.label="alt-drag"] 日志标签。
 * @param {number} [options.holdMs=5000] 停在落点的保持时长（毫秒）。
 * @param {((png: import("pngjs").PNG) => number)|null} [options.score=null] 并发截图评分函数。
 * @param {number} [options.minScore=1] 提前停止轮询的评分下限。
 * @param {boolean} [options.needTree=false] 是否读取拖动中的 UI 树。
 * @returns {Promise<{png: import("pngjs").PNG|null, score: number, midTree: object[]|null, queueMs: number}>} 最优截图、其评分、拖动中的 UI 树与队列耗时。
 */
async function altDrag(from, to, { label = "alt-drag", holdMs = 5000, score = null, minScore = 1, needTree = false } = {}) {
  const mid = { x: (from.x + to.x) / 2, y: (from.y + to.y) / 2 - 60 };
  const queue = [
    api.keyPress(["Alt"]),
    api.mouseMove(from.x, from.y),
    api.mousePress("left"),
    api.mouseMove(from.x + 16, from.y + 16),
    api.wait(250),
    api.mouseMove(mid.x, mid.y),
    api.wait(200),
    api.mouseMove(to.x, to.y),
    api.wait(holdMs),
    api.mouseRelease("left"),
    api.wait(300),
    api.keyRelease(["Alt"]),
  ];
  const started = Date.now();
  const pending = api.postInput(queue);
  let bestPng = null;
  let bestScore = -1;
  let midTree = null;
  if (score) {
    const offsets = [600, 1100, 1600, 2100, 2600, 3100, 3600, 4100];
    for (const offset of offsets) {
      const waitMs = started + offset - Date.now();
      if (waitMs > 0) await sleep(waitMs);
      try {
        const png = image.decodePng(await api.screenshot());
        const value = score(png);
        if (value > bestScore) {
          bestScore = value;
          bestPng = png;
        }
        if (value >= minScore) break;
      } catch (error) {
        console.log(`  (drag capture failed at ${offset}ms: ${error.message})`);
        break;
      }
    }
    console.log(`  (drag capture) ${label} bestScore=${bestScore}`);
  }
  if (needTree) {
    try {
      midTree = await api.uiTree();
    } catch (error) {
      console.log(`  (drag mid-tree failed: ${error.message})`);
    }
  }
  const queueMs = await pending;
  console.log(`  (alt-drag) ${label} queueMs=${queueMs} holdMs=${holdMs}`);
  await sleep(1400);
  return { png: bestPng, score: bestScore, midTree, queueMs };
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
      return ui.findEditable(tree, "数据库名称") ?? ui.findEditable(tree, "Database Name") ? tree : null;
    },
    { timeout: 20000, label: "home page name input" },
  );
  if (ui.findEditable(initial, "数据库名称")) {
    return false;
  }
  console.log("  (ui language is not Chinese; switching back via the language menu)");
  const button = ui.findButton(initial, "切换语言") ?? ui.findButton(initial, "Switch Language");
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
 * 用例主流程。
 * @returns {Promise<void>} 无返回值。
 */
async function main() {
  report.section("S0 前置与解锁（前置条件）");
  const info = await api.health();
  report.check(info.width > 0 && info.height > 0, "调试自动化服务可用", `${info.width}x${info.height}`);
  await ensureChineseUi();
  await unlock();
  let tree = await api.uiTree();
  report.check(ui.hasText(tree, "账号 A"), "解锁后进入根画布并出现节点「账号 A」");
  report.check(ui.hasText(tree, "备注 B"), "解锁后出现节点「备注 B」");
  report.check(crumbText(tree).join("/") === "画布宇宙/根画布", "根画布面包屑为「画布宇宙 / 根画布」", crumbText(tree).join("/"));
  report.check(edgeGroups(tree).length === 1, "fixture 根画布存在 1 条边", `edges=${edgeGroups(tree).length}`);
  await sleep(600);
  await snap("s0-unlocked-root");

  report.section("S1 创建画布数据节点并重命名（步骤 1）");
  {
    await dragCreateNode("拖拽创建画布数据节点", ROOT_WORKSPACE_DROP);
    tree = await api.uiTree();
    report.check(cardOf(tree, "新画布") !== null, "步骤 1 拖拽创建出现画布数据节点「新画布」");
    await renameNode("新画布", "工作区");
    tree = await api.uiTree();
    report.check(cardOf(tree, "工作区") !== null, "步骤 1 重命名后根画布出现画布数据节点「工作区」");
    report.check(textNodes(tree, "新画布").length === 0, "步骤 1 旧标题「新画布」消失");
  }
  await snap("step1-workspace-node");

  report.section("S2 创建参考区（步骤 2）");
  {
    await dragCreateNode("拖拽创建画布数据节点", ROOT_REFERENCE_DROP);
    await renameNode("新画布", "参考区");
    tree = await api.uiTree();
    report.check(
      cardOf(tree, "工作区") !== null && cardOf(tree, "参考区") !== null,
      "步骤 2 根画布存在「工作区」「参考区」两个画布数据节点",
    );
  }
  await snap("step2-reference-node");

  report.section("S3 四级画布链与重命名同步（步骤 3）");
  {
    await enterSubCanvas("工作区");
    tree = await api.uiTree();
    report.check(atCanvas(tree, "工作区"), "步骤 3 进入「工作区」后面包屑当前项为「工作区」（节点重命名已同步画布名）");
    report.check(crumbText(tree).length === 3, "步骤 3 三级链无折叠（画布宇宙 / 根画布 / 工作区）", crumbText(tree).join("/"));
    await dragCreateNode("拖拽创建画布数据节点", CHILD_DROP);
    await renameNode("新画布", "子级一");
    await enterSubCanvas("子级一");
    tree = await api.uiTree();
    report.check(atCanvas(tree, "子级一"), "步骤 3 进入「子级一」");
    report.check(
      crumbText(tree).join("/") === "画布宇宙/根画布/工作区/子级一" && !ui.hasText(tree, "…"),
      "步骤 3 链长为 3 的画布链平铺显示完整层级且无省略号",
      crumbText(tree).join("/"),
    );
    await dragCreateNode("拖拽创建画布数据节点", CHILD_DROP);
    await renameNode("新画布", "子级二");
    await enterSubCanvas("子级二");
    tree = await api.uiTree();
    report.check(atCanvas(tree, "子级二"), "步骤 3 进入「子级二」，画布链达到 4 层");
    await snap("step3-level4");
  }

  report.section("S4 面包屑返回根画布（步骤 4）");
  {
    await ui.clickText("根画布");
    await ui.waitForText("备注 B", { timeout: 15000 });
    await sleep(1200);
    tree = await api.uiTree();
    report.check(atCanvas(tree, "根画布"), "步骤 4 点击面包屑「根画布」返回根画布");
    report.check(!ui.hasText(tree, "…"), "步骤 4 根画布面包屑无省略号（链长 ≤3 时平铺）");
    report.check(textNodes(tree, "备注 B").length === 1, "步骤 4 根画布内容恢复（出现「备注 B」）");
  }
  await snap("step4-back-root");

  report.section("S5 四级链折叠为省略号（步骤 5）");
  {
    await enterSubCanvas("工作区", { expectText: "子级一" });
    await enterSubCanvas("子级一", { expectText: "子级二" });
    await enterSubCanvas("子级二");
    tree = await api.uiTree();
    const names = crumbText(tree);
    report.check(
      names.join("/") === "画布宇宙/根画布/…/子级一/子级二",
      "步骤 5 链长为 4 的画布链折叠为「根画布 / … / 子级一 / 子级二」",
      names.join("/"),
    );
    report.check(!ui.hasText(tree, "工作区"), "步骤 5 被折叠的「工作区」不在面包屑中显示");
  }
  await snap("step5-collapsed-breadcrumb");

  report.section("S6 省略号菜单跳转（步骤 6）");
  {
    await ui.clickText("…", { exact: true });
    await sleep(900);
    tree = await api.uiTree();
    const menuItems = textNodes(tree, "工作区").filter((n) => n.bounds.top >= 130 && n.bounds.top < 260);
    report.check(menuItems.length === 1, "步骤 6 省略号菜单列出被隐藏层级「工作区」", `hits=${menuItems.length}`);
    await snap("step6-ellipsis-menu");
    await ui.clickNode(menuItems[0]);
    await waitForCondition(
      async () => {
        const t = await api.uiTree();
        return atCanvas(t, "工作区") && ui.hasText(t, "子级一") && !ui.hasText(t, "子级二") ? t : null;
      },
      { timeout: 12000, interval: 300, label: "省略号菜单跳转「工作区」" },
    );
    await sleep(900);
    tree = await api.uiTree();
    report.check(atCanvas(tree, "工作区"), "步骤 6 点击菜单项后面包屑当前项为「工作区」");
    report.check(ui.hasText(tree, "子级一"), "步骤 6 已进入「工作区」（可见其内容「子级一」）");
  }
  await snap("step6-jumped-workspace");

  report.section("S7 跨画布连接与入向影子（步骤 7）");
  {
    await backToRoot();
    const h1 = await handleCenterOf("账号 A", "right");
    const h2 = await handleCenterOf("工作区", "top");
    await connect(h1.center, h2.center);
    await deselect();
    tree = await api.uiTree();
    report.check(edgeGroups(tree).length === 2, "步骤 7 「账号 A」→「工作区」连线建立成功", `edges=${edgeGroups(tree).length}`);
    await snap("step7-edge-to-workspace");
    await enterSubCanvas("工作区", { expectText: "账号 A" });
    tree = await api.uiTree();
    const shadowCard = cardOf(tree, "账号 A");
    report.check(shadowCard !== null, "步骤 7 进入「工作区」后出现影子节点「账号 A」");
    report.check(!ui.hasText(tree, "备注 B"), "步骤 7 子画布内不含根画布节点「备注 B」");
    if (shadowCard) {
      const inflowEdge = virtualEdgeOf(tree, "inflow", shadowCard);
      report.check(
        inflowEdge !== null && inflowEdge.role === "button" && (inflowEdge.name ?? "").trim() === SHADOW_INFLOW_HINT,
        "步骤 7 入向虚拟边以 role=button + 入向 hint 文案出现在 UI 树中",
        inflowEdge ? `node=${JSON.stringify({ role: inflowEdge.role, name: inflowEdge.name })}` : "not-found",
      );
      if (inflowEdge) {
        checkVirtualEdgeGeometry(inflowEdge, shadowCard, "步骤 7 入向虚拟边");
        report.check(
          inflowEdge.bounds.left + inflowEdge.bounds.width <= shadowCard.bounds.left,
          "步骤 7 入向虚拟边位于影子卡片左侧",
          `edge=${JSON.stringify(inflowEdge.bounds)} card=${JSON.stringify(shadowCard.bounds)}`,
        );
        const png = image.decodePng(await api.screenshot());
        const dark = countPixels(png, inflowEdge.bounds, (r, g, b) => r < 150 && g < 150 && b < 150);
        report.check(dark > 100, "步骤 7 入向虚拟边渲染虚线主色（深色像素断言）", `darkPixels=${dark}`);
      }
    }
  }
  await snap("step7-shadow-appears");

  report.section("S8 虚拟边点击跳转并居中（步骤 8）");
  {
    tree = await api.uiTree();
    const shadowCard = cardOf(tree, "账号 A");
    if (!shadowCard) throw new Error("S8: shadow card not found");
    const inflowEdge = virtualEdgeOf(tree, "inflow", shadowCard);
    if (!inflowEdge) throw new Error("S8: inflow virtual edge not found in UI tree");
    // 入向虚拟边位于卡片左侧（可能贴窗口左缘），按 UI 树 bounds 取可见中心点击
    await clickNodeVisible(inflowEdge, info);
    await waitForCondition(
      async () => {
        const t = await api.uiTree();
        return atCanvas(t, "根画布") && ui.hasText(t, "备注 B") ? t : null;
      },
      { timeout: 12000, interval: 250, label: "虚拟边跳转回根画布" },
    );
    await sleep(1600);
    tree = await api.uiTree();
    report.check(atCanvas(tree, "根画布"), "步骤 8 点击虚拟边跳转回产生边所在画布（面包屑当前项为「根画布」）");
    report.check(textNodes(tree, "工作区").length === 1, "步骤 8 根画布内可见产生边的目标节点「工作区」");
    const aCard = cardOf(tree, "账号 A");
    const wCard = cardOf(tree, "工作区");
    if (aCard && wCard) {
      const a = ui.boundsCenter(aCard);
      const w = ui.boundsCenter(wCard);
      const mid = { x: (a.x + w.x) / 2, y: (a.y + w.y) / 2 };
      const delta = { x: Math.abs(mid.x - info.width / 2), y: Math.abs(mid.y - info.height / 2) };
      report.check(
        delta.x <= info.width * CENTER_TOLERANCE_RATIO && delta.y <= info.height * CENTER_TOLERANCE_RATIO,
        "步骤 8 定位动画后视图居中于该边（两端节点中心中点位于窗口中心 10% 容差内）",
        `mid=(${Math.round(mid.x)},${Math.round(mid.y)}) delta=(${Math.round(delta.x)},${Math.round(delta.y)})`,
      );
    }
  }
  await snap("step8-jumped-centered");

  report.section("S9 出向影子与出向虚拟边（步骤 9）");
  {
    await deselect();
    const h1 = await handleCenterOf("工作区", "right");
    const h2 = await handleCenterOf("参考区", "top");
    await connect(h1.center, h2.center);
    await deselect();
    tree = await api.uiTree();
    report.check(edgeGroups(tree).length === 3, "步骤 9 「工作区」→「参考区」连线建立成功", `edges=${edgeGroups(tree).length}`);
    await snap("step9-edge-workspace-reference");
    await enterSubCanvas("工作区", { expectText: "参考区" });
    tree = await api.uiTree();
    const shadowCard = cardOf(tree, "参考区");
    report.check(shadowCard !== null, "步骤 9 进入「工作区」后出现出向影子节点「参考区」");
    if (shadowCard) {
      const outflowEdge = virtualEdgeOf(tree, "outflow", shadowCard);
      report.check(
        outflowEdge !== null && outflowEdge.role === "button" && (outflowEdge.name ?? "").trim() === SHADOW_OUTFLOW_HINT,
        "步骤 9 出向虚拟边以 role=button + 出向 hint 文案出现在 UI 树中",
        outflowEdge ? `node=${JSON.stringify({ role: outflowEdge.role, name: outflowEdge.name })}` : "not-found",
      );
      if (outflowEdge) {
        checkVirtualEdgeGeometry(outflowEdge, shadowCard, "步骤 9 出向虚拟边");
        report.check(
          outflowEdge.bounds.left >= shadowCard.bounds.left + shadowCard.bounds.width,
          "步骤 9 出向虚拟边位于影子卡片右侧",
          `edge=${JSON.stringify(outflowEdge.bounds)} card=${JSON.stringify(shadowCard.bounds)}`,
        );
        const png = image.decodePng(await api.screenshot());
        const dark = countPixels(png, outflowEdge.bounds, (r, g, b) => r < 150 && g < 150 && b < 150);
        report.check(dark > 100, "步骤 9 出向虚拟边渲染虚线主色（深色像素断言）", `darkPixels=${dark}`);
      }
    }
  }
  await snap("step9-outflow-shadow");

  {
    // 出向虚拟边点击验证：跳转到产生边（工作区 → 参考区，位于根画布）所在画布并居中该边
    tree = await api.uiTree();
    const shadowCard = cardOf(tree, "参考区");
    const outflowEdge = shadowCard ? virtualEdgeOf(tree, "outflow", shadowCard) : null;
    if (!outflowEdge) throw new Error("S9: outflow virtual edge not found for click");
    await clickNodeVisible(outflowEdge, info);
    await waitForCondition(
      async () => {
        const t = await api.uiTree();
        return atCanvas(t, "根画布") && ui.hasText(t, "备注 B") ? t : null;
      },
      { timeout: 12000, interval: 250, label: "出向虚拟边跳转回根画布" },
    );
    await sleep(1600);
    tree = await api.uiTree();
    report.check(atCanvas(tree, "根画布"), "步骤 9 点击出向虚拟边跳转到产生边所在画布（面包屑当前项为「根画布」）");
    report.check(textNodes(tree, "参考区").length === 1, "步骤 9 根画布内可见产生边的目标节点「参考区」");
    const wCard = cardOf(tree, "工作区");
    const rCard = cardOf(tree, "参考区");
    if (wCard && rCard) {
      const w = ui.boundsCenter(wCard);
      const r = ui.boundsCenter(rCard);
      const mid = { x: (w.x + r.x) / 2, y: (w.y + r.y) / 2 };
      const delta = { x: Math.abs(mid.x - info.width / 2), y: Math.abs(mid.y - info.height / 2) };
      report.check(
        delta.x <= info.width * CENTER_TOLERANCE_RATIO && delta.y <= info.height * CENTER_TOLERANCE_RATIO,
        "步骤 9 定位动画后视图居中于该边（工作区/参考区节点中心中点位于窗口中心 10% 容差内）",
        `mid=(${Math.round(mid.x)},${Math.round(mid.y)}) delta=(${Math.round(delta.x)},${Math.round(delta.y)})`,
      );
    }
    await snap("step9-outflow-jump");
    await enterSubCanvas("工作区", { expectText: "参考区" });
  }

  report.section("S10 节点 C 连接出向影子（步骤 10）");
  {
    await dragCreateNode("拖拽创建数据节点", NODE_C_DROP);
    await renameNode("新节点", "节点 C");
    tree = await api.uiTree();
    report.check(cardOf(tree, "节点 C") !== null, "步骤 10 工作区出现数据节点「节点 C」");
    report.check(edgeGroups(tree).length === 0, "步骤 10 连接前工作区尚无内部边", `edges=${edgeGroups(tree).length}`);
    const h1 = await handleCenterOf("节点 C", "right");
    const h2 = await handleCenterOf("参考区", "left");
    await connect(h1.center, h2.center);
    await deselect();
    tree = await api.uiTree();
    report.check(edgeGroups(tree).length === 1, "步骤 10 「节点 C」→ 出向影子「参考区」连接成功", `edges=${edgeGroups(tree).length}`);
  }
  await snap("step10-node-c-connected");

  report.section("S11 反向替换确认框与取消 F4（步骤 11、F4）");
  {
    await backToRoot();
    const h1 = await handleCenterOf("参考区", "top");
    const h2 = await handleCenterOf("工作区", "right");
    await connect(h1.center, h2.center);
    await ui.waitForDialog("替换连接将断开节点连接", { timeout: 8000 });
    await sleep(700);
    tree = await api.uiTree();
    const dialog = ui.findDialog(tree, "替换连接将断开节点连接");
    const dialogText = dialog ? ui.subtreeText(dialog) : "";
    report.check(dialog !== null, "步骤 11 反向连接弹出「替换连接将断开节点连接」确认框");
    report.check(
      dialogText.includes("替换此连接会删除并重建相关影子节点") && dialogText.includes("以下节点将失去连接：节点 C"),
      "步骤 11 确认框正文列出将失去连接的「节点 C」",
      dialogText,
    );
    report.check(
      ui.findButton(tree, "取消", { root: dialog, exact: true }) !== null,
      "步骤 11 确认框含「取消」按钮",
    );
    report.check(
      ui.findButton(tree, "替换连接", { root: dialog, exact: true }) !== null,
      "步骤 11 确认按钮文案为「替换连接」（replace-edge-confirm）",
    );
    await snap("step11-replace-dialog");
    await clickDialogButton("替换连接将断开节点连接", "取消");
    await ui.waitForDialogGone("替换连接将断开节点连接", { timeout: 8000 });
    await sleep(1200);
    tree = await api.uiTree();
    report.check(edgeGroups(tree).length === 3, "F4 点「取消」后根画布边数量不变", `edges=${edgeGroups(tree).length}`);
    await enterSubCanvas("工作区", { expectText: "节点 C" });
    tree = await api.uiTree();
    report.check(cardOf(tree, "参考区") !== null, "F4 取消后出向影子「参考区」保留");
    report.check(edgeGroups(tree).length === 1, "F4 取消后「节点 C」与影子的边保留", `edges=${edgeGroups(tree).length}`);
  }
  await snap("f4-cancel-kept");

  report.section("S12 确认替换与方向反转（步骤 12）");
  {
    await backToRoot();
    const h1 = await handleCenterOf("参考区", "top");
    const h2 = await handleCenterOf("工作区", "right");
    await connect(h1.center, h2.center);
    await ui.waitForDialog("替换连接将断开节点连接", { timeout: 8000 });
    await sleep(600);
    await clickDialogButton("替换连接将断开节点连接", "替换连接", { exact: true });
    await ui.waitForDialogGone("替换连接将断开节点连接", { timeout: 8000 });
    await sleep(1500);
    tree = await api.uiTree();
    report.check(edgeGroups(tree).length === 3, "步骤 12 替换后根画布仍为 3 条边（删旧建新）", `edges=${edgeGroups(tree).length}`);
    report.check(
      !flattenNodes(tree).some((n) => (n.text ?? "").includes("失败") || (n.text ?? "").includes("错误")),
      "步骤 12 替换后无残留错误提示",
    );
    await snap("step12-root-after-replace");
    await enterSubCanvas("工作区", { expectText: "节点 C" });
    tree = await api.uiTree();
    report.check(cardOf(tree, "参考区") === null, "步骤 12 方向反转到「参考区」画布后，工作区内的出向影子「参考区」级联消失");
    report.check(cardOf(tree, "节点 C") !== null && cardOf(tree, "账号 A") !== null, "步骤 12 「节点 C」与入向影子「账号 A」保留");
    report.check(edgeGroups(tree).length === 0, "步骤 12 影子消失连带「节点 C」的外部连接被删除", `edges=${edgeGroups(tree).length}`);
    await snap("step12-workspace-after-replace");
    await backToRoot();
    await enterSubCanvas("参考区", { expectText: "工作区" });
    tree = await api.uiTree();
    report.check(cardOf(tree, "工作区") !== null, "步骤 12 新边（参考区 → 工作区）在参考区画布内产生出向影子「工作区」（方向反转证据）");
    await snap("step12-reference-shadow");
    await backToRoot();
  }

  report.section("S13 创建迁移节点（步骤 13）");
  {
    await dragCreateNode("拖拽创建数据节点", ROOT_MIGRATE_DROP);
    await renameNode("新节点", "迁移节点");
    tree = await api.uiTree();
    report.check(cardOf(tree, "迁移节点") !== null, "步骤 13 根画布出现「迁移节点」");
  }
  await snap("step13-migrate-node");

  report.section("S14 Alt 迁移到画布数据节点（步骤 14）");
  {
    await clearHover();
    tree = await api.uiTree();
    const src = cardOf(tree, "迁移节点");
    const dst = cardOf(tree, "工作区");
    if (!src || !dst) throw new Error("S14: card not found");
    const targetBounds = { ...dst.bounds };
    const from = ui.boundsCenter(src);
    const to = { x: targetBounds.left + 30, y: targetBounds.top + 30 };
    const { png, score: greenScore } = await altDrag(from, to, {
      label: "step14-allow",
      score: (frame) => ringTint(frame, targetBounds).green,
      minScore: RING_TINT_MIN,
    });
    if (png) {
      const tint = ringTint(png, targetBounds);
      savePng(png, "step14-drag-hold-allow");
      report.check(
        tint.green >= RING_TINT_MIN && tint.green > tint.red,
        "步骤 14 拖到画布数据节点上时落点显示“允许”高亮（success 描边）",
        `green=${tint.green} red=${tint.red}`,
      );
    } else {
      report.check(false, "步骤 14 拖到画布数据节点上时落点显示“允许”高亮（success 描边）", "未捕获到拖动中截图");
    }
    tree = await api.uiTree();
    report.check(cardOf(tree, "迁移节点") === null, "步骤 14 迁移成功后根画布中「迁移节点」消失");
    await snap("step14-after-relocate");
    await enterSubCanvas("工作区", { expectText: "迁移节点" });
    tree = await api.uiTree();
    const moved = cardOf(tree, "迁移节点");
    report.check(moved !== null, "步骤 14 目标画布「工作区」中出现「迁移节点」");
    if (moved) {
      const center = ui.boundsCenter(moved);
      const tolerance = { x: info.width * RELOCATE_TOLERANCE_RATIO, y: info.height * RELOCATE_TOLERANCE_RATIO };
      report.check(
        Math.abs(center.x - info.width / 2) <= tolerance.x && Math.abs(center.y - info.height / 2) <= tolerance.y,
        "步骤 14 迁移节点落位于目标画布视口中心附近（窗口 20% 容差）",
        `center=(${Math.round(center.x)},${Math.round(center.y)})`,
      );
    }
  }
  await snap("step14-in-workspace");

  report.section("S15 Alt 迁移到面包屑祖先片段（步骤 15）");
  {
    tree = await api.uiTree();
    const src = cardOf(tree, "迁移节点");
    const crumbRoot = crumbs(tree).find((item) => ui.subtreeText(item).trim() === "根画布");
    if (!src || !crumbRoot) throw new Error("S15: source or crumb not found");
    const crumbBounds = { ...crumbRoot.bounds };
    const { png } = await altDrag(ui.boundsCenter(src), ui.boundsCenter(crumbRoot), {
      label: "step15-allow-crumb",
      score: (frame) => crumbTint(frame, crumbBounds).green,
      minScore: CRUMB_TINT_MIN,
    });
    if (png) {
      const tint = crumbTint(png, crumbBounds);
      savePng(png, "step15-drag-hold-crumb-allow");
      report.check(
        tint.green >= CRUMB_TINT_MIN && tint.green > tint.red,
        "步骤 15 拖到面包屑「根画布」片段上时片段显示“允许”高亮（success 底纹）",
        `green=${tint.green} red=${tint.red}`,
      );
    } else {
      report.check(false, "步骤 15 拖到面包屑「根画布」片段上时片段显示“允许”高亮（success 底纹）", "未捕获到拖动中截图");
    }
    tree = await api.uiTree();
    report.check(cardOf(tree, "迁移节点") === null, "步骤 15 迁移成功后工作区中「迁移节点」消失");
    await backToRoot();
    tree = await api.uiTree();
    report.check(cardOf(tree, "迁移节点") !== null, "步骤 15 面包屑落点迁移成功：根画布出现「迁移节点」");
  }
  await snap("step15-back-to-root");

  report.section("S16 F1 含外部边的节点集（步骤 16）");
  {
    await clearHover();
    tree = await api.uiTree();
    const src = cardOf(tree, "账号 A");
    const dst = cardOf(tree, "工作区");
    if (!src || !dst) throw new Error("S16: card not found");
    const targetBounds = { ...dst.bounds };
    const to = { x: targetBounds.left + 30, y: targetBounds.top + 30 };
    const { png } = await altDrag(ui.boundsCenter(src), to, {
      label: "step16-forbid",
      score: (frame) => ringTint(frame, targetBounds).red,
      minScore: RING_TINT_MIN,
    });
    if (png) {
      const tint = ringTint(png, targetBounds);
      savePng(png, "step16-drag-hold-forbid");
      report.check(
        tint.red >= RING_TINT_MIN && tint.red > tint.green,
        "F1 拖到画布数据节点上时落点显示“禁止”高亮（error 描边）",
        `green=${tint.green} red=${tint.red}`,
      );
    } else {
      report.check(false, "F1 拖到画布数据节点上时落点显示“禁止”高亮（error 描边）", "未捕获到拖动中截图");
    }
    const shown = await ui
      .waitForTextStable("选中的节点与未选中的节点存在连线，无法迁移；如需迁移请将相关节点一并选中", { timeout: 6000 })
      .then(() => true)
      .catch(() => false);
    report.check(shown, "F1 松开后提示「选中的节点与未选中的节点存在连线，无法迁移；如需迁移请将相关节点一并选中」");
    await snap("f1-snackbar-external-edges");
    tree = await api.uiTree();
    report.check(cardOf(tree, "账号 A") !== null, "F1 未迁移：节点「账号 A」仍在根画布");
    // 非法迁移按画布内移动持久化，账号 A 落在工作区卡片上；挪回空白区以免遮挡后续操作。
    await moveNodeTo("账号 A", ACCOUNT_A_PARK);
  }

  report.section("S17 F2 影子节点（步骤 17）");
  {
    await enterSubCanvas("工作区", { expectText: "节点 C" });
    tree = await api.uiTree();
    const shadow = cardOf(tree, "账号 A");
    const crumbRoot = crumbs(tree).find((item) => ui.subtreeText(item).trim() === "根画布");
    if (!shadow || !crumbRoot) throw new Error("S17: shadow or crumb not found");
    const crumbBounds = { ...crumbRoot.bounds };
    const { png } = await altDrag(ui.boundsCenter(shadow), ui.boundsCenter(crumbRoot), {
      label: "step17-forbid-crumb",
      score: (frame) => crumbTint(frame, crumbBounds).red,
      minScore: CRUMB_TINT_MIN,
    });
    if (png) {
      const tint = crumbTint(png, crumbBounds);
      savePng(png, "step17-drag-hold-crumb-forbid");
      report.check(
        tint.red >= CRUMB_TINT_MIN && tint.red > tint.green,
        "F2 拖到面包屑祖先片段上时片段显示“禁止”高亮（error 底纹）",
        `green=${tint.green} red=${tint.red}`,
      );
    } else {
      report.check(false, "F2 拖到面包屑祖先片段上时片段显示“禁止”高亮（error 底纹）", "未捕获到拖动中截图");
    }
    const shown = await ui
      .waitForTextStable("选中的节点包含影子节点，影子节点不能跨画布迁移", { timeout: 6000 })
      .then(() => true)
      .catch(() => false);
    report.check(shown, "F2 松开后提示「选中的节点包含影子节点，影子节点不能跨画布迁移」");
    await snap("f2-snackbar-shadow");
    tree = await api.uiTree();
    report.check(cardOf(tree, "账号 A") !== null, "F2 未迁移：影子节点「账号 A」仍在工作区");
    // 影子被持久化到面包屑下方会遮住「根画布」片段；挪回空白区以便返回根画布。
    await moveNodeTo("账号 A", SHADOW_PARK);
  }

  report.section("S18 F3 画布数据节点（步骤 18）");
  {
    await backToRoot();
    await clearHover();
    tree = await api.uiTree();
    const src = cardOf(tree, "工作区");
    const dst = cardOf(tree, "参考区");
    if (!src || !dst) throw new Error("S18: card not found");
    const to = { x: dst.bounds.left + 30, y: dst.bounds.top + 30 };
    const draggedFallback = { left: to.x - 120, top: to.y - 60, width: 240, height: 120 };
    // 落点卡片（被拖拽节点自身）会按吸附网格移动，评分区域取外扩范围以保证拖动中的红色描边可被计分。
    const dragSearchRegion = {
      left: draggedFallback.left - 30,
      top: draggedFallback.top - 30,
      width: draggedFallback.width + 60,
      height: draggedFallback.height + 60,
    };
    const { png, midTree } = await altDrag(ui.boundsCenter(src), to, {
      label: "step18-forbid-canvas",
      score: (frame) => countPixels(frame, dragSearchRegion, isErrorRed),
      minScore: RING_TINT_MIN,
      needTree: true,
    });
    if (png) {
      // 迁移目标由 DOM 命中顺序决定：被拖拽的画布数据节点自身先命中，
      // 因此红色高亮出现在被拖拽节点上（取拖动中离落点最近的卡片）。
      const candidates = midTree ? cardGroups(midTree) : [];
      let dragged = null;
      for (const card of candidates) {
        const c = ui.boundsCenter(card);
        const d = Math.hypot(c.x - to.x, c.y - to.y);
        if (!dragged || d < dragged.d) dragged = { card, d };
      }
      const ringBounds = dragged && dragged.d < 160 ? dragged.card.bounds : draggedFallback;
      const tint = ringTint(png, ringBounds);
      savePng(png, "step18-drag-hold-forbid-canvas");
      report.check(
        tint.red >= RING_TINT_MIN && tint.red > tint.green,
        "F3 拖拽画布数据节点时显示“禁止”高亮（error 描边）",
        `green=${tint.green} red=${tint.red}`,
      );
    } else {
      report.check(false, "F3 拖拽画布数据节点时显示“禁止”高亮（error 描边）", "未捕获到拖动中截图");
    }
    const shown = await ui
      .waitForTextStable("选中的节点包含画布数据节点，画布数据节点不能跨画布迁移", { timeout: 6000 })
      .then(() => true)
      .catch(() => false);
    report.check(shown, "F3 松开后提示「选中的节点包含画布数据节点，画布数据节点不能跨画布迁移」");
    await snap("f3-snackbar-canvas-node");
    tree = await api.uiTree();
    report.check(cardOf(tree, "工作区") !== null, "F3 未迁移：画布数据节点「工作区」仍在根画布");
    await deselect();
  }

  report.section("S19 F5 后端兜底校验（UI 不可触达，不覆盖）");
  console.log("  (skip) F1/F2/F3 的提示文案与前端预检 i18n 一致，UI 正常路径不会绕过前端校验触达后端兜底错误码");
}

let fatal = null;
try {
  prepareCaseOutput(OUTPUT_DIR);
  prepareDataDir(DATA_DIR, "base");
  await withApp({ dataDir: DATA_DIR, logFile: path.join(OUTPUT_DIR, "app.log") }, async () => {
    try {
      await main();
    } catch (error) {
      await snap("fatal").catch(() => {});
      throw error;
    }
  });
} catch (error) {
  fatal = error;
  report.check(false, "用例执行中断", error.message);
} finally {
  finalizeReport(report, OUTPUT_DIR, fatal);
}
