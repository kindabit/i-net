// 共享最小 fixture（base）的构造脚本。
//
// 产物：e2e\script\_fixtures\base\data —— 含数据库「zz-e2e-base」（密码 e2e-password），
// 内容包括：根画布、两个数据节点（账号 A、备注 B）、两者之间的一条边。
// 流程：创建数据库 → 进入根画布 → 拖拽模板面板创建两个节点并改名 → 连线
// → 重新加载画布核对边持久化 → 保存退出 → 复制 data 目录到 fixture。
//
// 运行方式：在项目根目录执行 `node e2e\script\_fixtures\build.js`

import fs from "node:fs";
import path from "node:path";
import * as api from "../lib/api.js";
import { withApp, prepareCaseOutput } from "../lib/app.js";
import { FIXTURES_DIR, PROJECT_ROOT } from "../lib/paths.js";
import * as ui from "../lib/ui.js";
import { sleep, waitForCondition } from "../lib/util.js";

const BUILD_DIR = path.join(PROJECT_ROOT, ".temp", "e2e-fixture-build");
const DATA_DIR = path.join(BUILD_DIR, "data");
const SHOTS = path.join(BUILD_DIR, "shots");
const FIXTURE_DATA = path.join(FIXTURES_DIR, "base", "data");

const DB_NAME = "zz-e2e-base";
const DB_PASSWORD = "e2e-password";

let shotIndex = 0;

/**
 * 截取当前界面并保存到构造用截图目录。
 * @param {string} label 截图标签。
 * @returns {Promise<string>} 截图路径。
 */
async function snap(label) {
  shotIndex += 1;
  const file = path.join(SHOTS, `${String(shotIndex).padStart(2, "0")}-${label}.png`);
  await api.saveScreenshot(file);
  console.log(`(screenshot) ${file}`);
  return file;
}

/**
 * 等待具有指定属性值的节点出现。
 * @param {string} attr 属性名。
 * @param {string} value 属性值。
 * @param {number} [timeout=15000] 超时毫秒数。
 * @returns {Promise<object>} 命中节点。
 */
async function waitAttr(attr, value, timeout = 15000) {
  return waitForCondition(
    async () => ui.findAttr(await api.uiTree(), attr, value),
    { timeout, label: `attr ${attr}=${value}` },
  );
}

/**
 * 展开模板面板（已展开时直接返回）。
 * @returns {Promise<void>} 无返回值。
 */
async function openTemplatePanel() {
  if (ui.findAttr(await api.uiTree(), "title", "拖拽创建数据节点")) {
    return;
  }
  const button = await waitAttr("title", "新建节点");
  await ui.clickNode(button);
  await sleep(900);
  if (!ui.findAttr(await api.uiTree(), "title", "拖拽创建数据节点")) {
    throw new Error("template panel drag handle not found after opening");
  }
}

/**
 * 从模板面板拖拽「空节点」到画布指定坐标创建一个数据节点。
 * @param {number} x 落点 x 坐标（整窗物理像素）。
 * @param {number} y 落点 y 坐标（整窗物理像素）。
 * @returns {Promise<void>} 无返回值。
 */
async function dragBlankNodeTo(x, y) {
  await openTemplatePanel();
  const handle = ui.findAttr(await api.uiTree(), "title", "拖拽创建数据节点");
  if (!handle) {
    throw new Error("drag handle for data node not found");
  }
  const center = ui.boundsCenter(handle);
  await api.postInput([api.mouseDrag({ x: center.x, y: center.y }, { x, y })]);
  await ui.waitForText("新节点", { timeout: 8000 });
  await sleep(500);
}

/**
 * 双击「新节点」打开编辑对话框并修改标题。
 * @param {string} newTitle 新标题。
 * @returns {Promise<void>} 无返回值。
 */
async function renameNewNode(newTitle) {
  await ui.clickText("新节点", { count: 2 });
  await ui.waitForDialog("编辑节点", { timeout: 8000 });
  await ui.typeIntoEditable("标题", newTitle);
  await ui.clickButton("确认");
  await ui.waitForDialogGone("编辑节点", { timeout: 8000 });
  await ui.waitForText(newTitle, { timeout: 8000 });
  await sleep(300);
}

/**
 * 收集 UI 树中指定方向的连接桩元素。
 * @param {object[]} tree UI 树。
 * @param {string} pos 连接桩方向（top/right/bottom/left）。
 * @returns {object[]} 连接桩节点列表。
 */
