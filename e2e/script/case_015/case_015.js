// case_015 保存与退出链路。
//
// 流程（同一数据目录、三段应用会话，会话之间不清理数据目录）：
//   会话 1：解锁进入根画布（lastScene 恢复）→ 模板面板拖拽创建画布数据节点「新画布」
//     → 双击进入子画布（面包屑三级「画布宇宙 / 根画布 / 新画布」）→ Ctrl+S（snackbar「数据库已保存」）
//     → 右下角「保存并退出」回首页（应用不退出）→ 重新解锁：直接进入子画布（lastScene 生效）
//     → 拖拽创建数据节点并改名「关闭测试」→ Ctrl+S 保存基线（修订 1，见计划末尾的修订记录）
//     → Alt+F4 触发 close-confirm（标题/正文/三按钮断言 + Esc persistent 附加断言）→「取消」后应用存活
//     → 改名「丢弃修改」（仅内存）→ Alt+F4 →「不保存，直接关闭」→ 应用退出（/health 不可达）
//   会话 2：重启解锁 → 直接进入子画布（lastScene 跨会话恢复）；「关闭测试」保留、「丢弃修改」丢失（F2）
//     → 改名「待保存」→ Alt+F4 →「保存并关闭」→ 应用退出
//   会话 3：重启解锁 → 直接进入子画布；「待保存」保留（F3）→ POST /shutdown 收尾
//
// 脚本规范要点：
// - 用户数据库为内存库：所有修改只有 lifecycle_save 后才写回文件；「不保存，直接关闭」丢弃全部未保存修改
//   （含拖拽创建的节点），因此步骤 5b 按修订计划补一次 Ctrl+S 建立已保存基线；
// - Alt+F4 可触发 onCloseRequested 拦截（实测），放入单个 /input 请求内下发；
// - close-confirm 为 persistent 对话框：Esc 不关闭，只能点三个按钮之一；
// - 「保存并退出」= 保存 → 关闭数据库 → 回首页（应用不退出）；「保存并关闭」才销毁窗口；
// - 「退出前截图」实现为点击关闭按钮之前截图（窗口销毁后无法取帧）；
// - 多段会话用 launch/stop 手动管理；每段会话结束等待 /health 不可达后再启动下一段；
// - snackbar 断言使用 waitForTextStable，触发保存前先等待旧消息消失；
// - 面包屑通过顶部条内 listitem（top < 140）按 left 升序解析，当前画布为最后一项；
// - 关键步骤截图存 output；流程中断时额外截取 fatal 截图并写入 report.json。
//
// 运行方式：在项目根目录执行 `node e2e\script\case_015\case_015.js`

import path from "node:path";
import { fileURLToPath } from "node:url";
import * as api from "../lib/api.js";
import { launch, stop, prepareCaseOutput } from "../lib/app.js";
import { prepareDataDir } from "../lib/fixtures.js";
import { Report, finalizeReport } from "../lib/report.js";
import * as ui from "../lib/ui.js";
import { sleep, waitForCondition } from "../lib/util.js";

const CASE_DIR = path.dirname(fileURLToPath(import.meta.url));
const OUTPUT_DIR = path.join(CASE_DIR, "output");
const DATA_DIR = path.join(OUTPUT_DIR, "data");
/** 三段会话共用的应用日志（追加写入） */
const LOG_FILE = path.join(OUTPUT_DIR, "app.log");

/** fixture 数据库名称与密码（见 _fixtures\README.md） */
const DB_NAME = "zz-e2e-base";
const DB_PASSWORD = "e2e-password";

/** 拖拽创建的画布数据节点默认标题 */
const CANVAS_NODE_TITLE = "新画布";
/** 已保存基线节点标题（会话 1 保存，会话 2 改名前应仍存在） */
const SAVED_NODE_TITLE = "关闭测试";
/** 会话 1 未保存的改名（不保存关闭后应丢失） */
const DISCARDED_NODE_TITLE = "丢弃修改";
/** 会话 2 保存并关闭的改名（会话 3 应保留） */
const PENDING_NODE_TITLE = "待保存";

/** 模板面板内文本的 left 上限（面板位于窗口左上角，宽约 260px） */
const PANEL_MAX_LEFT = 400;

const report = new Report("case_015 保存与退出链路");

