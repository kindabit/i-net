// case_006 连线与边管理。
//
// 流程：复制 base fixture → 启动应用 → 解锁（lastScene 恢复根画布）
// → 边右键菜单与编辑对话框（标题「边一」详情「详情一」）→ 同向重复连接（不新增边）
// → 重开验证标题/详情保留 → 反向替换（继承标题与详情、箭头换向）
// → 删除边（无断连、无确认框）与 F4 旧中点不再弹出菜单
// → F1 非法连接（自环、两端同连接桩，前端拦截）与全屏像素基线比对
// → 重建连接 → 创建画布数据节点「新画布」→ 跨画布连接产生影子节点
// → 子画布内创建「节点 C」并连接影子 → 删除产生边的断连确认（取消 / 确认）
// → 影子级联消失 → 重建产生边与影子→节点 C 边并验证影子节点按钮
// → 删除影子的断连确认（取消 / 确认）→ 回根画布确认产生边消失 → 输出 PASS/FAIL 报告。
//
// 脚本规范要点：
// - 边没有独立 DOM 锚点：无标签时按贝塞尔曲线 t=0.5 点右键，且优先选择远离节点卡片的
//   曲线点（卡片外扩危险区会遮挡边命中区，hover 工具条会截获右键，定位前先清场）；
//   有标签后改为右键标签文本中心；
// - 同向重复连接与反向替换均以「标签数量 + 边右键菜单可命中」断言；反向替换的箭头换向以
//   目标端连接桩外侧的灰度像素计数断言（替换前后对比）；
// - F1 非法连接除曲线探测外，还做全屏像素基线比对（两次非法拖拽前后截图一致）；
// - 影子链路：画布数据节点连线在子画布内产生入向影子（卡片外侧渲染虚线虚拟边），
//   删除产生边会级联删除影子与影子相连的边；判定是否弹断连确认框的依据是影子在子画布中
//   是否还有关联节点，因此 F2 前置需要重建影子→节点 C 的边（见修订记录）；
// - 对话框关闭后先等待「对话框消失 + 画布内容可读」再操作，规避遮罩期间 UI 树被过滤；
// - 应用生命周期由 withApp 包装，保证无论成败都经 POST /shutdown 收尾；
// - 关键步骤截图存 output；流程中断时额外截取 fatal 截图并写入 report.json。
//
// 运行方式：在项目根目录执行 `node e2e\script\case_006\case_006.js`

import { execSync } from "node:child_process";
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

/** 画布数据节点「新画布」的落点（节点左上角，窗口物理像素） */
const CANVAS_DROP = { x: 1150, y: 950 };
/** 子画布中「节点 C」的落点（窗口物理像素） */
const NODE_C_DROP = { x: 950, y: 950 };
/** 清场坐标（两个画布内均为空白区，窗口物理像素） */
const NEUTRAL_POINT = { x: 1400, y: 300 };
/** 取消选中用的画布空白点（窗口物理像素） */
const BLANK_POINT = { x: 320, y: 600 };

/** 连接桩检索的卡片外扩范围（窗口物理像素） */
const HANDLE_SEARCH_MARGIN = 45;
/** 边命中点危险区：卡片外扩（含 hover 工具条桥接区） */
const CARD_SIDE_MARGIN = 12;
const CARD_TOP_MARGIN = 14;

const report = new Report("case_006 连线与边管理");

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
 * 查找 UI 树中文本精确匹配的第 index 个 #text 节点。
 * @param {object[]} tree UI 树顶层节点数组。
 * @param {string} text 目标文本。
 * @param {number} [index=0] 出现序号（按 top、left 升序）。
 * @returns {object|null} 命中的文本节点；未命中时为 null。
 */
function findTextNode(tree, text, index = 0) {
  return textNodes(tree, text)[index] ?? null;
}

/**
 * 返回 UI 树中全部 vue-flow 节点卡片（role=group、带 data-id 且 aria-roledescription=node 的容器）。
 * 边同样以 role=group + data-id 渲染，故必须按 aria-roledescription 过滤。
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
 * 收集指定卡片附近（外扩 HANDLE_SEARCH_MARGIN）的连接桩元素。
 * @param {object[]} tree UI 树顶层节点数组。
 * @param {object} card 节点卡片。
 * @returns {object[]} 连接桩节点数组。
 */
function handlesNear(tree, card) {
  const m = HANDLE_SEARCH_MARGIN;
  return flattenNodes(tree).filter(
    (node) =>
      node.attrs?.["data-handlepos"] &&
      node.bounds &&
      node.bounds.width > 0 &&
      node.bounds.left + node.bounds.width > card.bounds.left - m &&
      node.bounds.left < card.bounds.left + card.bounds.width + m &&
      node.bounds.top + node.bounds.height > card.bounds.top - m &&
      node.bounds.top < card.bounds.top + card.bounds.height + m,
  );
}

/**
 * 取指定卡片指定方向的连接桩。
 * @param {object[]} tree UI 树顶层节点数组。
 * @param {object} card 节点卡片。
 * @param {string} pos 连接桩方向（top/right/bottom/left）。
 * @returns {object|null} 命中的连接桩；未命中时为 null。
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
 * 按「源节点标题 + 源 handle 方向 + 目标节点标题 + 目标 handle 方向」计算边的贝塞尔几何点。
 * @param {object[]} tree UI 树顶层节点数组。
 * @param {{sourceText: string, sourcePos: string, targetText: string, targetPos: string, sourceIndex?: number, targetIndex?: number}} spec 边端点描述。
 * @returns {{pts: object, sCard: object, tCard: object, sH: object, tH: object}} 几何点与端点元素。
 */
