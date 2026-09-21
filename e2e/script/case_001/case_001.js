// case_001 启动与新建数据库（样例脚本，同时作为测试脚本格式规范）。
//
// 流程：清空 output → 启动应用 → 名称步/密码步的校验失败路径 → 创建数据库
// 成功路径（进入画布宇宙）→ 磁盘产物检查 → 关闭应用 → 输出 PASS/FAIL 报告。
//
// 脚本规范要点：
// - 断言优先使用 UI 树（文本、bounds、states），截图用于视觉留档与像素比对；
// - 错误提示断言使用 waitForTextStable（等过渡静止后复查），并追加旧提示
//   的 waitForGone 断言，规避 Vuetify 消息过渡期间新旧文本瞬时并存的问题；
// - 应用生命周期由 withApp 包装，保证无论成败都经 POST /shutdown 收尾；
// - 关键步骤截图存 output；流程中断时额外截取 fatal 截图并写入 report.json。
//
// 运行方式：在项目根目录执行 `node e2e\script\case_001\case_001.js`

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import * as api from "../lib/api.js";
import { withApp, prepareCaseOutput } from "../lib/app.js";
import { Report, finalizeReport } from "../lib/report.js";
import * as ui from "../lib/ui.js";

const CASE_DIR = path.dirname(fileURLToPath(import.meta.url));
const OUTPUT_DIR = path.join(CASE_DIR, "output");
const DATA_DIR = path.join(OUTPUT_DIR, "data");

/** 本用例使用的数据库名称与密码 */
const DB_NAME = "zz-e2e-001";
const DB_PASSWORD = "abc123";

const report = new Report("case_001 启动与新建数据库");

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
 * 用例主流程。
 * @returns {Promise<void>} 无返回值。
 */
async function main() {
  report.section("S0 基线");
  const info = await api.health();
  report.check(info.width > 0 && info.height > 0, "调试自动化服务可用", `${info.width}x${info.height}`);
  await ui.waitForEditable("数据库名称", { timeout: 20000 });
  report.check(ui.hasText(await api.uiTree(), "数据库名称"), "首页显示「数据库名称」输入框");
  await snap("home");

  report.section("S1 名称步校验失败路径");
  await ui.clickButton("确认");
  const nameEmpty = await ui.waitForTextStable("数据库名称不能为空", { timeout: 6000 }).catch(() => null);
  report.check(nameEmpty !== null, "F1 空名称提交提示「数据库名称不能为空」");
  await snap("name-empty");

  report.section("S2 进入密码步");
  await ui.typeIntoEditable("数据库名称", DB_NAME);
  await ui.clickButton("确认");
  await ui.waitForEditable("创建密码", { timeout: 6000 });
  const passwordStep = await api.uiTree();
  report.check(ui.findEditable(passwordStep, "创建密码") !== null, "显示「创建密码」输入框");
  report.check(ui.findEditable(passwordStep, "确认密码") !== null, "显示「确认密码」输入框");
  report.check(ui.findEditable(passwordStep, "数据库名称") === null, "「数据库名称」输入框已隐藏");
  await snap("password-step");

  report.section("S3 密码步校验失败路径");
  await ui.clickButton("确认");
  const passwordEmpty = await ui.waitForTextStable("密码不能为空", { timeout: 6000 }).catch(() => null);
  report.check(passwordEmpty !== null, "F2 空密码提交提示「密码不能为空」");

  await ui.typeIntoEditable("创建密码", "123");
  await ui.clickButton("确认");
  const tooShort = await ui.waitForTextStable("密码长度不能少于 6 位", { timeout: 6000 }).catch(() => null);
  report.check(tooShort !== null, "F3 短密码提交提示「密码长度不能少于 6 位」");
  const emptyGone = await ui.waitForGone("密码不能为空", { timeout: 3000 }).catch(() => false);
  report.check(emptyGone, "F3 空密码提示已消失");

  await ui.typeIntoEditable("创建密码", DB_PASSWORD);
  await ui.typeIntoEditable("确认密码", "abc124");
  await ui.clickButton("确认");
  const mismatch = await ui.waitForTextStable("两次输入的密码不一致", { timeout: 6000 }).catch(() => null);
  report.check(mismatch !== null, "F4 两次密码不一致提示「两次输入的密码不一致」");
  const shortGone = await ui.waitForGone("密码长度不能少于 6 位", { timeout: 3000 }).catch(() => false);
  report.check(shortGone, "F4 短密码提示已消失");
  await snap("validation-failed");

  report.section("S4 返回名称步");
  await ui.clickButton("返回");
  await ui.waitForEditable("数据库名称", { timeout: 6000 });
  const backToName = await api.uiTree();
  report.check(ui.findEditable(backToName, "数据库名称") !== null, "重新显示「数据库名称」输入框");
  report.check(ui.findEditable(backToName, "创建密码") === null, "隐藏「创建密码」输入框");

  report.section("S5 创建数据库成功路径");
  await ui.clickButton("确认");
  await ui.waitForEditable("创建密码", { timeout: 6000 });
  await ui.typeIntoEditable("创建密码", DB_PASSWORD);
  await ui.typeIntoEditable("确认密码", DB_PASSWORD);
  await ui.clickButton("确认");
  await ui.waitForText("根画布", { timeout: 30000 });
  report.check(true, "创建成功并进入画布宇宙（出现「根画布」）");
  await snap("canvas-universe");

  report.section("S6 磁盘产物");
  const metadataFile = path.join(DATA_DIR, "metadata.sqlite");
  report.check(fs.existsSync(metadataFile), "metadata.sqlite 已创建", metadataFile);
  const setDir = path.join(DATA_DIR, "user_database_set");
  const subdirs = fs.existsSync(setDir) ? fs.readdirSync(setDir) : [];
  report.check(subdirs.length === 1, "user_database_set 下存在一个数据库目录", `count=${subdirs.length}`);
  const dbFile = subdirs.length === 1 ? path.join(setDir, subdirs[0], "user_database.sqlite") : null;
  report.check(dbFile !== null && fs.existsSync(dbFile), "用户数据库文件已创建", dbFile ?? "");
}

let fatal = null;
try {
  prepareCaseOutput(OUTPUT_DIR);
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