let shotIndex = 0;
/** 当前会话的应用句柄（null 表示当前无会话） */
let app = null;

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
 * 返回 UI 树中文本精确匹配且有尺寸的 #text 节点数组。
 * @param {object[]} tree UI 树顶层节点数组。
 * @param {string} text 目标文本。
 * @returns {object[]} 命中的文本节点数组。
 */
function textNodes(tree, text) {
  return flattenNodes(tree)
    .filter((node) => node.tag === "#text" && (node.text ?? "").trim() === text && node.bounds.width > 0)
    .sort((a, b) => a.bounds.top - b.bounds.top || a.bounds.left - b.bounds.left);
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
 * 返回面包屑各级文案数组（根画布在前、当前画布在尾）。
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
 * 等待应用进程退出（/health 请求失败或超时视为不可达）。
 * @param {number} [timeoutMs=15000] 超时毫秒数。
 * @returns {Promise<boolean>} 超时前不可达时为 true。
 */
async function waitForAppGone(timeoutMs = 15000) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    let up = false;
    try {
      await api.health();
      up = true;
    } catch {
      up = false;
    }
    if (!up) {
      return true;
    }
    await sleep(400);
  }
  return false;
}

/**
 * 启动一段应用会话并记录断言。
 * @param {string} label 会话标签（用于报告与日志）。
 * @returns {Promise<void>} 无返回值。
 */
async function launchSession(label) {
  app = await launch({ dataDir: DATA_DIR, logFile: LOG_FILE });
  report.check(true, `启动应用（${label}）`, `data-dir=${DATA_DIR}`);
  await sleep(400);
}

/**
 * 等待当前会话应用退出，未退出时抛出异常。
 * @param {string} context 触发退出的操作描述。
 * @param {number} [timeoutMs=10000] 等待超时毫秒数。
 * @returns {Promise<void>} 无返回值。
 */
async function expectAppExit(context, timeoutMs = 10000) {
  const gone = await waitForAppGone(timeoutMs);
  report.check(gone, `${context} 后应用进程退出（/health 10 秒内不可达）`);
  if (!gone) {
    throw new Error(`app did not exit after ${context}`);
  }
}

/**
 * 关闭当前会话应用（兜底；应用已退出时幂等）。
 * @returns {Promise<void>} 无返回值。
 */
async function closeSession() {
  if (!app) {
    return;
  }
  await stop(app).catch(() => {});
  app = null;
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
      return (ui.findEditable(tree, "数据库名称") ?? ui.findEditable(tree, "Database Name")) ? tree : null;
    },
    { timeout: 30000, label: "首页数据库名称输入框" },
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
  await ui.clickText("简体中文", { exact: true });
  await ui.waitForEditable("数据库名称", { timeout: 15000 });
  return true;
}

/**
 * 解锁 fixture 数据库：名称步选择候选并确认，密码步输入密码确认，等待数据库视图就绪。
 * @returns {Promise<void>} 无返回值。
 */
async function unlock() {
  await ensureChineseUi();
  await ui.clickEditable("数据库名称");
  await api.postInput([api.typeText(DB_NAME)]);
  await ui.waitForTextStable(DB_NAME, { timeout: 8000 });
  await api.postInput([api.keyClick(["Enter"])]);
  await ui.clickButton("确认");
  await ui.waitForEditable("密码", { timeout: 30000 });
  await ui.typeIntoEditable("密码", DB_PASSWORD);
  await ui.clickButton("确认");
  await waitForCondition(
    async () => {
      const tree = await api.uiTree();
      return ui.findButton(tree, "保存并退出") !== null ? tree : null;
    },
    { timeout: 40000, interval: 300, label: "数据库视图就绪" },
  );
  await sleep(1200);
}

/**
 * 点击右下角「保存并退出」保存数据库并回到首页（应用不退出）。
 * @returns {Promise<void>} 无返回值。
 */
async function saveAndExit() {
  const saveExit = await waitForCondition(
    async () => {
      const tree = await api.uiTree();
      return (
        flattenNodes(tree)
          .filter((n) => n.role === "button" && ui.subtreeText(n).includes("保存并退出"))
          .sort((a, b) => b.bounds.top - a.bounds.top)[0] ?? null
      );
    },
    { timeout: 10000, interval: 200, label: "「保存并退出」按钮" },
  );
  await ui.clickNode(saveExit);
  await ui.waitForEditable("数据库名称", { timeout: 30000 });
  await sleep(1200);
}

