import { describe, expect, it } from "vitest";

import { joinPath, matchesExtensions, validateFileName } from "./file-picker";

describe("matchesExtensions", () => {
  it("空扩展名列表：全部匹配（含无扩展名文件）", () => {
    // 测试意图：扩展名过滤未启用时不得隐藏任何文件，无扩展名文件也必须可见。
    expect(matchesExtensions("photo.png", [])).toBe(true);
    expect(matchesExtensions("README", [])).toBe(true);
  });

  it("大小写不敏感：文件名与扩展名列表两侧均忽略大小写", () => {
    // 测试意图：锁定大小写不敏感契约——真实文件名的扩展名大小写不定，列表元素也不保证严格小写。
    expect(matchesExtensions("photo.PNG", ["png"])).toBe(true);
    expect(matchesExtensions("photo.png", ["PNG"])).toBe(true);
  });

  it("双扩展名：只取最后一段扩展名", () => {
    // 测试意图：复合扩展名按最后一段判定——a.tar.gz 的扩展名是 gz，不是 tar。
    expect(matchesExtensions("a.tar.gz", ["gz"])).toBe(true);
    expect(matchesExtensions("a.tar.gz", ["tar"])).toBe(false);
  });

  it("无扩展名：列表非空时不匹配", () => {
    // 测试意图：失败路径——无扩展名文件与以点号结尾的孤立点号文件在过滤生效时必须隐藏。
    expect(matchesExtensions("README", ["txt"])).toBe(false);
    expect(matchesExtensions("README.", ["txt"])).toBe(false);
  });

  it("含点文件名：点号开头的文件名按最后一段点号后缀判定", () => {
    // 测试意图：锁定含点文件名的边界语义——.gitignore 的最后一段后缀为 gitignore。
    expect(matchesExtensions(".gitignore", ["gitignore"])).toBe(true);
    expect(matchesExtensions(".gitignore", ["txt"])).toBe(false);
  });

  it("常规命中与未命中", () => {
    // 测试意图：冒烟——多元素列表命中任一即匹配，未命中任一项则不匹配。
    expect(matchesExtensions("note.txt", ["txt", "md"])).toBe(true);
    expect(matchesExtensions("note.txt", ["md", "json"])).toBe(false);
  });
});

describe("joinPath", () => {
  it("目录以正斜杠结尾：直接拼接", () => {
    // 测试意图：POSIX 风格目录不得产生重复分隔符。
    expect(joinPath("/home/user/", "a.txt")).toBe("/home/user/a.txt");
  });

  it("目录以反斜杠结尾：直接拼接", () => {
    // 测试意图：Windows 风格目录不得在反斜杠后再补一个正斜杠。
    expect(joinPath("C:\\Users\\", "a.txt")).toBe("C:\\Users\\a.txt");
  });

  it("目录无尾分隔符：以正斜杠连接", () => {
    // 测试意图：两种风格的目录在缺少尾分隔符时均以正斜杠补齐。
    expect(joinPath("/home/user", "a.txt")).toBe("/home/user/a.txt");
    expect(joinPath("C:\\Users", "a.txt")).toBe("C:\\Users/a.txt");
  });

  it("根目录：POSIX 根与盘符根均正确拼接", () => {
    // 测试意图：边界场景——根目录本身以分隔符结尾，不得产生双分隔符。
    expect(joinPath("/", "a.txt")).toBe("/a.txt");
    expect(joinPath("C:\\", "a.txt")).toBe("C:\\a.txt");
  });
});

describe("validateFileName", () => {
  it("合法名称：普通名称、中文、多扩展名与保留名前缀相似名均通过", () => {
    // 测试意图：正常输入不得被误拦——含中文、多段扩展名、以保留名开头但整体更长的名称都是合法文件名。
    expect(validateFileName("report.txt")).toBeNull();
    expect(validateFileName("报告 2024.md")).toBeNull();
    expect(validateFileName("a.tar.gz")).toBeNull();
    expect(validateFileName("CONSOLE.txt")).toBeNull();
    expect(validateFileName("com10.txt")).toBeNull();
  });

  it("非法字符：Windows 禁用字符与控制字符均返回 invalid-character", () => {
    // 测试意图：锁定禁用字符集合——< > : " / \ | ? * 与控制字符必须被拦截，防止路径注入与写入失败。
    const names = [
      "a<b.txt",
      "a>b.txt",
      "a:b.txt",
      'a"b.txt',
      "a/b.txt",
      "a\\b.txt",
      "a|b.txt",
      "a?b.txt",
      "a*b.txt",
      "a\u0000b.txt",
    ];
    for (const name of names) {
      expect(validateFileName(name)).toBe("invalid-character");
    }
  });

  it("点号：点号结尾与单独的 . / .. 返回 invalid-character", () => {
    // 测试意图：点号结尾会被系统静默截断（导致覆盖确认与实际写入目标不一致），. 与 .. 会指向目录本身或父目录。
    expect(validateFileName("a.")).toBe("invalid-character");
    expect(validateFileName("a.txt.")).toBe("invalid-character");
    expect(validateFileName(".")).toBe("invalid-character");
    expect(validateFileName("..")).toBe("invalid-character");
  });

  it("尾随空格：返回 invalid-character", () => {
    // 测试意图：尾随空格同样会被系统静默剥离；调用方虽先 trim，函数自身仍保持防御。
    expect(validateFileName("a.txt ")).toBe("invalid-character");
  });

  it("保留设备名：大小写与扩展名变体均返回 reserved-name", () => {
    // 测试意图：保留设备名写入会静默丢弃数据（如 NUL）或产生异常行为，必须拦截。
    const names = [
      "CON",
      "con",
      "Con.txt",
      "PRN",
      "AUX.log",
      "NUL.bak",
      "COM0",
      "com1",
      "lpt9.txt",
    ];
    for (const name of names) {
      expect(validateFileName(name)).toBe("reserved-name");
    }
  });
});
