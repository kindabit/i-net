<!--
  首页 / 数据目录页。

  提供数据库名称输入（带历史下拉）与密码输入两步流程，输入完成后打开
  数据库（名称未注册时先注册）并跳转至画布页；右下角垃圾箱按钮打开归档管理对话框。
-->
<script lang="ts">
/** registry 中"上次场景"的键名；值为画布 id，空值代表画布宇宙 */
export const LAST_SCENE_KEY = "lastScene";
</script>

<script setup lang="ts">
import { computed, onMounted, ref, useTemplateRef } from "vue";
import { t, d } from "@/i18n";
import { useRouter } from "vue-router";
import {
  metadataList,
  metadataRegister,
  metadataSave,
  reclaimMetadata,
  reclaimPreference,
  reclaimUserDatabase,
  userDatabaseCanvasList,
  userDatabaseLifecycleInitialize,
  userDatabaseRegistryGet,
  userDatabaseRegistrySet,
} from "@/api";
import type { Metadata } from "@/api-types";
import { snackbarErrorCode } from "@/composables/use-snackbar";
import AutoCompleteField from "@/components/AutoCompleteField.vue";
import PasswordField from "@/components/PasswordField.vue";
import ArchiveManagementDialog from "@/views/HomeComponents/ArchiveManagementDialog.vue";
import BackupDialog from "@/views/HomeComponents/BackupDialog.vue";
import RestoreDialog from "@/views/HomeComponents/RestoreDialog.vue";
import RestoreSuccessDialog from "@/views/HomeComponents/RestoreSuccessDialog.vue";

const router = useRouter();

/** 当前输入步骤：数据库名称或密码 */
const step = ref<"name" | "password">("name");
/** 面板切换动画名（由步骤切换方向决定） */
const transitionName = computed(() =>
  step.value === "password" ? "step-forward" : "step-back",
);

/** 输入的数据库名称 */
const dbName = ref("");
/** 输入的密码 */
const password = ref("");
/** 确认密码（仅新建数据库时填写） */
const passwordConfirm = ref("");
/** 名称输入错误提示 */
const nameError = ref("");
/** 密码输入错误提示 */
const passwordError = ref("");
/** 注册提交状态 */
const submitting = ref(false);
/** 未归档数据库元数据列表 */
const metadatas = ref<Metadata[]>([]);

const nameInputRef = useTemplateRef<InstanceType<typeof AutoCompleteField>>(
  "nameInputRef",
);
const passwordInputRef = useTemplateRef<{ focus: () => void }>(
  "passwordInputRef",
);
const passwordConfirmInputRef = useTemplateRef<{ focus: () => void }>(
  "passwordConfirmInputRef",
);
const archiveDialogRef = useTemplateRef<
  InstanceType<typeof ArchiveManagementDialog>
>("archiveDialogRef");
const backupDialogRef = useTemplateRef<InstanceType<typeof BackupDialog>>(
  "backupDialogRef",
);
const restoreDialogRef = useTemplateRef<InstanceType<typeof RestoreDialog>>(
  "restoreDialogRef",
);
const restoreSuccessDialogRef = useTemplateRef<
  InstanceType<typeof RestoreSuccessDialog>
>("restoreSuccessDialogRef");

onMounted(async () => {
  void refreshMetadatas();
  // 空闲时预加载数据库布局页组件，加快进入数据库的速度
  const preloadDatabaseView = () => void import("@/views/DatabaseView.vue");
  if ("requestIdleCallback" in window) {
    requestIdleCallback(preloadDatabaseView);
  } else {
    setTimeout(preloadDatabaseView, 200);
  }
});

/**
 * 刷新未归档数据库列表。
 * @returns 无返回值
 */
async function refreshMetadatas() {
  try {
    metadatas.value = await metadataList(false);
  } catch (error) {
    snackbarErrorCode(error);
  }
}

/**
 * 还原成功后刷新三个模块的内存 connection，然后让用户在 success dialog
 * 中点击「重启」销毁窗口以彻底重置后端状态。
 * 任一 reclaim 失败则弹错误，不显示成功对话框、不销毁窗口。
 * @returns 无返回值
 */
async function onRestoreSuccess() {
  try {
    await reclaimPreference();
    await reclaimMetadata();
    await reclaimUserDatabase();
    restoreSuccessDialogRef.value?.open();
  } catch (error) {
    snackbarErrorCode(error);
  }
}

