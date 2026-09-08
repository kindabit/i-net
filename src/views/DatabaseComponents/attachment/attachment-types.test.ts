import { describe, expect, it } from "vitest";

import { resolveTextAttachmentFileName } from "./attachment-types";

describe("resolveTextAttachmentFileName", () => {
  it("无扩展名时自动补全 .txt", () => {
    // 意图：核心便利路径——用户只输入主名时免去手工输入扩展名
    expect(resolveTextAttachmentFileName("note")).toBe("note.txt");
  });

  it("文件名以孤立点号结尾时补全为 txt 而不产生双点", () => {
    // 意图：结尾点号按无扩展名处理，补全必须得到 note.txt 而非 note..txt
    expect(resolveTextAttachmentFileName("note.")).toBe("note.txt");
  });

  it("文本类型扩展名保持用户输入原样", () => {
    // 意图：成功路径——校验大小写不敏感，但不改写输入的原始大小写
    expect(resolveTextAttachmentFileName("note.txt")).toBe("note.txt");
    expect(resolveTextAttachmentFileName("笔记.MD")).toBe("笔记.MD");
  });

  it("非文本类型的已知扩展名返回 null", () => {
    // 意图：失败路径——png 等二进制扩展名创建的附件无法被文本查看器预览，必须拦截
    expect(resolveTextAttachmentFileName("note.png")).toBeNull();
  });

  it("未收录的扩展名返回 null", () => {
    // 意图：失败路径——无法识别的扩展名同样无法预览，必须拦截
    expect(resolveTextAttachmentFileName("note.xyz123")).toBeNull();
  });
});
