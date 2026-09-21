// case_011 的 KeePass 2.0 测试数据库（KDBX 4.0 / Argon2id）生成脚本。
//
// 产物（均位于 e2e\script\case_011\）：
// - e2e-test.kdbx：KeePass 2.0 数据库（Master Password 固定为 e2e-kdbx-pass）
// - invalid-file.kdbx：内容为普通文本的伪 .kdbx（供「无效的 KeePass 2.0 数据库文件」路径）
// - not-a-database.txt：普通文本文件（供文件选择器扩展名过滤的 F4 变体）
// e2e-test.kdbx 内容结构：
// - 根 group「KeePass 根分组」：entry「KeePass 条目」（密码 kp-secret、访问链接、备注）
// - 子分组 A：entry「子条目 A」（密码 kp-sub-secret）
// - 子分组 B：entry「子条目 B」（备注 kp-b-note）
// 生成后立即用 kdbxweb 重新解密加载自检，打印分组与条目清单。
//
// Argon2 注入方式与 src\migration\keepass2.ts 一致（@noble/hashes），
// 内存参数下调为 8 MiB 以缩短生成与导入耗时。
//
// 运行方式：在项目根目录执行 `node e2e\script\case_011\make-kdbx.mjs`

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { argon2dAsync, argon2iAsync, argon2idAsync } from "@noble/hashes/argon2.js";
import kdbxweb from "kdbxweb";

const CASE_DIR = path.dirname(fileURLToPath(import.meta.url));
const OUT_FILE = path.join(CASE_DIR, "e2e-test.kdbx");
const INVALID_FILE = path.join(CASE_DIR, "invalid-file.kdbx");
const TEXT_FILE = path.join(CASE_DIR, "not-a-database.txt");

/** 测试数据库的 Master Password */
const MASTER_PASSWORD = "e2e-kdbx-pass";

/**
 * 向 kdbxweb 注入基于 @noble/hashes 的 argon2 实现（与 src\migration\keepass2.ts 等价）。
 * @returns {void} 无返回值。
 */
function installArgon2() {
  kdbxweb.CryptoEngine.setArgon2Impl(
    async (password, salt, memory, iterations, length, parallelism, type, version) => {
      const kind = type;
      const derive = kind === 0 ? argon2dAsync : kind === 1 ? argon2iAsync : argon2idAsync;
      const hash = await derive(new Uint8Array(password), new Uint8Array(salt), {
        t: iterations,
        m: memory,
        p: parallelism,
        dkLen: length,
        version,
      });
      return hash.slice().buffer;
    },
  );
}

/**
 * 设置条目的字段（ProtectedValue 用于密码等敏感值）。
 * @param {object} entry kdbxweb 条目。
 * @param {Record<string, string>} fields 字段名到明文/保护值的映射。
 * @param {string[]} [protectedNames=[]] 需要以 ProtectedValue 存储的字段名。
 * @returns {void} 无返回值。
 */
function setEntryFields(entry, fields, protectedNames = []) {
  for (const [name, value] of Object.entries(fields)) {
    entry.fields.set(
      name,
      protectedNames.includes(name) ? kdbxweb.ProtectedValue.fromString(value) : value,
    );
  }
}

/**
 * 生成测试数据库并写入磁盘。
 * @returns {Promise<void>} 无返回值。
 */
async function build() {
  installArgon2();
  const credentials = new kdbxweb.Credentials(
    kdbxweb.ProtectedValue.fromString(MASTER_PASSWORD),
  );
  const db = kdbxweb.Kdbx.create(credentials, "KeePass 根分组");
  db.setKdf(kdbxweb.Consts.KdfId.Argon2id);
  const kdfParameters = db.header.kdfParameters;
  if (kdfParameters === undefined) {
    throw new Error("kdbxweb 未生成 Argon2id KDF 参数");
  }
  // 默认 M 为 1 GiB，太重；降为 8 MiB（覆盖同一代码路径）。
  kdfParameters.set("M", kdbxweb.VarDictionary.ValueType.UInt64, new kdbxweb.Int64(8 * 1024));

  const root = db.getDefaultGroup();
  const rootEntry = db.createEntry(root);
  setEntryFields(
    rootEntry,
    {
      Title: "KeePass 条目",
      Password: "kp-secret",
      URL: "https://keepass.example.com/",
      Notes: "kp-note",
    },
    ["Password"],
  );

  const groupA = db.createGroup(root, "子分组 A");
  const entryA = db.createEntry(groupA);
  setEntryFields(entryA, { Title: "子条目 A", Password: "kp-sub-secret" }, ["Password"]);

  const groupB = db.createGroup(root, "子分组 B");
  const entryB = db.createEntry(groupB);
  setEntryFields(entryB, { Title: "子条目 B", Notes: "kp-b-note" });

  const bytes = new Uint8Array(await db.save());
  fs.writeFileSync(OUT_FILE, bytes);
  console.log(`kdbx written: ${OUT_FILE} (${bytes.length} bytes)`);

  // 伪 .kdbx（扩展名通过后端校验、内容无法解析）与普通文本文件。
  fs.writeFileSync(INVALID_FILE, "this is not a kdbx file\n");
  fs.writeFileSync(TEXT_FILE, "not a keepass database\n");
  console.log(`invalid asset written: ${INVALID_FILE}`);
  console.log(`text asset written: ${TEXT_FILE}`);
}

/**
 * 自检：用 kdbxweb 重新解密加载并打印分组/条目清单。
 * @returns {Promise<void>} 无返回值。
 */
async function verify() {
  installArgon2();
  const credentials = new kdbxweb.Credentials(
    kdbxweb.ProtectedValue.fromString(MASTER_PASSWORD),
  );
  const data = fs.readFileSync(OUT_FILE);
  const db = await kdbxweb.Kdbx.load(
    data.buffer.slice(data.byteOffset, data.byteOffset + data.byteLength),
    credentials,
  );
  /** 递归打印分组及其条目。 */
  const walk = (group, depth) => {
    console.log(`${"  ".repeat(depth)}[group] ${group.name}`);
    for (const entry of group.entries) {
      const title = entry.fields.get("Title") ?? "";
      const password = entry.fields.get("Password");
      const plain = password === undefined ? "" : typeof password === "string" ? password : password.getText();
      console.log(`${"  ".repeat(depth + 1)}[entry] ${title} password=${plain}`);
    }
    for (const subgroup of group.groups) {
      walk(subgroup, depth + 1);
    }
  };
  walk(db.getDefaultGroup(), 0);

  // 错误密码自检：必须以 InvalidKey 失败。
  try {
    await kdbxweb.Kdbx.load(
      data.buffer.slice(data.byteOffset, data.byteOffset + data.byteLength),
      new kdbxweb.Credentials(kdbxweb.ProtectedValue.fromString("wrong")),
    );
    throw new Error("wrong password unexpectedly succeeded");
  } catch (error) {
    if (error instanceof kdbxweb.KdbxError && error.code === kdbxweb.Consts.ErrorCodes.InvalidKey) {
      console.log("wrong-password check: InvalidKey (expected)");
    } else {
      throw error;
    }
  }
}

await build();
await verify();
