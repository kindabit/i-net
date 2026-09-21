// case_002 已注册数据库解锁与密码校验。
//
// 流程：清空 output → 复制 base fixture → 启动应用 → 首页名称步的候选下拉展示与键盘
// 操作 → 名称步失败路径（空名称）→ 进入密码步 → 密码内联校验失败路径 → 错误密码
// （snackbar「解密失败」）→ 重试正确密码解锁 → 按 registry lastScene 进入根画布
// → 磁盘产物检查 → 关闭应用 → 输出 PASS/FAIL 报告。
//
// 脚本规范要点：
// - 断言优先使用 UI 树（文本、bounds、states），截图用于视觉留档（候选高亮样式只在
//   截图中可见，UI 树未暴露 aria-activedescendant / aria-selected 的变化）；
// - 错误提示断言使用 waitForTextStable（等过渡静止后复查），并追加旧提示
//   waitForGone 断言，规避 Vuetify 消息过渡期间新旧文本瞬时并存的问题；
// - 应用生命周期由 withApp 包装，保证无论成败都经 POST /shutdown 收尾；
// - 关键步骤截图存 output；流程中断时额外截取 fatal 截图并写入 report.json。
//
// 运行方式：在项目根目录执行 `node e2e\script\case_002\case_002.js`

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import * as api from "../lib/api.js";
import { withApp, prepareCaseOutput } from "../lib/app.js";
import { prepareDataDir } from "../lib/fixtures.js";
import { Report, finalizeReport } from "../lib/report.js";
import * as ui from "../lib/ui.js";
import { waitForCondition } from "../lib/util.js";

const CASE_DIR = path.dirname(fileURLToPath(import.meta.url));
const OUTPUT_DIR = path.join(CASE_DIR, "output");
const DATA_DIR = path.join(OUTPUT_DIR, "data");

/** fixture 数据库名称与密码（见 _fixtures\README.md） */
const DB_NAME = "zz-e2e-base";
const DB_PASSWORD = "e2e-password";

const report = new Report("case_002 已注册数据库解锁与密码校验");

/** 启动前记录的 fixture 用户数据库目录名列表（步骤 11 核对数据库未被重建） */
let dbDirsBeforeLaunch = [];

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
 * 等待指定可编辑控件被聚焦（首页入场与步骤面板切换动画结束的信号：
 * 两个面板的 @after-enter 都会把焦点落到当前步骤的输入框）。
 * @param {string} nameFragment 名称片段。
 * @param {object} [options] 可选参数。
 * @param {number} [options.timeout=8000] 超时毫秒数。
 * @returns {Promise<object>} 已聚焦的可编辑节点。
 */
async function waitForEditableFocused(nameFragment, { timeout = 8000 } = {}) {
  return waitForCondition(
    async () => {
      const node = ui.findEditable(await api.uiTree(), nameFragment);
      return node && node.states?.focused === true ? node : null;
    },
    { timeout, label: `focused editable "${nameFragment}"` },
  );
}

/**
 * 在 UI 树中查找候选下拉里对应目标数据库的列表项。
 * @param {object[]} tree UI 树顶层节点数组。
 * @returns {object|null} 候选列表项节点；未命中时为 null。
 */
function findCandidateItem(tree) {
  const hit = ui
    .findByRole(tree, "listitem")
    .find((node) => ui.subtreeText(node).includes(DB_NAME));
  return hit ?? null;
}

/**
 * 等待候选下拉中出现目标数据库的列表项。
 * @param {object} [options] 可选参数。
 * @param {number} [options.timeout=8000] 超时毫秒数。
 * @returns {Promise<object>} 命中的候选列表项节点。
 */
async function waitForCandidateItem({ timeout = 8000 } = {}) {
  return waitForCondition(async () => findCandidateItem(await api.uiTree()), {
    timeout,
    label: `dropdown candidate "${DB_NAME}"`,
  });
}

/**
 * 读取候选项的副标题（标题之外的第一条文本，即本地化的最后打开时间）。
 * @param {object} item 候选列表项节点。
 * @returns {string} 副标题文本；不存在时为空串。
 */
function candidateSubtitle(item) {
  const texts = (item.children ?? [])
    .filter((child) => child.tag === "#text")
    .map((child) => (child.text ?? "").trim())
    .filter((text) => text !== "" && text !== DB_NAME);
  return texts[0] ?? "";
}

