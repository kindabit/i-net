<!--
  附件文本查看/编辑器。

  将附件明文按 UTF-8 解码后交由 CodeMirror 6 渲染，按文件扩展名动态加载对应语言的
  语法高亮（暗色主题下套用 oneDark）；支持直接编辑并保存，保存时明文经后端加密覆盖
  附件文件并更新元数据。编辑后未保存时工具条显示未保存标记并启用保存按钮；
  组件通过 defineExpose 暴露 hasUnsavedChanges()，供预览对话框在关闭前确认。
-->
<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { useTheme } from "vuetify";
import { t } from "@/i18n";
import { EditorState, StateEffect, type Extension } from "@codemirror/state";
import { EditorView } from "@codemirror/view";
import { basicSetup } from "codemirror";
import { StreamLanguage } from "@codemirror/language";
import { oneDark } from "@codemirror/theme-one-dark";
import { userDatabaseAttachmentUpdateFile } from "@/api";
import { snackbarErrorCode, snackbarText } from "@/composables/use-snackbar";
import { textLanguageOf } from "./attachment-types";

const props = defineProps<{
  /** 附件明文内容（UTF-8 字节） */
  bytes: Uint8Array;
  /** 附件文件名（用于推断语言） */
  fileName: string;
  /** 附件 id（保存回写时使用） */
  attachmentId: string;
}>();

/** 语言标识 → 异步语言加载器（按需分包；返回 CodeMirror 语言扩展） */
type LanguageLoader = () => Promise<Extension>;

/** 语言加载器表：非 legacy 语言用官方语言包，legacy 语言经 StreamLanguage 包装 */
const LANGUAGE_LOADERS: Record<string, LanguageLoader> = {
  markdown: async () => (await import("@codemirror/lang-markdown")).markdown(),
  json: async () => (await import("@codemirror/lang-json")).json(),
  yaml: async () => (await import("@codemirror/lang-yaml")).yaml(),
  xml: async () => (await import("@codemirror/lang-xml")).xml(),
  html: async () => (await import("@codemirror/lang-html")).html(),
  css: async () => (await import("@codemirror/lang-css")).css(),
  javascript: async () => (await import("@codemirror/lang-javascript")).javascript(),
  typescript: async () =>
    (await import("@codemirror/lang-javascript")).javascript({ typescript: true }),
  jsx: async () =>
    (await import("@codemirror/lang-javascript")).javascript({ jsx: true }),
  tsx: async () =>
    (await import("@codemirror/lang-javascript")).javascript({
      typescript: true,
      jsx: true,
    }),
  python: async () => (await import("@codemirror/lang-python")).python(),
  rust: async () => (await import("@codemirror/lang-rust")).rust(),
  java: async () => (await import("@codemirror/lang-java")).java(),
  cpp: async () => (await import("@codemirror/lang-cpp")).cpp(),
  sql: async () => (await import("@codemirror/lang-sql")).sql(),
  toml: async () =>
    StreamLanguage.define((await import("@codemirror/legacy-modes/mode/toml")).toml),
  shell: async () =>
    StreamLanguage.define((await import("@codemirror/legacy-modes/mode/shell")).shell),
  ini: async () =>
    StreamLanguage.define((await import("@codemirror/legacy-modes/mode/properties")).properties),
  go: async () =>
    StreamLanguage.define((await import("@codemirror/legacy-modes/mode/go")).go),
  ruby: async () =>
    StreamLanguage.define((await import("@codemirror/legacy-modes/mode/ruby")).ruby),
  swift: async () =>
    StreamLanguage.define((await import("@codemirror/legacy-modes/mode/swift")).swift),
  powershell: async () =>
    StreamLanguage.define((await import("@codemirror/legacy-modes/mode/powershell")).powerShell),
  diff: async () =>
    StreamLanguage.define((await import("@codemirror/legacy-modes/mode/diff")).diff),
  lua: async () =>
    StreamLanguage.define((await import("@codemirror/legacy-modes/mode/lua")).lua),
  r: async () =>
    StreamLanguage.define((await import("@codemirror/legacy-modes/mode/r")).r),
  perl: async () =>
    StreamLanguage.define((await import("@codemirror/legacy-modes/mode/perl")).perl),
  dockerfile: async () =>
    StreamLanguage.define((await import("@codemirror/legacy-modes/mode/dockerfile")).dockerFile),
  csv: async () =>
    StreamLanguage.define((await import("@codemirror/legacy-modes/mode/spreadsheet")).spreadsheet),
  c: async () =>
    StreamLanguage.define((await import("@codemirror/legacy-modes/mode/clike")).c),
  csharp: async () =>
    StreamLanguage.define((await import("@codemirror/legacy-modes/mode/clike")).csharp),
  kotlin: async () =>
    StreamLanguage.define((await import("@codemirror/legacy-modes/mode/clike")).kotlin),
  dart: async () =>
    StreamLanguage.define((await import("@codemirror/legacy-modes/mode/clike")).dart),
  scala: async () =>
    StreamLanguage.define((await import("@codemirror/legacy-modes/mode/clike")).scala),
  php: async () =>
    StreamLanguage.define(
      (await import("@codemirror/legacy-modes/mode/clike")).clike(phpClikeConfig()),
    ),
};

