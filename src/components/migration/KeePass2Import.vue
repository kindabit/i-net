<!--
  KeePass 2.0 数据导入对话框。

  从 KeePass 2.0 数据库（.kdbx）导入数据的对话框：用户选择数据库文件并输入其
  Master Password，确认后由前端读取文件字节、解密解析并构造好树形节点数据
  （含表示数据库本身的根节点、父子边、树形布局坐标与字段名国际化），再调用
  后端聚合导入接口写库，在根画布下创建新画布节点。导入成功后 emit "success"
  事件并携带新画布 id，父组件借此跳转至新画布。
-->
<script setup lang="ts">
import { ref, watch } from "vue";
import { t } from "@/i18n";
import {
  userDatabaseCanvasList,
  userDatabaseMigrationImportKeepass2,
  userDatabaseMigrationPickFile,
  userDatabaseMigrationReadFile,
  userDatabaseNodeList,
} from "@/api";
import type { ImportedTree } from "@/migration/keepass2";
import { snackbarErrorCode, snackbarText } from "@/composables/use-snackbar";
import { triggerFatalError } from "@/composables/use-fatal-error";
import PasswordField from "@/components/PasswordField.vue";
import {
  layoutImportedTree,
  parseKeepass2Database,
  type Keepass2FieldText,
} from "@/migration/keepass2";

const emit = defineEmits<{
  /** 导入成功后通知父组件跳转至新画布，携带新画布 id */
  success: [canvasId: string];
}>();

/** 对话框显示状态 */
const dialog = ref(false);
/** KeePass 2.0 数据库文件路径 */
const filePath = ref("");
/** KeePass 2.0 数据库的 Master Password */
const masterPassword = ref("");
/** 文件路径校验错误文案 */
const fileError = ref("");
/** 密码校验错误文案 */
const passwordError = ref("");
/** 文件选择进行中（系统对话框打开期间） */
const picking = ref(false);
/** 导入进行中 */
const importing = ref(false);

/**
 * 打开对话框并重置表单状态（文件路径空、密码空、校验错误清空）。
 */
function open() {
  filePath.value = "";
  masterPassword.value = "";
  fileError.value = "";
  passwordError.value = "";
  picking.value = false;
  importing.value = false;
  dialog.value = true;
}

// 密码输入变化时清除密码校验错误，让用户修正后重试
watch(masterPassword, () => {
  passwordError.value = "";
});

/**
 * 触发"选择文件"按钮：调后端弹出系统文件选择对话框；
 * 用户在系统对话框中取消时保持原状，选中时回填路径并清除路径校验错误。
 */
async function pickFile() {
  picking.value = true;
  try {
    const picked = await userDatabaseMigrationPickFile();
    if (picked === null) return;
    filePath.value = picked;
    fileError.value = "";
  } catch (error) {
    snackbarErrorCode(error);
  } finally {
    picking.value = false;
  }
}

/**
 * 取文件路径的文件名 stem（basename 去扩展名）：
 * Windows（\）与 POSIX（/）分隔符都处理；扩展名分隔点必须位于非首字符位置，
 * 避免把 ".kdbx" 这类以点开头的整名剥成空串。
 * @param path 文件路径
 * @returns 文件名 stem，路径不含文件名（如以分隔符结尾）时返回空串
 */
function fileNameStem(path: string): string {
  const segments = path.split(/[\\/]/);
  const basename = segments[segments.length - 1];
  const dotIndex = basename.lastIndexOf(".");
  return dotIndex > 0 ? basename.slice(0, dotIndex) : basename;
}

/**
 * 触发"确定"按钮：先做前端非空校验（路径、密码），校验通过后执行新导入流程：
 * 读文件字节 → 前端解密解析 → 计算画布名与落点坐标 → 树形布局 → 后端聚合导入。
 * 成功后关闭对话框并 emit "success"（携带新画布 id），失败保持打开供用户重试。
 */