/**
 * 列出用户数据目录下 user_database_set 内的数据库子目录名。
 * @param {string} dataDir 用户数据目录。
 * @returns {string[]} 数据库子目录名列表（目录不存在时为空数组）。
 */
function listUserDbDirs(dataDir) {
  const setDir = path.join(dataDir, "user_database_set");
  return fs.existsSync(setDir) ? fs.readdirSync(setDir).sort() : [];
}

/**
 * 用例主流程。
 * @returns {Promise<void>} 无返回值。
 */
async function main() {
  report.section("S0 前置与首页（步骤 1）");
  const info = await api.health();
  report.check(info.width > 0 && info.height > 0, "调试自动化服务可用", `${info.width}x${info.height}`);
  await ensureChineseUi();
  await waitForEditableFocused("数据库名称");
  const homeTree = await api.uiTree();
  report.check(ui.hasText(homeTree, "数据库名称"), "首页显示「数据库名称」输入框");
  report.check(ui.findButton(homeTree, "确认") !== null, "首页显示「确认」按钮");
  await snap("step1-home");

  report.section("S1 候选下拉与键盘操作（步骤 2-5、F4）");
  // 步骤 2：聚焦名称输入框，展开候选下拉
  await ui.clickEditable("数据库名称");
  const item = await waitForCandidateItem().catch(() => null);
  report.check(item !== null, "步骤 2 候选下拉展开并显示已注册数据库", DB_NAME);
  const subtitle = item ? candidateSubtitle(item) : "";
  report.check(
    /^\d{1,4}\/\d{1,2}\/\d{1,4}.*\d{1,2}:\d{2}/.test(subtitle),
    "候选项显示非空的最后打开时间副标题（本地化 short 日期时间）",
    subtitle,
  );
  await snap("step2-dropdown-open");

  // 步骤 3：Escape 收起下拉，输入框保持聚焦
  await api.postInput([api.keyClick(["Escape"])]);
  const collapsed = await ui.waitForGone(DB_NAME, { timeout: 3000 }).catch(() => false);
  report.check(collapsed, "步骤 3 Escape 后候选下拉收起");
  const afterEscape = await api.uiTree();
  report.check(
    ui.findEditable(afterEscape, "数据库名称")?.states?.focused === true,
    "步骤 3 收起下拉后输入框仍保持聚焦",
  );

  // F4：下拉收起后名称仍为空时直接提交
  await ui.clickButton("确认");
  const nameEmpty = await ui.waitForTextStable("数据库名称不能为空", { timeout: 6000 }).catch(() => null);
  report.check(nameEmpty !== null, "F4 空名称提交提示「数据库名称不能为空」");
  const f4Tree = await api.uiTree();
  report.check(ui.findEditable(f4Tree, "密码") === null, "F4 未进入密码步");
  await snap("f4-name-empty");

  // 步骤 4：重新聚焦输入框，ArrowDown 高亮第一项候选（高亮样式见截图）
  await ui.clickEditable("数据库名称");
  await waitForCandidateItem();
  await api.postInput([api.keyClick(["Down"])]);
  const afterArrow = await api.uiTree();
  report.check(
    findCandidateItem(afterArrow) !== null,
    "步骤 4 ArrowDown 后候选下拉保持展开且候选仍高亮在首项上（高亮样式见截图）",
  );
  await snap("step4-arrow-down");

  // 步骤 5：Enter 选中候选并收起下拉
  await api.postInput([api.keyClick(["Enter"])]);
  const selectedGone = await ui.waitForGone(DB_NAME, { timeout: 3000 }).catch(() => false);
  report.check(selectedGone, "步骤 5 Enter 后候选下拉收起（候选已选中）");

  report.section("S2 进入密码步（步骤 6）");
  await ui.clickButton("确认");
  await waitForEditableFocused("密码");
  const passwordTree = await api.uiTree();
  report.check(ui.findEditable(passwordTree, "密码") !== null, "步骤 6 显示「密码」输入框");
  report.check(ui.findEditable(passwordTree, "创建密码") === null, "步骤 6 不显示「创建密码」输入框");
  report.check(ui.findEditable(passwordTree, "确认密码") === null, "步骤 6 不显示「确认密码」输入框");
  report.check(ui.findEditable(passwordTree, "数据库名称") === null, "步骤 6「数据库名称」输入框已消失");
  await snap("step6-password-step");

  report.section("S3 密码内联校验失败路径（步骤 7-8、F1-F2）");
  // 步骤 7 / F1：空密码提交
  await ui.clickButton("确认");
  const passwordEmpty = await ui.waitForTextStable("密码不能为空", { timeout: 6000 }).catch(() => null);
  report.check(passwordEmpty !== null, "F1 空密码提交提示「密码不能为空」");
  const stillPassword = await api.uiTree();
  report.check(ui.findEditable(stillPassword, "密码") !== null, "F1 空密码提交后仍停留在密码步");
  await snap("step7-password-empty");

  // 步骤 8 / F2：短密码提交
  await ui.typeIntoEditable("密码", "12345");
  await ui.clickButton("确认");
  const tooShort = await ui.waitForTextStable("密码长度不能少于 6 位", { timeout: 6000 }).catch(() => null);
  report.check(tooShort !== null, "F2 短密码提交提示「密码长度不能少于 6 位」");
  const emptyGone = await ui.waitForGone("密码不能为空", { timeout: 3000 }).catch(() => false);
  report.check(emptyGone, "F2 空密码提示「密码不能为空」已消失");
  await snap("step8-password-too-short");

  report.section("S4 错误密码（步骤 9、F3）");
  await ui.typeIntoEditable("密码", "wrong-password");
  await ui.clickButton("确认");
  const failToDecrypt = await ui.waitForTextStable("解密失败", { timeout: 8000 }).catch(() => null);
  report.check(failToDecrypt !== null, "F3 错误密码提交后 snackbar 提示「解密失败」");
  await snap("step9-wrong-password");
  const wrongTree = await api.uiTree();
  report.check(ui.findEditable(wrongTree, "密码") !== null, "F3 错误密码后仍停留在密码步");
  report.check(
    ui.findEditable(wrongTree, "数据库名称") === null && !ui.hasText(wrongTree, "账号 A"),
    "F3 错误密码后未进入数据库页面",
  );

  report.section("S5 重试正确密码解锁（步骤 10）");
  const errorGone = await ui.waitForGone("解密失败", { timeout: 9000 }).catch(() => false);
  report.check(errorGone, "步骤 10 等待错误提示消失（snackbar 5 秒后自动关闭）");
  await ui.typeIntoEditable("密码", DB_PASSWORD);
  await ui.clickButton("确认");
  await ui.waitForTextStable("账号 A", { timeout: 30000, settleMs: 800 });
  const unlocked = await api.uiTree();
  report.check(ui.hasText(unlocked, "账号 A"), "步骤 10 解锁后出现数据节点「账号 A」");
  report.check(ui.hasText(unlocked, "备注 B"), "步骤 10 解锁后出现数据节点「备注 B」");
  report.check(ui.hasText(unlocked, "根画布"), "步骤 10 按 lastScene 恢复场景（面包屑出现「根画布」）");
  report.check(ui.hasText(unlocked, "画布宇宙"), "步骤 10 面包屑包含上层「画布宇宙」");
  await ui.waitForGone("数据库名称", { timeout: 5000 }).catch(() => false);
  await snap("step10-unlocked-root-canvas");

  report.section("S6 磁盘产物（步骤 11）");
  const dbDirsAfter = listUserDbDirs(DATA_DIR);
  report.check(
    dbDirsAfter.length === 1 && dbDirsBeforeLaunch.length === 1 && dbDirsAfter[0] === dbDirsBeforeLaunch[0],
    "用户数据库目录未被重建（目录名与启动前一致）",
    `${dbDirsBeforeLaunch.join(",")} -> ${dbDirsAfter.join(",")}`,
  );
  const dbFile = path.join(DATA_DIR, "user_database_set", dbDirsAfter[0] ?? "", "user_database.sqlite");
  report.check(dbDirsAfter.length === 1 && fs.existsSync(dbFile), "用户数据库文件仍存在", dbFile);
}

let fatal = null;
try {
  prepareCaseOutput(OUTPUT_DIR);
  prepareDataDir(DATA_DIR, "base");
  dbDirsBeforeLaunch = listUserDbDirs(DATA_DIR);
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
