// e2e 脚本的关键路径常量。

import path from "node:path";
import { fileURLToPath } from "node:url";

/** 本文件所在目录（e2e\script\lib） */
const LIB_DIR = path.dirname(fileURLToPath(import.meta.url));
/** e2e 脚本根目录（e2e\script） */
export const SCRIPT_DIR = path.resolve(LIB_DIR, "..");
/** 共享 fixture 目录（e2e\script\_fixtures） */
export const FIXTURES_DIR = path.join(SCRIPT_DIR, "_fixtures");
/** 项目根目录（i-net） */
export const PROJECT_ROOT = path.resolve(SCRIPT_DIR, "..", "..");
/** 调试构建的应用程序可执行文件（由 `pnpm tauri:build:debug` 产出，内嵌前端产物） */
export const APP_EXE_PATH = path.join(PROJECT_ROOT, "src-tauri", "target", "debug", "i_net.exe");