function edgeMidpoint(tree, spec) {
  const sCard = cardOf(tree, spec.sourceText, spec.sourceIndex ?? 0);
  const tCard = cardOf(tree, spec.targetText, spec.targetIndex ?? 0);
  if (!sCard || !tCard) throw new Error(`edgeMidpoint: card not found ${JSON.stringify(spec)}`);
  const sH = handleOf(tree, sCard, spec.sourcePos);
  const tH = handleOf(tree, tCard, spec.targetPos);
  if (!sH || !tH) throw new Error(`edgeMidpoint: handle not found ${JSON.stringify(spec)}`);
  return {
    pts: bezPoints(ui.boundsCenter(sH), ui.boundsCenter(tH), spec.sourcePos, spec.targetPos),
    sCard,
    tCard,
    sH,
    tH,
  };
}

/**
 * 生成全部节点卡片的右键危险区（卡片外扩；顶部额外考虑 hover 工具条桥接区）。
 * @param {object[]} tree UI 树顶层节点数组。
 * @returns {{left: number, top: number, right: number, bottom: number}[]} 危险区矩形数组。
 */
function dangerRects(tree) {
  return cardGroups(tree).map((g) => ({
    left: g.bounds.left - CARD_SIDE_MARGIN,
    top: g.bounds.top - CARD_TOP_MARGIN,
    right: g.bounds.left + g.bounds.width + CARD_SIDE_MARGIN,
    bottom: g.bounds.top + g.bounds.height + CARD_SIDE_MARGIN,
  }));
}

/**
 * 判断点是否位于全部危险区之外。
 * @param {{x: number, y: number}} p 点坐标。
 * @param {{left: number, top: number, right: number, bottom: number}[]} rects 危险区数组。
 * @returns {boolean} 位于全部危险区之外时为 true。
 */
function pointClear(p, rects) {
  return rects.every((r) => p.x < r.left || p.x > r.right || p.y < r.top || p.y > r.bottom);
}

/**
 * 在边曲线上挑选右键探测点：优先取曲线上远离全部节点卡片的点，无净空点时取净空最大者。
 * @param {object} pts 贝塞尔几何点。
 * @param {{left: number, top: number, right: number, bottom: number}[]} rects 危险区数组。
 * @returns {{point: {x: number, y: number}, t: number, clearance: number}} 探测点信息。
 */