/** 下拉候选列表（附最后打开时间） */
const dropdownItems = computed(() =>
  metadatas.value.map((metadata) => ({
    title: metadata.name,
    subtitle: d(new Date(metadata.last_open_time), "short"),
    value: metadata.name,
  })),
);

/** 当前输入的名称是否对应已注册数据库（决定密码步骤是"打开"还是"新建"） */
const isExistingDatabase = computed(() =>
  metadatas.value.some((metadata) => metadata.name === dbName.value.trim()),
);

/** 校验名称并进入密码输入步骤 */
function validateNameAndNext() {
  nameError.value = "";
  if (dbName.value.trim() === "") {
    nameError.value = t("home.validation.name-empty");
    return;
  }
  nameInputRef.value?.blur();
  step.value = "password";
}

/** 返回名称输入步骤并清空密码 */
function goBack() {
  password.value = "";
  passwordConfirm.value = "";
  passwordError.value = "";
  step.value = "name";
}

/** 入场或面板切换动画结束后聚焦当前步骤的输入框 */
function onPanelAfterEnter() {
  if (step.value === "password") {
    passwordInputRef.value?.focus();
  } else {
    nameInputRef.value?.focus();
  }
}

/**
 * 校验密码并打开数据库（名称未注册时先注册再打开），成功后跳转画布页。
 * @returns 无返回值
 */
async function submitPassword() {
  passwordError.value = "";
  if (password.value === "") {
    passwordError.value = t("home.validation.password-empty");
    return;
  }
  if (password.value.length < 6) {
    passwordError.value = t("home.validation.password-too-short");
    return;
  }
  if (!isExistingDatabase.value && password.value !== passwordConfirm.value) {
    passwordError.value = t("home.validation.password-mismatch");
    return;
  }
  const name = dbName.value.trim();
  submitting.value = true;
  try {
    // 名称已注册则直接打开，否则先注册再打开
    const existing = metadatas.value.find(
      (metadata) => metadata.name === name,
    );
    const id = existing ? existing.id : (await metadataRegister(name)).id;
    await userDatabaseLifecycleInitialize(id, password.value);
    void metadataSave().catch(snackbarErrorCode);
    const lastCanvasId = await userDatabaseRegistryGet(LAST_SCENE_KEY);
    if (lastCanvasId) {
      // 上次在某个画布内：校验画布仍存在（可能已被删除），失效则静默回退画布宇宙
      const canvases = await userDatabaseCanvasList(false);
      if (canvases.some((canvas) => canvas.id === lastCanvasId)) {
        await router.push({ name: "canvas", params: { canvasId: lastCanvasId } });
      } else {
        void userDatabaseRegistrySet(LAST_SCENE_KEY, "").catch(snackbarErrorCode);
        await router.push("/database");
      }
    } else {
      await router.push("/database");
    }
  } catch (error) {
    snackbarErrorCode(error);
  } finally {
    submitting.value = false;
  }
}

/**
 * 第一个密码框的回车处理：新建数据库且确认密码框为空时，聚焦确认密码框而非直接提交。
 * @returns 无返回值
 */
function onPasswordEnter() {
  if (!isExistingDatabase.value && passwordConfirm.value === "") {
    passwordConfirmInputRef.value?.focus();
    return;
  }
  void submitPassword();
}
</script>