/** 语言标识 → 工具条展示名 */
const LANGUAGE_LABELS: Record<string, string> = {
  text: "Plain Text",
  markdown: "Markdown",
  json: "JSON",
  yaml: "YAML",
  toml: "TOML",
  xml: "XML",
  html: "HTML",
  css: "CSS",
  javascript: "JavaScript",
  typescript: "TypeScript",
  jsx: "JSX",
  tsx: "TSX",
  python: "Python",
  rust: "Rust",
  java: "Java",
  c: "C",
  cpp: "C++",
  csharp: "C#",
  go: "Go",
  ruby: "Ruby",
  php: "PHP",
  swift: "Swift",
  kotlin: "Kotlin",
  shell: "Shell",
  powershell: "PowerShell",
  sql: "SQL",
  ini: "INI",
  diff: "Diff",
  lua: "Lua",
  r: "R",
  dart: "Dart",
  scala: "Scala",
  perl: "Perl",
  dockerfile: "Dockerfile",
  csv: "CSV",
};

/**
 * 构造 PHP 的 clike 模式配置（legacy-modes 未内置 PHP，基于 clike 模式裁剪关键词）。
 * 无输入参数，返回 clike 配置对象。
 */
function phpClikeConfig() {
  /**
   * 将空格分隔的单词串转换为关键词字典（供 clike 模式的 keywords/types 等配置使用）。
   * @param text 空格分隔的关键词字符串
   * @returns 关键词字典（{word: true}）
   */
  function wordsOf(text: string): Record<string, boolean> {
    const obj: Record<string, boolean> = {};
    for (const word of text.split(" ")) {
      obj[word] = true;
    }
    return obj;
  }

  return {
    name: "php",
    keywords: wordsOf(
      "abstract and array as break callable case catch class clone const continue declare " +
        "default do echo else elseif empty enddeclare endfor endforeach endif endswitch endwhile " +
        "enum eval exit extends final finally fn for foreach function global goto if implements " +
        "include include_once instanceof insteadof interface isset list match namespace new or " +
        "print private protected public readonly require require_once return static switch throw " +
        "trait try unset use var while xor yield",
    ),
    types: wordsOf(
      "bool boolean int integer float double string array object callable mixed void null",
    ),
    blockKeywords: wordsOf(
      "catch class do else elseif for foreach if switch try while final",
    ),
    atoms: wordsOf("true false null TRUE FALSE NULL"),
  };
}

/** 当前文件的语言标识 */
const languageKey = computed(() => textLanguageOf(props.fileName));

/** 工具条展示的语言名（未收录时回退为语言标识） */
const languageLabel = computed(() => LANGUAGE_LABELS[languageKey.value] ?? languageKey.value);