function pickProbePoint(pts, rects) {
  const candidates = [0.5, 0.46, 0.54, 0.42, 0.58, 0.38, 0.62, 0.34, 0.66, 0.3, 0.7, 0.26, 0.74, 0.22, 0.78, 0.18, 0.82];
  let best = null;
  for (const t of candidates) {
    const p = bezAt(pts, t);
    const clearance = Math.min(
      ...rects.map((r) => Math.max(r.left - p.x, p.x - r.right, r.top - p.y, p.y - r.bottom)),
    );
    if (pointClear(p, rects)) return { point: p, t, clearance };
    if (!best || clearance > best.clearance) best = { point: p, t, clearance };
  }
  return best;
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
 * 在指定坐标右键并等待菜单渲染。
 * @param {{x: number, y: number}} point 目标坐标。
 * @returns {Promise<void>} 无返回值。
 */
async function rightClickAt(point) {
  await api.postInput([api.mouseMove(point.x, point.y), api.mouseClick("right", 1)]);
  await sleep(450);
}

/**
 * 判断边右键菜单是否可见（以菜单项「编辑边」为锚点）。
 * @returns {Promise<boolean>} 可见时为 true。
 */
async function edgeMenuVisible() {
  const tree = await api.uiTree();
  return ui.findByText(tree, "编辑边", { exact: true }) !== null;
}

/**
 * 关闭边右键菜单（Esc）并等待其消失。
 * @returns {Promise<void>} 无返回值。
 */
async function closeMenu() {
  if (!(await edgeMenuVisible())) return;
  await api.postInput([api.keyClick(["Escape"])]);
  await waitForCondition(async () => !(await edgeMenuVisible()), {
    timeout: 3000,
    interval: 150,
    label: "边右键菜单关闭",
  });
}

/**
 * 点击边右键菜单中的指定菜单项。
 * @param {string} text 菜单项文本（「编辑边」或「删除边」）。
 * @returns {Promise<void>} 无返回值。
 */
async function clickMenuItem(text) {
  const item = await waitForCondition(
    async () => ui.findByText(await api.uiTree(), text, { exact: true }),
    { timeout: 4000, interval: 150, label: `菜单项「${text}」` },
  );
  await ui.clickNode(item);
  await sleep(400);
}

/**
 * 点击对话框内按钮（对话框内查找以避免与嵌套对话框的同名按钮混淆）。
 * @param {string} dialogTitle 对话框标题/内容片段。
 * @param {string} buttonText 按钮文本。
 * @param {object} [options] 可选参数。
 * @param {boolean} [options.exact=false] 是否精确匹配按钮子树文本。
 * @returns {Promise<void>} 无返回值。
 */
async function clickDialogButton(dialogTitle, buttonText, { exact = false } = {}) {
  const tree = await api.uiTree();
  const dialog = ui.findDialog(tree, dialogTitle);
  if (!dialog) {
    throw new Error(`clickDialogButton: dialog "${dialogTitle}" not found`);
  }
  const button = ui.findButton(tree, buttonText, { root: dialog, exact });
  if (!button) {
    throw new Error(`clickDialogButton: button "${buttonText}" not found in "${dialogTitle}"`);
  }
  await ui.clickNode(button);
}

/**
 * 从起点拖到终点建立/更新边。
 * @param {{x: number, y: number}} from 起点（窗口物理像素）。
 * @param {{x: number, y: number}} to 终点（窗口物理像素）。
 * @returns {Promise<void>} 无返回值。
 */
async function connect(from, to) {
  await api.postInput([api.mouseDrag(from, to)]);
  await sleep(1200);
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
  const node = findTextNode(tree, text, index);
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
 * 在边的曲线上弹出右键菜单并保持打开（带重试，规避后端写入与渲染延迟）。
 * @param {object} spec 边端点描述（见 edgeMidpoint）。
 * @param {string} label 等待条件名（用于错误消息）。
 * @param {object} [options] 可选参数。
 * @param {number} [options.timeout=8000] 超时毫秒数。
 * @returns {Promise<boolean>} 菜单已弹出时返回 true（超时抛异常）。
 */
async function openEdgeMenu(spec, label, { timeout = 8000 } = {}) {
  await waitForCondition(
    async () => {
      const tree = await api.uiTree();
      const info = edgeMidpoint(tree, spec);
      const pick = pickProbePoint(info.pts, dangerRects(tree));
      await clearHover();
      await api.postInput([api.mouseMove(pick.point.x, pick.point.y)]);
      await sleep(380);
      await rightClickAt(pick.point);
      if (await edgeMenuVisible()) {
        console.log(`  (edge menu) ${label} t=${pick.t.toFixed(2)} @ (${Math.round(pick.point.x)},${Math.round(pick.point.y)})`);
        return true;
      }
      return null;
    },
    { timeout, interval: 500, label: `边菜单「${label}」` },
  );
  return true;
}

/**
 * 在边曲线上探测右键菜单并自动关闭；用于存在性/不存在性断言。
 * @param {object} spec 边端点描述（见 edgeMidpoint）。
 * @param {string} label 探测标签（用于日志）。
 * @param {object} [options] 可选参数。
 * @param {number[]} [options.ts=[0.5]] 附加探测参数列表。
 * @returns {Promise<boolean>} 是否弹出过菜单。
 */
async function probeEdgeMenu(spec, label, { ts = [0.5] } = {}) {
  const tree = await api.uiTree();
  const info = edgeMidpoint(tree, spec);
  const rects = dangerRects(tree);
  const pick = pickProbePoint(info.pts, rects);
  const tried = [pick.t, ...ts.filter((t) => t !== pick.t)];
  let anyMenu = false;
  for (const t of tried) {
    const point = bezAt(info.pts, t);
    await clearHover();
    await api.postInput([api.mouseMove(point.x, point.y)]);
    await sleep(380);
    await rightClickAt(point);
    const visible = await edgeMenuVisible();
    console.log(`  (probe ${label}) t=${t.toFixed(2)}: menu=${visible}`);
    if (visible) {
      anyMenu = true;
      await closeMenu();
    }
  }
  return anyMenu;
}

/**
 * 等待边存在（曲线上可弹出右键菜单）。
 * @param {object} spec 边端点描述（见 edgeMidpoint）。
 * @param {string} label 等待条件名。
 * @param {object} [options] 可选参数。
 * @param {number} [options.timeout=10000] 超时毫秒数。
 * @returns {Promise<boolean>} 边存在时返回 true。
 */
async function waitEdgeExists(spec, label, { timeout = 10000 } = {}) {
  await waitForCondition(
    async () => (await probeEdgeMenu(spec, label, { ts: [0.46, 0.54, 0.42] })) || null,
    { timeout, interval: 600, label: `边存在「${label}」` },
  );
  return true;
}

/**
 * 等待边不存在（曲线上不再弹出右键菜单）。
 * @param {object} spec 边端点描述（见 edgeMidpoint）。
 * @param {string} label 等待条件名。
 * @param {object} [options] 可选参数。
 * @param {number} [options.timeout=8000] 超时毫秒数。
 * @returns {Promise<boolean>} 边已消失时返回 true。
 */
async function waitEdgeGone(spec, label, { timeout = 8000 } = {}) {
  await waitForCondition(
    async () => ((await probeEdgeMenu(spec, label, { ts: [0.46, 0.54] })) ? null : true),
    { timeout, interval: 600, label: `边消失「${label}」` },
  );
  return true;
}

/**
 * 点击画布空白处取消选中并复位 hover 状态。
 * @returns {Promise<void>} 无返回值。
 */
async function deselect() {
  await api.postInput([api.mouseMove(BLANK_POINT.x, BLANK_POINT.y), api.mouseClick("left", 1)]);
  await sleep(700);
}

/**
 * 统计截图中 A 端连接桩右侧与 B 端连接桩左侧小窗口内的灰度（#b1b1b7 邻域）像素数，
 * 用于判断箭头位于哪一端（箭头位于目标端）。
 * @param {{x: number, y: number}} pointRight 取右侧窗口的参考点。
 * @param {{x: number, y: number}} pointLeft 取左侧窗口的参考点。
 * @returns {Promise<{right: number, left: number}>} 两侧灰度像素数。
 */
async function grayStats(pointRight, pointLeft) {
  const png = image.decodePng(await api.screenshot());
  const count = (region) => {
    let n = 0;
    const left = Math.max(0, Math.floor(region.left));
    const top = Math.max(0, Math.floor(region.top));
    const right = Math.min(png.width, Math.ceil(region.left + region.width));
    const bottom = Math.min(png.height, Math.ceil(region.top + region.height));
    for (let y = top; y < bottom; y++) {
      for (let x = left; x < right; x++) {
        const i = (png.width * y + x) << 2;
        const [r, g, b] = [png.data[i], png.data[i + 1], png.data[i + 2]];
        if (r >= 120 && r <= 215 && Math.abs(r - g) <= 14 && Math.abs(g - b) <= 20) n += 1;
      }
    }
    return n;
  };
  return {
    right: count({ left: pointRight.x + 3, top: pointRight.y - 10, width: 12, height: 20 }),
    left: count({ left: pointLeft.x - 15, top: pointLeft.y - 10, width: 12, height: 20 }),
  };
}

/**
 * 统计 PNG 指定区域内的深色像素数量（用于断言影子节点的虚线虚拟边）。
 * @param {import("pngjs").PNG} png 整窗截图 PNG。
 * @param {{left: number, top: number, width: number, height: number}} region 统计区域。
 * @returns {number} 深色像素数量。
 */
function darkPixelsIn(png, region) {
  let count = 0;
  const left = Math.max(0, Math.floor(region.left));
  const top = Math.max(0, Math.floor(region.top));
  const right = Math.min(png.width, Math.ceil(region.left + region.width));
  const bottom = Math.min(png.height, Math.ceil(region.top + region.height));
  for (let y = top; y < bottom; y++) {
    for (let x = left; x < right; x++) {
      const i = (png.width * y + x) << 2;
      if (png.data[i] < 150 && png.data[i + 1] < 150 && png.data[i + 2] < 150) count += 1;
    }
  }
  return count;
}

/**
 * 读取系统剪贴板文本（仅用于验证脚本自身写入的测试值）。
 * @returns {string|null} 剪贴板文本；读取失败时为 null。
 */
function readClipboard() {
  try {
    return execSync(
      'powershell -NoProfile -Command "[Console]::OutputEncoding=[Text.Encoding]::UTF8; Get-Clipboard -Raw"',
      { encoding: "utf8", timeout: 15000 },
    ).replace(/\r?\n$/, "");
  } catch (error) {
    console.log(`  (clipboard read failed: ${error.message})`);
    return null;
  }
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
 * 双击画布数据节点进入其子画布，并等待子画布内容出现（根画布独有文本消失）。
 * @param {string} canvasText 画布数据节点标题。
 * @param {string} expectText 子画布内应出现的文本。
 * @returns {Promise<void>} 无返回值。
 */
async function enterSubCanvas(canvasText, expectText) {
  await ui.clickText(canvasText, { count: 2 });
  await waitForCondition(
    async () => {
      const t = await api.uiTree();
      return ui.hasText(t, expectText) && !ui.hasText(t, "备注 B") ? t : null;
    },
    { timeout: 15000, interval: 300, label: `进入子画布「${canvasText}」` },
  );
  await sleep(1000);
}

/**
 * 点击面包屑「根画布」返回根画布。
 * @returns {Promise<void>} 无返回值。
 */
async function backToRoot() {
  await ui.clickText("根画布");
  await ui.waitForText("备注 B", { timeout: 15000 });
  await sleep(1200);
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
 * 打开指定边的编辑对话框，用剪贴板读取「标题」「详情」输入框内容后取消关闭。
 * @param {object} spec 边端点描述（见 edgeMidpoint）。
 * @param {string} label 日志标签。
 * @returns {Promise<{title: string|null, description: string|null}>} 读取到的标题与详情。
 */
async function readEdgeFieldsViaDialog(spec, label) {
  await openEdgeMenu(spec, label);
  await clickMenuItem("编辑边");
  await ui.waitForDialog("编辑边", { timeout: 8000 });
  await sleep(700);
  await ui.clickEditable("标题");
  await api.postInput([api.keyClick(["Control", "a"]), api.keyClick(["Control", "c"])]);
  await sleep(500);
  const title = readClipboard();
  await ui.clickEditable("详情");
  await api.postInput([api.keyClick(["Control", "a"]), api.keyClick(["Control", "c"])]);
  await sleep(500);
  const description = readClipboard();
  await clickDialogButton("编辑边", "取消");
  await ui.waitForDialogGone("编辑边", { timeout: 8000 });
  await sleep(500);
  return { title, description };
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

  const abSpec = { sourceText: "账号 A", sourcePos: "right", targetText: "备注 B", targetPos: "left" };

  report.section("S1 边右键菜单（步骤 1）");
  await openEdgeMenu(abSpec, "A-B 中点");
  let tree = await api.uiTree();
  report.check(
    ui.findByText(tree, "编辑边", { exact: true }) !== null,
    "步骤 1 右键边中点弹出菜单且含「编辑边」",
  );
  report.check(
    ui.findByText(tree, "删除边", { exact: true }) !== null,
    "步骤 1 菜单含「删除边」",
  );
  await snap("step1-edge-menu");

  report.section("S2 编辑边对话框与标签（步骤 2-3）");
  await clickMenuItem("编辑边");
  await ui.waitForDialog("编辑边", { timeout: 8000 });
  await sleep(600);
  tree = await api.uiTree();
  report.check(ui.findDialog(tree, "编辑边") !== null, "步骤 2 打开「编辑边」对话框");
  report.check(ui.findEditable(tree, "标题") !== null, "步骤 2 对话框包含「标题」输入框");
  report.check(ui.findEditable(tree, "详情") !== null, "步骤 2 对话框包含「详情」输入框");
  await snap("step2-edit-edge-dialog");
  await ui.typeIntoEditable("标题", "边一");
  await ui.typeIntoEditable("详情", "详情一");
  await clickDialogButton("编辑边", "确认");
  await ui.waitForDialogGone("编辑边", { timeout: 8000 });
  await waitForCondition(async () => textNodes(await api.uiTree(), "边一").length === 1, {
    timeout: 6000,
    interval: 200,
    label: "画布出现边标签「边一」",
  });
  report.check(true, "步骤 3 确认后对话框关闭且画布出现边标签「边一」");
  await snap("step3-edge-labeled");

  report.section("S3 同向重复连接（步骤 4）");
  {
    const h1 = await handleCenterOf("账号 A", "right");
    const h2 = await handleCenterOf("备注 B", "left");
    await connect(h1.center, h2.center);
    await deselect();
    tree = await api.uiTree();
    report.check(textNodes(tree, "边一").length === 1, "步骤 4 同向重复连接后标签「边一」仍只有一个");
    report.check(
      await probeEdgeMenu(abSpec, "同向重复连接后"),
      "步骤 4 同向重复连接后边仍存在（右键中点仍弹出菜单）",
    );
  }
  await snap("step4-same-direction");

  report.section("S4 重开验证标题与详情保留（步骤 5）");
  await clearHover();
  const label = findTextNode(await api.uiTree(), "边一");
  report.check(label !== null, "步骤 5 画布存在标签「边一」");
  await rightClickAt(ui.boundsCenter(label));
  report.check(await edgeMenuVisible(), "步骤 5 右键标签文本弹出边菜单");
  const retained = await (async () => {
    await clickMenuItem("编辑边");
    await ui.waitForDialog("编辑边", { timeout: 8000 });
    await sleep(700);
    await ui.clickEditable("标题");
    await api.postInput([api.keyClick(["Control", "a"]), api.keyClick(["Control", "c"])]);
    await sleep(500);
    const title = readClipboard();
    await ui.clickEditable("详情");
    await api.postInput([api.keyClick(["Control", "a"]), api.keyClick(["Control", "c"])]);
    await sleep(500);
    const description = readClipboard();
    await snap("step5-retained-fields");
    await clickDialogButton("编辑边", "取消");
    await ui.waitForDialogGone("编辑边", { timeout: 8000 });
    await sleep(500);
    return { title, description };
  })();
  report.check(retained.title === "边一", "步骤 5 对话框「标题」仍为 边一（同向更新未丢失数据）", `clipboard=${JSON.stringify(retained.title)}`);
  report.check(retained.description === "详情一", "步骤 5 对话框「详情」仍为 详情一（同向更新未丢失数据）", `clipboard=${JSON.stringify(retained.description)}`);

  report.section("S5 反向替换（步骤 6）");
  {
    const h1 = await handleCenterOf("备注 B", "left");
    const h2 = await handleCenterOf("账号 A", "right");
    const beforeTree = await api.uiTree();
    const aRight = ui.boundsCenter(handleOf(beforeTree, cardOf(beforeTree, "账号 A"), "right"));
    const bLeft = ui.boundsCenter(handleOf(beforeTree, cardOf(beforeTree, "备注 B"), "left"));
    const grayBefore = await grayStats(aRight, bLeft);
    await connect(h1.center, h2.center);
    await deselect();
    tree = await api.uiTree();
    report.check(ui.findByRole(tree, "dialog").length === 0, "步骤 6 反向替换未弹出确认框（此时无影子）");
    report.check(textNodes(tree, "边一").length === 1, "步骤 6 反向替换后标签「边一」保留");
    const revSpec = { sourceText: "备注 B", sourcePos: "left", targetText: "账号 A", targetPos: "right" };
    await waitEdgeExists(revSpec, "反向替换后的 B->A 边");
    report.check(true, "步骤 6 反向替换后边按新方向存在（B->A 曲线可命中）");
    const grayAfter = await grayStats(aRight, bLeft);
    report.check(
      grayBefore.right < grayBefore.left && grayAfter.right > grayAfter.left,
      "步骤 6 箭头由 B 端换到 A 端（两端外侧灰度像素对比）",
      `before=${JSON.stringify(grayBefore)} after=${JSON.stringify(grayAfter)}`,
    );
    report.check(
      grayAfter.right - grayBefore.right >= 15 && grayBefore.left - grayAfter.left >= 15,
      "步骤 6 换向前后 A 端灰度显著增加、B 端显著减少",
      `before=${JSON.stringify(grayBefore)} after=${JSON.stringify(grayAfter)}`,
    );
    const inherited = await readEdgeFieldsViaDialog(revSpec, "换向后重开编辑边");
    report.check(inherited.title === "边一", "步骤 6 替换继承标题「边一」", `clipboard=${JSON.stringify(inherited.title)}`);
    report.check(inherited.description === "详情一", "步骤 6 替换继承详情「详情一」", `clipboard=${JSON.stringify(inherited.description)}`);
  }
  await snap("step6-reversed");

  report.section("S6 删除边与 F4（步骤 7、F4）");
  await clearHover();
  const label2 = findTextNode(await api.uiTree(), "边一");
  await rightClickAt(ui.boundsCenter(label2));
  await clickMenuItem("删除边");
  await waitForCondition(async () => textNodes(await api.uiTree(), "边一").length === 0, {
    timeout: 6000,
    interval: 200,
    label: "标签「边一」消失",
  });
  tree = await api.uiTree();
  report.check(ui.findByRole(tree, "dialog").length === 0, "步骤 7 删除边未弹出确认框（无断连）");
  report.check(ui.hasText(tree, "账号 A") && ui.hasText(tree, "备注 B"), "步骤 7 删除边后「账号 A」「备注 B」仍在");
  report.check(
    !(await probeEdgeMenu(abSpec, "F4 旧中点")),
    "F4 删除后原边中点位置不再弹出菜单",
  );
  await snap("step7-edge-deleted");

  report.section("S7 非法连接 F1（步骤 8-9）");
  let f1Baseline = null;
  {
    await deselect();
    await clearHover();
    await sleep(400);
    f1Baseline = image.decodePng(await api.screenshot());
    await snap("f1-baseline");
  }
  {
    const h1 = await handleCenterOf("账号 A", "right");
    const h2 = await handleCenterOf("账号 A", "top");
    await connect(h1.center, h2.center);
    await deselect();
    tree = await api.uiTree();
    const card = cardOf(tree, "账号 A");
    const loopPts = bezPoints(
      ui.boundsCenter(handleOf(tree, card, "right")),
      ui.boundsCenter(handleOf(tree, card, "top")),
      "right",
      "top",
    );
    const probe = pickProbePoint(loopPts, dangerRects(tree));
    await clearHover();
    await api.postInput([api.mouseMove(probe.point.x, probe.point.y)]);
    await sleep(380);
    await rightClickAt(probe.point);
    const menuShown = await edgeMenuVisible();
    if (menuShown) await closeMenu();
    report.check(!menuShown, "步骤 8 自环拖拽不产生新边（自环曲线探测无菜单）", `t=${probe.t.toFixed(2)}`);
  }
  await snap("f1-self-loop");
  {
    const h1 = await handleCenterOf("账号 A", "top");
    const h2 = await handleCenterOf("备注 B", "top");
    const sameSpec = { sourceText: "账号 A", sourcePos: "top", targetText: "备注 B", targetPos: "top" };
    await connect(h1.center, h2.center);
    await deselect();
    report.check(
      !(await probeEdgeMenu(sameSpec, "同 handle top->top", { ts: [0.46, 0.54, 0.4] })),
      "步骤 9 两端相同连接桩拖拽不产生新边",
    );
  }
  await snap("f1-same-handle");
  {
    await deselect();
    await clearHover();
    await sleep(500);
    const after = image.decodePng(await api.screenshot());
    const cmp = image.comparePng(f1Baseline, after, {
      maxDiffRatio: 0.0002,
      diffFile: path.join(OUTPUT_DIR, "diff-f1.png"),
    });
    report.check(
      cmp.pass,
      "F1 两次非法拖拽前后画布截图一致（无新边/无副作用）",
      `diffCount=${cmp.diffCount} diffRatio=${cmp.diffRatio}`,
    );
  }

  report.section("S8 重建连接（步骤 10）");
  {
    const h1 = await handleCenterOf("账号 A", "right");
    const h2 = await handleCenterOf("备注 B", "left");
    await connect(h1.center, h2.center);
    await waitEdgeExists(abSpec, "重建的 A->B 边");
    tree = await api.uiTree();
    report.check(textNodes(tree, "边一").length === 0, "步骤 10 重建的边无标签");
    report.check(true, "步骤 10 重建连接后边重新出现（右键中点可弹出菜单）");
  }
  await snap("step10-rebuilt");

  report.section("S9 创建画布数据节点（步骤 11）");
  {
    await dragCreateNode("拖拽创建画布数据节点", CANVAS_DROP);
    tree = await api.uiTree();
    report.check(ui.hasText(tree, "新画布"), "步骤 11 画布出现标题「新画布」的画布数据节点");
    report.check(
      ui.findAttr(tree, "title", "拖拽创建画布数据节点") === null,
      "步骤 11 创建成功后模板面板关闭",
    );
  }
  await snap("step11-canvas-node");

  const toCanvasSpec = { sourceText: "账号 A", sourcePos: "right", targetText: "新画布", targetPos: "top" };

  report.section("S10 跨画布连接与影子出现（步骤 12）");
  {
    const h1 = await handleCenterOf("账号 A", "right");
    const h2 = await handleCenterOf("新画布", "top");
    await connect(h1.center, h2.center);
    await waitEdgeExists(toCanvasSpec, "A->新画布");
    report.check(true, "步骤 12 「账号 A」→「新画布」连线建立成功");
    await deselect();
  }
  await snap("step12-edge-to-canvas");
  await enterSubCanvas("新画布", "账号 A");
  {
    tree = await api.uiTree();
    const shadowCard = cardOf(tree, "账号 A");
    report.check(shadowCard !== null, "步骤 12 进入「新画布」子画布后出现影子节点「账号 A」");
    report.check(!ui.hasText(tree, "备注 B"), "步骤 12 子画布内不含根画布节点「备注 B」");
    const png = image.decodePng(await api.screenshot());
    const dark = shadowCard
      ? darkPixelsIn(png, {
          left: shadowCard.bounds.left - 80,
          top: shadowCard.bounds.top + shadowCard.bounds.height / 2 - 15,
          width: 80,
          height: 30,
        })
      : 0;
    report.check(dark > 50, "步骤 12 影子节点左侧渲染虚线虚拟边（深色像素断言）", `darkPixels=${dark}`);
  }
  await snap("step12-shadow-appears");

  report.section("S11 子画布创建「节点 C」（步骤 13）");
  {
    await dragCreateNode("拖拽创建数据节点", NODE_C_DROP);
    await sleep(500);
    await ui.clickText("新节点", { count: 2 });
    await ui.waitForDialog("编辑节点", { timeout: 8000 });
    await sleep(600);
    await ui.typeIntoEditable("标题", "节点 C");
    await clickDialogButton("编辑节点", "确认");
    await ui.waitForDialogGone("编辑节点", { timeout: 8000 });
    await ui.waitForText("节点 C", { timeout: 8000 });
    await sleep(700);
    report.check(ui.hasText(await api.uiTree(), "节点 C"), "步骤 13 子画布出现「节点 C」");
  }
  await snap("step13-node-c");

  const shadowToCSpec = { sourceText: "账号 A", sourcePos: "right", targetText: "节点 C", targetPos: "left" };

  report.section("S12 影子与「节点 C」连线（步骤 14）");
  {
    const h1 = await handleCenterOf("账号 A", "right");
    const h2 = await handleCenterOf("节点 C", "left");
    await connect(h1.center, h2.center);
    await waitEdgeExists(shadowToCSpec, "影子->节点 C");
    report.check(true, "步骤 14 影子节点与「节点 C」之间建立边（子画布内连线可命中）");
    await deselect();
  }
  await snap("step14-shadow-to-c");

  report.section("S13 删除产生边的断连确认（步骤 15）");
  {
    await backToRoot();
    await deselect();
    await waitEdgeExists(toCanvasSpec, "删除前确认 A->新画布");
    await openEdgeMenu(toCanvasSpec, "A->新画布 删除前");
    await clickMenuItem("删除边");
    await ui.waitForDialog("删除边将断开连接", { timeout: 8000 });
    const expectedBody = "删除此边会同时删除相关影子节点（可能跨越多级画布），以下节点将失去连接：节点 C";
    const shown = await ui.waitForTextStable(expectedBody, { timeout: 6000 }).catch(() => null);
    report.check(shown !== null, "步骤 15 弹出「删除边将断开连接」确认框且正文含失去连接的节点 C", expectedBody);
    tree = await api.uiTree();
    const dialog = ui.findDialog(tree, "删除边将断开连接");
    report.check(ui.findButton(tree, "取消", { root: dialog, exact: true }) !== null, "步骤 15 确认框含「取消」按钮");
    report.check(ui.findButton(tree, "删除边", { root: dialog, exact: true }) !== null, "步骤 15 确认按钮文案为「删除边」");
  }
  await snap("step15-disconnect-dialog");

  report.section("S14 断连确认取消 F2（步骤 16）");
  {
    await clickDialogButton("删除边将断开连接", "取消");
    await ui.waitForDialogGone("删除边将断开连接", { timeout: 6000 });
    await sleep(900);
    await deselect();
    await waitEdgeExists(toCanvasSpec, "取消后保留的 A->新画布");
    report.check(true, "F2 点「取消」后边仍存在（再次右键中点仍可弹出菜单）");
  }
  await snap("step16-cancel-kept");

  report.section("S15 断连确认删除（步骤 17）");
  {
    await openEdgeMenu(toCanvasSpec, "A->新画布 确认删除前");
    await clickMenuItem("删除边");
    await ui.waitForDialog("删除边将断开连接", { timeout: 8000 });
    await sleep(500);
    await clickDialogButton("删除边将断开连接", "删除边", { exact: true });
    await ui.waitForDialogGone("删除边将断开连接", { timeout: 8000 });
    await waitForCondition(
      async () => {
        const t = await api.uiTree();
        return !ui.hasText(t, "删除边将断开连接") && ui.hasText(t, "新画布") && ui.hasText(t, "账号 A") ? t : null;
      },
      { timeout: 6000, interval: 250, label: "确认框关闭且画布可读" },
    );
    await waitEdgeGone(toCanvasSpec, "确认删除后的 A->新画布");
    report.check(true, "步骤 17 确认后边被删除（曲线不再命中）");
    tree = await api.uiTree();
    report.check(
      ui.hasText(tree, "新画布") && ui.hasText(tree, "账号 A"),
      "步骤 17 删除边后「新画布」与「账号 A」仍存在",
    );
    await sleep(500);
  }
  await snap("step17-edge-deleted");

  report.section("S16 影子级联消失（步骤 18）");
  {
    await enterSubCanvas("新画布", "节点 C");
    tree = await api.uiTree();
    report.check(!ui.hasText(tree, "账号 A"), "步骤 18 影子节点「账号 A」随产生边级联消失");
    report.check(ui.hasText(tree, "节点 C"), "步骤 18 子画布内仍存在「节点 C」");
  }
  await snap("step18-shadow-gone");

  report.section("S17 重建产生边与影子按钮（步骤 19）");
  {
    await backToRoot();
    await deselect();
    const h1 = await handleCenterOf("账号 A", "right");
    const h2 = await handleCenterOf("新画布", "top");
    await connect(h1.center, h2.center);
    await waitEdgeExists(toCanvasSpec, "重建的 A->新画布");
    report.check(true, "步骤 19 重建「账号 A」→「新画布」的边");
    await enterSubCanvas("新画布", "账号 A");
    const { buttons } = await hoverCardActions("账号 A");
    const titles = buttons.map((b) => b.attrs.title);
    report.check(
      titles.length === 2 &&
        titles[0] === "编辑节点" &&
        titles[1] === "删除影子节点（等同于删除产生它的边）",
      "步骤 19 hover 影子节点出现「编辑节点」与「删除影子节点（等同于删除产生它的边）」两个按钮",
      JSON.stringify(titles),
    );
  }
  await snap("step19-shadow-buttons");
  {
    const h1 = await handleCenterOf("账号 A", "right");
    const h2 = await handleCenterOf("节点 C", "left");
    await connect(h1.center, h2.center);
    await waitEdgeExists(shadowToCSpec, "重建的影子->节点 C");
    report.check(true, "步骤 19b 重建影子与「节点 C」的边（删除影子断连确认的前置）");
    await deselect();
  }
  await snap("step19b-shadow-to-c");

  report.section("S18 删除影子的断连确认取消 F2（步骤 20）");
  {
    const { buttons } = await hoverCardActions("账号 A");
    const deleteButton = buttons.find((b) => b.attrs.title === "删除影子节点（等同于删除产生它的边）");
    report.check(deleteButton !== undefined, "步骤 20 影子节点删除按钮文案与普通节点不同（含「删除影子节点」）");
    await ui.clickNode(deleteButton);
    await ui.waitForDialog("删除影子节点将断开连接", { timeout: 8000 });
    const expectedBody = "删除影子节点等同于删除产生它的边，以下节点将失去连接：节点 C";
    const shown = await ui.waitForTextStable(expectedBody, { timeout: 6000 }).catch(() => null);
    report.check(shown !== null, "步骤 20 弹出「删除影子节点将断开连接」确认框且正文含「节点 C」", expectedBody);
  }
  await snap("step20-shadow-disconnect-dialog");
  {
    await clickDialogButton("删除影子节点将断开连接", "取消");
    await ui.waitForDialogGone("删除影子节点将断开连接", { timeout: 6000 });
    await waitForCondition(
      async () => {
        const t = await api.uiTree();
        return ui.hasText(t, "账号 A") && !ui.hasText(t, "删除影子节点将断开连接") ? t : null;
      },
      { timeout: 6000, interval: 250, label: "取消后影子节点可读" },
    );
    report.check(true, "F2 点「取消」后影子节点「账号 A」保留");
    await deselect();
    await waitEdgeExists(shadowToCSpec, "取消后保留的影子->节点 C");
    report.check(true, "F2 点「取消」后影子与「节点 C」的边保留");
  }
  await snap("f2-shadow-cancel-kept");

  report.section("S19 删除影子确认与产生边消失（步骤 21）");
  {
    const { buttons } = await hoverCardActions("账号 A");
    const deleteButton = buttons.find((b) => b.attrs.title === "删除影子节点（等同于删除产生它的边）");
    if (!deleteButton) throw new Error("shadow delete button not found before confirm");
    await ui.clickNode(deleteButton);
    await ui.waitForDialog("删除影子节点将断开连接", { timeout: 8000 });
    await sleep(500);
    tree = await api.uiTree();
    const dialog = ui.findDialog(tree, "删除影子节点将断开连接");
    report.check(
      ui.findButton(tree, "删除影子节点（等同于删除产生它的边）", { root: dialog, exact: true }) !== null,
      "步骤 21 确认按钮文案为「删除影子节点（等同于删除产生它的边）」",
    );
    await clickDialogButton("删除影子节点将断开连接", "删除影子节点（等同于删除产生它的边）", { exact: true });
    await ui.waitForDialogGone("删除影子节点将断开连接", { timeout: 8000 });
    await waitForCondition(
      async () => {
        const t = await api.uiTree();
        return !ui.hasText(t, "账号 A") && ui.hasText(t, "节点 C") ? t : null;
      },
      { timeout: 8000, interval: 250, label: "影子消失且节点 C 可读" },
    );
    report.check(true, "步骤 21 确认后影子节点「账号 A」消失，「节点 C」保留");
    await sleep(600);
  }
  await snap("step21-shadow-deleted");
  {
    await backToRoot();
    await deselect();
    tree = await api.uiTree();
    report.check(
      ui.hasText(tree, "账号 A") && ui.hasText(tree, "新画布") && ui.hasText(tree, "备注 B"),
      "步骤 21 回根画布后「账号 A」「新画布」「备注 B」均存在",
    );
    await waitEdgeGone(toCanvasSpec, "影子删除后的 A->新画布");
    report.check(true, "步骤 21 删除影子等价于删除产生它的边：根画布内 A->新画布 边已消失");
  }
  await snap("step21-root-edge-gone");
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