async function confirmImport() {
  fileError.value = "";
  passwordError.value = "";
  if (!filePath.value) {
    fileError.value = t("database.migration.file-required");
  }
  if (!masterPassword.value) {
    passwordError.value = t("database.migration.password-required");
  }
  if (fileError.value || passwordError.value) return;
  importing.value = true;
  try {
    // a) 读文件字节（后端 command 失败抛 ErrorCode，由末尾 catch 统一反馈）
    const bytes = await userDatabaseMigrationReadFile(filePath.value);

    // b) 前端解密解析为树形节点数据；字段名文案取当前语言，导入后即用户所见字段名
    const fieldText: Keepass2FieldText = {
      password: t("database.migration.field-password"),
      url: t("database.migration.field-url"),
      notes: t("database.migration.field-notes"),
    };
    let tree: ImportedTree;
    try {
      tree = await parseKeepass2Database(bytes, masterPassword.value, fieldText);
    } catch (error) {
      // 失败值为 Keepass2ParseError（"incorrect-password" 或 "invalid-file"），
      // 非 ErrorCode，用本地化文案直接提示。
      snackbarText(
        error === "incorrect-password"
          ? t("database.migration.incorrect-password")
          : t("database.migration.invalid-file"),
        "error",
      );
      return;
    }

    // c) 画布名取文件名 stem
    const canvasName = fileNameStem(filePath.value);
    if (canvasName === "") {
      // 防御分支：文件路径不含文件名（理论上后端文件选择对话框不会返回这种路径），
      // 无法推导画布名，按无效文件提示并保持对话框打开供用户重选。
      snackbarText(t("database.migration.invalid-file"), "error");
      return;
    }
    // 根节点表示数据库本身：KeePass 根 group 名为空时以文件名兜底，
    // 保证根节点始终有可辨识的标题。
    if (tree.nodes[0].title === "") {
      tree.nodes[0].title = canvasName;
    }

    // d) 查找根画布（新画布节点的落点画布）
    const canvases = await userDatabaseCanvasList(false);
    const root = canvases.find((canvas) => canvas.parent_id === null);
    if (root === undefined) {
      // 不变量破坏：已打开的用户数据库必有根画布，找不到说明数据结构损坏，
      // 走受控崩溃而非静默容错（见 use-fatal-error 与 FatalErrorDialog）。
      // data.id 与后端同场景返回的 NoCanvasWithSuchId { id: "root" } 保持一致，
      // 供 FatalErrorDialog 的错误文案插值。
      triggerFatalError({ variant: "NoCanvasWithSuchId", data: { id: "root" } });
      return;
    }

    // e) 画布节点 y = 根画布现有节点的最大 y（无节点取 -240）+ 240，x 恒 0
    const existingNodes = await userDatabaseNodeList(root.id, false);
    const canvasNodeY =
      existingNodes.reduce((max, node) => Math.max(max, node.y), -240) + 240;

    // f) 就地写入树形布局坐标
    layoutImportedTree(tree);

    // g) 后端聚合导入（失败抛 ErrorCode，由末尾 catch 统一反馈），返回新画布 id
    const newCanvasId = await userDatabaseMigrationImportKeepass2(
      canvasName,
      0,
      canvasNodeY,
      tree.nodes,
      tree.edges,
    );

    dialog.value = false;
    snackbarText(t("database.migration.imported"), "success");
    emit("success", newCanvasId);
  } catch (error) {
    snackbarErrorCode(error);
  } finally {
    importing.value = false;
  }
}

defineExpose({ open });
</script>

<template>
  <VDialog v-model="dialog" max-width="30rem" :persistent="importing">
    <VCard>
      <VCardTitle>{{ t("database.migration.dialog-title") }}</VCardTitle>
      <VCardText>
        <div class="form-rows">
          <VTextField
            v-model="filePath"
            readonly
            variant="outlined"
            :label="t('database.migration.file-label')"
            :error-messages="fileError"
            hide-details="auto"
          >
            <template #append-inner>
              <VBtn
                variant="text"
                size="small"
                :loading="picking"
                @click="pickFile"
              >
                {{ t("database.migration.pick-file") }}
              </VBtn>
            </template>
          </VTextField>
          <PasswordField
            v-model="masterPassword"
            :label="t('database.migration.password-label')"
            :error-messages="passwordError"
          />
        </div>
      </VCardText>
      <VCardActions>
        <VSpacer />
        <VBtn variant="text" :disabled="importing" @click="dialog = false">
          {{ t("common.cancel") }}
        </VBtn>
        <VBtn
          color="primary"
          variant="flat"
          :loading="importing"
          @click="confirmImport"
        >
          {{ importing ? t("database.migration.importing") : t("common.confirm") }}
        </VBtn>
      </VCardActions>
    </VCard>
  </VDialog>
</template>

<style lang="scss" scoped>
.form-rows {
  display: flex;
  flex-direction: column;
  gap: 1rem;
  padding-top: 0.5rem;
}
</style>