/** 是否存在未保存的修改 */
const dirty = ref(false);
/** 保存进行中 */
const saving = ref(false);
/** 编辑器挂载容器 */
const editorContainer = ref<HTMLDivElement | null>(null);

/** CodeMirror 编辑器视图实例（挂载后创建，卸载前销毁） */
let view: EditorView | null = null;
/** 已生效的扩展列表（语言加载完成后追加并 reconfigure） */
let extensions: Extension[] = [];

/** 初始文本：将附件明文字节按 UTF-8 容错解码（非法字节替换为替换符） */
const initialText = new TextDecoder("utf-8").decode(props.bytes);

onMounted(() => {
  const container = editorContainer.value;
  if (!container) return;
  const dark = useTheme().global.current.value.dark;
  extensions = [
    basicSetup,
    EditorView.updateListener.of((update) => {
      if (update.docChanged) {
        dirty.value = true;
      }
    }),
    ...(dark ? [oneDark] : []),
  ];
  view = new EditorView({
    state: EditorState.create({
      doc: initialText,
      extensions,
    }),
    parent: container,
  });
  void loadLanguage();
});

onUnmounted(() => {
  view?.destroy();
  view = null;
});

/**
 * 按当前语言标识异步加载语法高亮扩展，加载完成后 reconfigure 到编辑器。
 * 语言包加载失败不影响编辑功能，仅记录错误。
 * 无输入参数，无返回值。
 */
async function loadLanguage(): Promise<void> {
  const loader = LANGUAGE_LOADERS[languageKey.value];
  if (!loader) return;
  try {
    const language = await loader();
    if (!view) return;
    extensions = [...extensions, language];
    view.dispatch({ effects: StateEffect.reconfigure.of(extensions) });
  } catch (e) {
    console.error("[attachment text] failed to load language:", e);
  }
}

/**
 * 保存当前编辑内容：将文本编码为 UTF-8 字节并调用后端更新接口覆盖附件文件。
 * 成功提示并清除未保存标记，失败提示错误。无输入参数，无返回值。
 */
async function save(): Promise<void> {
  if (!view || !dirty.value || saving.value) return;
  saving.value = true;
  try {
    const content = new TextEncoder().encode(view.state.doc.toString());
    await userDatabaseAttachmentUpdateFile(props.attachmentId, content);
    dirty.value = false;
    snackbarText(t("database.canvas.attachment.text-saved"), "success");
  } catch (e) {
    snackbarErrorCode(e);
  } finally {
    saving.value = false;
  }
}

defineExpose({
  /** 是否存在未保存的修改（供预览对话框关闭前确认） */
  hasUnsavedChanges: (): boolean => dirty.value,
});
</script>

<template>
  <div class="viewer-text">
    <div class="viewer-text-toolbar">
      <span class="viewer-text-language">{{ languageLabel }}</span>
      <span v-if="dirty" class="viewer-text-unsaved">
        {{ t("database.canvas.attachment.text-unsaved") }}
      </span>
      <VBtn
        variant="flat"
        color="primary"
        size="small"
        :disabled="!dirty"
        :loading="saving"
        @click="save"
      >
        {{ t("database.canvas.attachment.text-save") }}
      </VBtn>
    </div>
    <div ref="editorContainer" class="viewer-text-editor"></div>
  </div>
</template>

<style lang="scss" scoped>
.viewer-text {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  height: 100%;
  min-height: 0;
}

.viewer-text-toolbar {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 0.75rem;
}

.viewer-text-language {
  font-size: 0.875rem;
  opacity: 0.75;
}

.viewer-text-unsaved {
  font-size: 0.875rem;
  color: rgb(var(--v-theme-warning));
}

.viewer-text-editor {
  flex: 1;
  min-height: 0;
  overflow: hidden;
  border: 1px solid rgba(var(--v-theme-on-surface), 0.12);
  border-radius: 0.25rem;

  :deep(.cm-editor) {
    height: 100%;
  }

  :deep(.cm-scroller) {
    overflow: auto;
  }
}
</style>