/**
 * 在画布区域中选取一个不靠近任何文本节点的空白落点。
 * @returns {Promise<{x: number, y: number}>} 空白点坐标（窗口物理像素）。
 */
async function pickBlankPoint() {
  const info = await api.health();
  const tree = await api.uiTree();
  const texts = flattenNodes(tree).filter((n) => n.tag === "#text" && n.bounds.width > 0);
  const pad = 100;
  for (let y = 320; y < info.height - 220; y += 90) {
    for (let x = 520; x < info.width - 260; x += 90) {
      const hit = texts.some(
        (n) =>
          x > n.bounds.left - pad &&
          x < n.bounds.left + n.bounds.width + pad &&
          y > n.bounds.top - pad &&
          y < n.bounds.top + n.bounds.height + pad,
      );
      if (!hit) {
        return { x, y };
      }
    }
  }
  throw new Error("pickBlankPoint: no blank point found");
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
 * 双击 UI 树中指定文本节点。
 * @param {string} text 目标文本。
 * @returns {Promise<void>} 无返回值。
 */
async function doubleClickText(text) {
  const node = await waitForCondition(async () => textNodes(await api.uiTree(), text)[0] ?? null, {
    timeout: 8000,
    interval: 200,
    label: `文本「${text}」`,
  });
  await ui.clickNode(node, { count: 2 });
}

/**
 * 点击编辑节点对话框内按钮（在对话框子树内查找以避免歧义）。
 * @param {string} buttonText 按钮文本。
 * @returns {Promise<void>} 无返回值。
 */
async function clickEditDialogButton(buttonText) {
  const button = await waitForCondition(
    async () => {
      const tree = await api.uiTree();
      const dialog = ui.findDialog(tree, "编辑节点");
      return dialog ? ui.findButton(tree, buttonText, { root: dialog, exact: true }) : null;
    },
    { timeout: 8000, interval: 200, label: `编辑节点按钮「${buttonText}」` },
  );
  await ui.clickNode(button);
  await sleep(400);
}

/**
 * 双击节点标题打开编辑节点对话框，把标题改为新标题并确认，等待画布标题更新。
 * @param {string} oldTitle 当前节点标题。
 * @param {string} newTitle 新节点标题。
 * @returns {Promise<void>} 无返回值。
 */
async function renameNode(oldTitle, newTitle) {
  await doubleClickText(oldTitle);
  await ui.waitForDialog("编辑节点", { timeout: 10000 });
  await sleep(700);
  await ui.typeIntoEditable("标题", newTitle);
  await sleep(300);
  await clickEditDialogButton("确认");
  await ui.waitForDialogGone("编辑节点", { timeout: 10000 });
  await ui.waitForTextStable(newTitle, { timeout: 10000, settleMs: 500 });
}

/**
 * 按 Ctrl+S 触发数据库保存（先等待旧「数据库已保存」提示消失，避免新旧消息混淆）。
 * @returns {Promise<void>} 无返回值。
 */
async function pressCtrlS() {
  await ui.waitForGone("数据库已保存", { timeout: 1500 }).catch(() => null);
  await api.postInput([api.keyClick(["Control", "s"])]);
}

/**
 * 发送 Alt+F4 触发窗口关闭请求（与点击原生标题栏关闭按钮同一 onCloseRequested 拦截），
 * 并等待「关闭数据库」确认对话框出现。
 * @returns {Promise<void>} 无返回值。
 */
async function openCloseConfirm() {
  await api.postInput([api.keyClick(["Alt", "F4"])]);
  await ui.waitForDialog("关闭数据库", { timeout: 8000 });
  await sleep(900);
}

/**
 * 点击「关闭数据库」确认对话框内的指定按钮。
 * @param {string} text 按钮文本（精确匹配）。
 * @returns {Promise<void>} 无返回值。
 */
async function clickCloseConfirmButton(text) {
  const button = await waitForCondition(
    async () => {
      const tree = await api.uiTree();
      const dialog = ui.findDialog(tree, "关闭数据库");
      return dialog ? ui.findButton(tree, text, { root: dialog, exact: true }) : null;
    },
    { timeout: 8000, interval: 250, label: `关闭确认按钮「${text}」` },
  );
  await sleep(300);
  await ui.clickNode(button);
}

/**
 * 等待当前画布为指定画布（面包屑当前项匹配）。
 * @param {string} canvasName 目标画布文案。
 * @param {number} [timeout=15000] 超时毫秒数。
 * @returns {Promise<void>} 无返回值。
 */
async function waitForCanvas(canvasName, timeout = 15000) {
  await waitForCondition(
    async () => {
      const tree = await api.uiTree();
      return atCanvas(tree, canvasName) ? true : null;
    },
    { timeout, interval: 300, label: `当前画布为「${canvasName}」` },
  );
  await sleep(900);
}

/**
 * 用例主流程。
 * @returns {Promise<void>} 无返回值。
 */
async function main() {
  report.section("S0 前置与会话 1 启动");
  await launchSession("会话 1");
  const info = await api.health();
  report.check(info.width > 0 && info.height > 0, "调试自动化服务可用", `${info.width}x${info.height}`);
  const switched = await ensureChineseUi();
  report.check(true, "前置：界面语言为中文（必要时经语言菜单切回）", switched ? "已切回中文" : "初始即中文");
  await unlock();
  {
    const tree = await api.uiTree();
    report.check(textNodes(tree, "账号 A").length === 1, "前置：解锁后按 lastScene 进入根画布并出现节点「账号 A」");
    report.check(
      crumbText(tree).join("/") === "画布宇宙/根画布",
      "前置：根画布面包屑为「画布宇宙 / 根画布」",
      crumbText(tree).join("/"),
    );
  }
  await snap("s0-unlocked-root");

  report.section("S1 步骤 1：创建画布数据节点并进入子画布");
  {
    const blank = await pickBlankPoint();
    console.log(`  (blank point for canvas node: ${JSON.stringify(blank)})`);
    await dragCreateNode("拖拽创建画布数据节点", blank);
    await waitForCondition(async () => textNodes(await api.uiTree(), CANVAS_NODE_TITLE).length >= 1, {
      timeout: 10000,
      interval: 250,
      label: `「${CANVAS_NODE_TITLE}」节点出现`,
    });
    await sleep(900);
  }
  {
    const tree = await api.uiTree();
    report.check(textNodes(tree, CANVAS_NODE_TITLE).length === 1, `步骤 1 根画布出现画布数据节点「${CANVAS_NODE_TITLE}」`);
    report.check(
      ui.findAttr(tree, "title", "拖拽创建画布数据节点") === null,
      "步骤 1 创建成功后模板面板自动关闭",
    );
    report.check(!atCanvas(tree, CANVAS_NODE_TITLE), "步骤 1 创建节点后仍停留在根画布（未自动进入子画布）");
  }
  await snap("s1-canvas-node-created");
  await doubleClickText(CANVAS_NODE_TITLE);
  await waitForCanvas(CANVAS_NODE_TITLE);
  {
    const tree = await api.uiTree();
    report.check(
      crumbText(tree).join("/") === "画布宇宙/根画布/新画布",
      "步骤 1 双击进入子画布，面包屑为「画布宇宙 / 根画布 / 新画布」",
      crumbText(tree).join("/"),
    );
    report.check(textNodes(tree, "账号 A").length === 0, "步骤 1 子画布为空（不含根画布节点「账号 A」）");
  }
  await snap("s1-sub-canvas");

  report.section("S2 步骤 2：Ctrl+S 保存");
  await pressCtrlS();
  const saved1 = await ui.waitForTextStable("数据库已保存", { timeout: 10000 }).catch(() => null);
  report.check(saved1 !== null, "步骤 2 Ctrl+S 后 snackbar「数据库已保存」");
  await snap("s2-ctrl-s-saved");

  report.section("S3 步骤 3：右下角「保存并退出」");
  await saveAndExit();
  {
    const tree = await api.uiTree();
    report.check(
      ui.findEditable(tree, "数据库名称") !== null,
      "步骤 3 「保存并退出」后关闭数据库并回到首页（出现「数据库名称」输入框）",
    );
    const up = await api.health().then(() => true).catch(() => false);
    report.check(up, "步骤 3 应用未退出（「保存并退出」≠ 退出应用）");
  }
  await snap("s3-home-after-save-exit");

  report.section("S4 步骤 4：重新解锁恢复子画布（内存 lastScene 生效）");
  await unlock();
  await waitForCanvas(CANVAS_NODE_TITLE);
  {
    const tree = await api.uiTree();
    report.check(
      crumbText(tree).join("/") === "画布宇宙/根画布/新画布",
      "步骤 4 解锁后直接进入子画布「新画布」（内存 lastScene 生效）",
      crumbText(tree).join("/"),
    );
  }
  await snap("s4-reunlock-sub-canvas");

  report.section("S5 步骤 5（修订 1）：创建数据节点、改名并保存基线");
  {
    const blank = await pickBlankPoint();
    console.log(`  (blank point for data node: ${JSON.stringify(blank)})`);
    await dragCreateNode("拖拽创建数据节点", blank);
    await waitForCondition(async () => textNodes(await api.uiTree(), "新节点").length >= 1, {
      timeout: 10000,
      interval: 250,
      label: "「新节点」出现",
    });
    await sleep(900);
    await renameNode("新节点", SAVED_NODE_TITLE);
  }
  {
    const tree = await api.uiTree();
    report.check(textNodes(tree, SAVED_NODE_TITLE).length === 1, `步骤 5a 子画布出现节点「${SAVED_NODE_TITLE}」`);
    report.check(textNodes(tree, "新节点").length === 0, "步骤 5a 改名后默认标题「新节点」消失");
  }
  await snap("s5-node-created");
  await pressCtrlS();
  const saved2 = await ui.waitForTextStable("数据库已保存", { timeout: 10000 }).catch(() => null);
  report.check(saved2 !== null, "步骤 5b（修订）保存基线：Ctrl+S 后 snackbar「数据库已保存」");
  await snap("s5-baseline-saved");

  report.section("S6 步骤 6：Alt+F4 触发关闭确认框");
  await openCloseConfirm();
  {
    const tree = await api.uiTree();
    const dialog = ui.findDialog(tree, "关闭数据库");
    report.check(dialog !== null, "步骤 6 Alt+F4 弹出对话框「关闭数据库」");
    report.check(
      dialog !== null && ui.hasText([dialog], "确定要关闭当前数据库吗？", { exact: true }),
      "步骤 6 对话框正文为「确定要关闭当前数据库吗？」",
    );
    const names = ["取消", "不保存，直接关闭", "保存并关闭"];
    report.check(
      dialog !== null && names.every((name) => ui.findButton(tree, name, { root: dialog, exact: true }) !== null),
      "步骤 6 对话框含三个按钮「取消」「不保存，直接关闭」「保存并关闭」",
      names.join(","),
    );
    await api.postInput([api.keyClick(["Escape"])]);
    await sleep(700);
    report.check(
      ui.findDialog(await api.uiTree(), "关闭数据库") !== null,
      "步骤 6 附加：Esc 不关闭确认框（persistent）",
    );
  }
  await snap("s6-close-confirm");

  report.section("S7 步骤 7（F1）：取消后应用继续运行");
  await clickCloseConfirmButton("取消");
  await ui.waitForDialogGone("关闭数据库", { timeout: 8000 });
  await sleep(700);
  {
    const up = await api.health().then(() => true).catch(() => false);
    report.check(up, "F1 取消后应用存活（/health 正常）");
    const tree = await api.uiTree();
    report.check(crumbText(tree).join("/") === "画布宇宙/根画布/新画布", "F1 取消后 UI 树仍返回子画布内容");
    report.check(
      textNodes(tree, SAVED_NODE_TITLE).length === 1,
      `F1 取消后节点「${SAVED_NODE_TITLE}」仍在（取消不改变任何状态）`,
    );
  }
  await snap("s7-after-cancel");

  report.section("S8 步骤 8：未保存的标题修改");
  await renameNode(SAVED_NODE_TITLE, DISCARDED_NODE_TITLE);
  {
    const tree = await api.uiTree();
    report.check(
      textNodes(tree, DISCARDED_NODE_TITLE).length === 1,
      `步骤 8 节点标题改为「${DISCARDED_NODE_TITLE}」（仅内存，未保存）`,
    );
    report.check(textNodes(tree, SAVED_NODE_TITLE).length === 0, `步骤 8 旧标题「${SAVED_NODE_TITLE}」不再显示`);
  }
  await snap("s8-renamed-discard");

  report.section("S9 步骤 9（F2 前半）：不保存，直接关闭");
  await openCloseConfirm();
  await snap("s9-close-confirm-before-discard");
  await clickCloseConfirmButton("不保存，直接关闭");
  await expectAppExit("步骤 9 「不保存，直接关闭」", 10000);
  await closeSession();

  report.section("S10 步骤 10（会话 2）：重启并按 lastScene 恢复");
  await launchSession("会话 2");
  await unlock();
  await waitForCanvas(CANVAS_NODE_TITLE);
  {
    const tree = await api.uiTree();
    report.check(
      crumbText(tree).join("/") === "画布宇宙/根画布/新画布",
      "步骤 10 重启解锁后直接进入子画布「新画布」（lastScene 跨会话恢复）",
      crumbText(tree).join("/"),
    );
  }
  await snap("s10-session2-unlocked");

  report.section("S11 步骤 11（F2 后半）：不保存语义数据核对");
  {
    const tree = await api.uiTree();
    report.check(
      textNodes(tree, SAVED_NODE_TITLE).length === 1,
      `步骤 11 已保存基线节点「${SAVED_NODE_TITLE}」保留`,
    );
    report.check(
      textNodes(tree, DISCARDED_NODE_TITLE).length === 0,
      `步骤 11 未保存的标题「${DISCARDED_NODE_TITLE}」在重启后丢失（F2）`,
    );
    report.check(textNodes(tree, "新节点").length === 0, "步骤 11 无残留默认标题节点");
  }
  await snap("s11-session2-data");

  report.section("S12 步骤 12：改名为「待保存」");
  await renameNode(SAVED_NODE_TITLE, PENDING_NODE_TITLE);
  {
    const tree = await api.uiTree();
    report.check(
      textNodes(tree, PENDING_NODE_TITLE).length === 1,
      `步骤 12 节点标题改为「${PENDING_NODE_TITLE}」（仅内存，未保存）`,
    );
  }
  await snap("s12-renamed-pending");

  report.section("S13 步骤 13（F3 前半）：保存并关闭");
  await openCloseConfirm();
  await snap("s13-close-confirm-before-save-close");
  await clickCloseConfirmButton("保存并关闭");
  await expectAppExit("步骤 13 「保存并关闭」", 10000);
  await closeSession();

  report.section("S14 步骤 14（会话 3，F3 后半）：保存语义数据核对");
  await launchSession("会话 3");
  await unlock();
  await waitForCanvas(CANVAS_NODE_TITLE);
  {
    const tree = await api.uiTree();
    report.check(
      crumbText(tree).join("/") === "画布宇宙/根画布/新画布",
      "步骤 14 重启解锁后直接进入子画布「新画布」（lastScene 恢复）",
      crumbText(tree).join("/"),
    );
    report.check(
      textNodes(tree, PENDING_NODE_TITLE).length === 1,
      `步骤 14 「保存并关闭」保留的标题「${PENDING_NODE_TITLE}」存在（F3）`,
    );
    report.check(textNodes(tree, SAVED_NODE_TITLE).length === 0, `步骤 14 旧标题「${SAVED_NODE_TITLE}」不存在`);
    report.check(textNodes(tree, DISCARDED_NODE_TITLE).length === 0, `步骤 14 「${DISCARDED_NODE_TITLE}」不存在`);
  }
  await snap("s14-session3-data");

  report.section("S15 步骤 15：/shutdown 收尾");
  await api.shutdown();
  const gone = await waitForAppGone(15000);
  report.check(gone, "步骤 15 POST /shutdown 后应用正常退出（/health 不可达）");
  await closeSession();
  report.check(true, "F4 按计划不构造：保存失败路径需要只读数据目录等环境破坏（记录）");
  report.check(true, "F5 按计划不构造：lastScene 指向已删除画布的兜底分支（记录）");
}

let fatal = null;
try {
  prepareCaseOutput(OUTPUT_DIR);
  prepareDataDir(DATA_DIR, "base");
  await main();
} catch (error) {
  fatal = error;
  await snap("fatal").catch(() => {});
  report.check(false, "用例执行中断", error.message);
} finally {
  await closeSession();
  finalizeReport(report, OUTPUT_DIR, fatal);
}