<template>
  <div class="home-viewport">
    <Transition name="entrance" appear @after-enter="onPanelAfterEnter">
      <div class="entrance-wrapper">
        <div class="panel-host">
          <Transition
            :name="transitionName"
            mode="out-in"
            @after-enter="onPanelAfterEnter"
          >
            <!-- 数据库名称面板 -->
            <div v-if="step === 'name'" key="name" class="panel">
              <div class="input-row">
                <AutoCompleteField
                  ref="nameInputRef"
                  v-model="dbName"
                  :items="dropdownItems"
                  :label="t('home.database-name')"
                  :error-messages="nameError"
                  variant="outlined"
                  class="input-field"
                  @submit="validateNameAndNext"
                />
                <VBtn
                  icon="mdi-arrow-right"
                  variant="text"
                  class="enter-btn"
                  :aria-label="t('common.confirm')"
                  @click="validateNameAndNext"
                />
              </div>
            </div>
            <!-- 密码面板：已有数据库输入密码，新建数据库额外确认密码 -->
            <div v-else key="password" class="panel">
              <div class="input-row">
                <div class="input-fields">
                  <PasswordField
                    ref="passwordInputRef"
                    v-model="password"
                    :label="
                      isExistingDatabase
                        ? t('home.password')
                        : t('home.password-create')
                    "
                    :error-messages="passwordError"
                    class="input-field"
                    @keydown.enter.prevent="onPasswordEnter"
                  />
                  <PasswordField
                    v-if="!isExistingDatabase"
                    ref="passwordConfirmInputRef"
                    v-model="passwordConfirm"
                    :label="t('home.password-confirm')"
                    class="input-field"
                    @keydown.enter.prevent="submitPassword"
                  />
                </div>
                <VBtn
                  icon="mdi-arrow-left"
                  variant="text"
                  class="enter-btn"
                  :aria-label="t('home.back')"
                  @click="goBack"
                />
                <VBtn
                  icon="mdi-check"
                  variant="text"
                  class="enter-btn"
                  :loading="submitting"
                  :aria-label="t('common.confirm')"
                  @click="submitPassword"
                />
              </div>
            </div>
          </Transition>
        </div>
        <div class="bottom-actions">
          <div class="frosted-btns frosted-glass">
            <!-- 归档管理：保留原有功能入口 -->
            <VTooltip :text="t('home.archive-management-tooltip')" location="top">
              <template #activator="{ props }">
                <VIconBtn
                  v-bind="props"
                  icon="mdi-delete-outline"
                  color="error"
                  variant="text"
                  :aria-label="t('home.archive-management-tooltip')"
                  @click="archiveDialogRef?.open()"
                />
              </template>
            </VTooltip>
            <!-- 备份：全量备份数据目录 -->
            <VTooltip :text="t('home.backup-tooltip')" location="top">
              <template #activator="{ props }">
                <VIconBtn
                  v-bind="props"
                  icon="mdi-content-save-outline"
                  variant="text"
                  :aria-label="t('home.backup-tooltip')"
                  @click="backupDialogRef?.open()"
                />
              </template>
            </VTooltip>
            <!-- 还原：从备份恢复数据目录 -->
            <VTooltip :text="t('home.restore-tooltip')" location="top">
              <template #activator="{ props }">
                <VIconBtn
                  v-bind="props"
                  icon="mdi-restore"
                  variant="text"
                  :aria-label="t('home.restore-tooltip')"
                  @click="restoreDialogRef?.open()"
                />
              </template>
            </VTooltip>
          </div>
        </div>
      </div>
    </Transition>
    <ArchiveManagementDialog ref="archiveDialogRef" @update="refreshMetadatas" />
    <BackupDialog ref="backupDialogRef" />
    <RestoreDialog ref="restoreDialogRef" @success="onRestoreSuccess" />
    <RestoreSuccessDialog ref="restoreSuccessDialogRef" />
  </div>
</template>

<style lang="scss" scoped>
.home-viewport {
  position: relative;
  width: 100%;
  height: 100%;
  overflow: hidden;
}

.entrance-wrapper {
  position: relative;
  width: 100%;
  height: 100%;
}

.panel-host {
  position: absolute;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
}

.panel {
  width: 100%;
  display: flex;
  justify-content: center;
}

.input-row {
  width: 50%;
  min-width: 20rem;
  max-width: 80%;
  display: flex;
  align-items: flex-start;
  gap: 0.25rem;

  .input-fields {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .input-field {
    flex: 1;
  }

  .enter-btn {
    margin-top: 0.5rem;
    flex-shrink: 0;
  }
}

.bottom-actions {
  position: absolute;
  bottom: 1rem;
  right: 1rem;
  z-index: 10;
}

.frosted-btns {
  display: flex;
  gap: 0.25rem;
  padding: 0.25rem;
  border-radius: 0.5rem;
}

// 入场动画：轻微上浮淡入
.entrance-enter-active {
  transition:
    opacity 0.3s ease,
    transform 0.3s ease;
}

.entrance-enter-from {
  opacity: 0;
  transform: translateY(0.75rem);
}

// 步骤切换动画：小幅度水平滑动 + 淡入淡出
.step-forward-enter-active,
.step-forward-leave-active,
.step-back-enter-active,
.step-back-leave-active {
  transition:
    opacity 0.22s ease,
    transform 0.22s ease;
}

.step-forward-enter-from {
  opacity: 0;
  transform: translateX(1.5rem);
}

.step-forward-leave-to {
  opacity: 0;
  transform: translateX(-1.5rem);
}

.step-back-enter-from {
  opacity: 0;
  transform: translateX(-1.5rem);
}

.step-back-leave-to {
  opacity: 0;
  transform: translateX(1.5rem);
}
</style>