function handlesByPos(tree, pos) {
  return ui
    .flatten(tree)
    .map(({ node }) => node)
    .filter((node) => node.attrs?.["data-handlepos"] === pos && node.bounds && node.bounds.width > 0);
}

/**
 * 在「账号 A」与「备注 B」之间建立一条边。
 * @returns {Promise<void>} 无返回值。
 */
async function connectNodes() {
  const tree = await api.uiTree();
  const aTitle = ui.findByText(tree, "账号 A");
  const bTitle = ui.findByText(tree, "备注 B");
  if (!aTitle || !bTitle) {
    throw new Error(`node title not found: a=${!!aTitle} b=${!!bTitle}`);
  }
  const ac = ui.boundsCenter(aTitle);
  const bc = ui.boundsCenter(bTitle);
  await api.postInput([api.mouseMove(ac.x, ac.y)]);
  await sleep(500);
  const hoverTree = await api.uiTree();
  const rights = handlesByPos(hoverTree, "right");
  const lefts = handlesByPos(hoverTree, "left");
  if (rights.length === 0 || lefts.length === 0) {
    throw new Error(`handle elements not found in UI tree: right=${rights.length} left=${lefts.length}`);
  }
  const aRight = rights.reduce((best, h) =>
    Math.abs(h.bounds.top + h.bounds.height / 2 - ac.y) <
    Math.abs(best.bounds.top + best.bounds.height / 2 - ac.y)
      ? h
      : best,
  );
  const bLeft = lefts.reduce((best, h) =>
    Math.abs(h.bounds.top + h.bounds.height / 2 - bc.y) <
    Math.abs(best.bounds.top + best.bounds.height / 2 - bc.y)
      ? h
      : best,
  );
  const from = ui.boundsCenter(aRight);
  const to = ui.boundsCenter(bLeft);
  await api.postInput([api.mouseDrag(from, to)]);
  await sleep(1200);
  await snap("edge-created");
}

/**
 * 切到画布宇宙再回到根画布，核对数据持久化。
 * @returns {Promise<void>} 无返回值。
 */
async function reloadCanvas() {
  await ui.clickText("画布宇宙");
  await ui.waitForText("根画布", { timeout: 10000 });
  await sleep(400);
  await ui.clickText("根画布", { count: 2 });
  await waitAttr("title", "新建节点", 15000);
  await sleep(800);
  await snap("after-reload");
}

prepareCaseOutput(BUILD_DIR);

await withApp({ dataDir: DATA_DIR, logFile: path.join(BUILD_DIR, "app.log") }, async () => {
  console.log("== 创建数据库 ==");
  await ui.waitForEditable("数据库名称", { timeout: 20000 });
  await ui.typeIntoEditable("数据库名称", DB_NAME);
  await ui.clickButton("确认");
  await ui.waitForEditable("创建密码", { timeout: 6000 });
  await ui.typeIntoEditable("创建密码", DB_PASSWORD);
  await ui.typeIntoEditable("确认密码", DB_PASSWORD);
  await ui.clickButton("确认");
  await ui.waitForText("根画布", { timeout: 30000 });
  await snap("universe");

  console.log("== 进入根画布 ==");
  await ui.clickText("根画布", { count: 2 });
  await waitAttr("title", "新建节点", 15000);
  await sleep(600);
  await snap("canvas-empty");

  console.log("== 创建节点 1 ==");
  await dragBlankNodeTo(700, 380);
  await renameNewNode("账号 A");
  await snap("node-a");

  console.log("== 创建节点 2 ==");
  await dragBlankNodeTo(1080, 640);
  await renameNewNode("备注 B");
  await snap("node-b");

  console.log("== 建立边 ==");
  await connectNodes();

  console.log("== 重新加载画布核对持久化 ==");
  await reloadCanvas();

  console.log("== 保存 ==");
  await api.postInput([api.keyClick(["Control", "s"])]);
  await sleep(1000);
});

console.log("== 复制 fixture 数据目录 ==");
fs.rmSync(path.dirname(FIXTURE_DATA), { recursive: true, force: true });
fs.mkdirSync(path.dirname(FIXTURE_DATA), { recursive: true });
fs.cpSync(DATA_DIR, FIXTURE_DATA, { recursive: true });
console.log(`fixture written: ${FIXTURE_DATA}`);
