<!--
  应用根组件。

  在所有页面中保持可见的全局元素：右上角的语言切换、设置、主题切换与关于入口。
  主题菜单区分内置主题与自定义主题，并提供主题管理入口
  （新建/编辑/导出/导入/删除自定义主题）。
  自身样式直接消费 :root 上的 --v-theme-* 主题变量（随主题切换自动更新）。
-->
<script setup lang="ts">
import { computed, ref, useTemplateRef } from "vue";
import { currentLocale, setLocale, supportedLocales, t } from "@/i18n";
import { currentThemeName, setTheme, themeList } from "@/themes";
import AppSnackbarQueue from "@/components/AppSnackbarQueue.vue";
import FatalErrorDialog from "@/components/FatalErrorDialog.vue";
import ThemeManagerDialog from "@/components/ThemeManagerDialog.vue";
import SettingsDialog from "@/components/SettingsDialog.vue";
import AboutDialog from "@/components/AboutDialog.vue";
import { useClipboardClear } from "@/composables/use-clipboard-clear";

/** 主题菜单显示状态 */
const themeMenuOpen = ref(false);
const themeManagerRef = useTemplateRef<InstanceType<typeof ThemeManagerDialog>>(
  "themeManagerRef",
);
const settingsRef = useTemplateRef<InstanceType<typeof SettingsDialog>>(
  "settingsRef",
);
const aboutRef = useTemplateRef<InstanceType<typeof AboutDialog>>("aboutRef");
const { refreshKey, timeoutSeconds } = useClipboardClear();

/** 内置主题选项 */
const builtinItems = computed(() => themeList.value.filter((item) => item.builtin));
/** 自定义主题选项 */
const customItems = computed(() => themeList.value.filter((item) => !item.builtin));

/**
 * 切换语言（setLocale 内部自动持久化偏好）。
 * @param code 语言代码
 */
function switchLocale(code: string) {
  setLocale(code);
}

/**
 * 切换主题（setTheme 内部自动持久化偏好）。
 * @param name 主题名
 */
function switchTheme(name: string) {
  setTheme(name);
}

/** 关闭主题菜单并打开主题管理对话框 */
function openThemeManager() {
  themeMenuOpen.value = false;
  themeManagerRef.value?.open();
}
</script>

<template>
  <div id="app-root">
    <AppSnackbarQueue />
    <FatalErrorDialog />
    <div
      v-if="refreshKey > 0"
      :key="refreshKey"
      class="clipboard-countdown-bar"
      :style="{
        '--timeout-duration': `${timeoutSeconds}s`,
      }"
    />
    <div class="top-right-actions">
      <div class="frosted-btns frosted-glass">
        <VMenu offset="8">
          <template #activator="{ props }">
          <VIconBtn
            icon="mdi-web"
            variant="text"
            :aria-label="t('app.switch-language')"
            v-bind="props"
          />
          </template>
          <VList density="compact">
            <VListItem
              v-for="loc in supportedLocales"
              :key="loc.code"
              :title="loc.label"
              :active="currentLocale === loc.code"
              @click="switchLocale(loc.code)"
            />
          </VList>
        </VMenu>
        <VIconBtn
          icon="mdi-cog-outline"
          variant="text"
          :aria-label="t('app.settings')"
          @click="settingsRef?.open()"
        />
        <VMenu v-model="themeMenuOpen" offset="8" :close-on-content-click="false">
          <template #activator="{ props }">
          <VIconBtn
            icon="mdi-palette-outline"
            variant="text"
            :aria-label="t('app.switch-theme')"
            v-bind="props"
          />
          </template>
          <VList density="compact">
            <VListSubheader>{{ t("themes.builtin-group") }}</VListSubheader>
            <VListItem
              v-for="item in builtinItems"
              :key="item.name"
              :title="item.displayName"
              :active="currentThemeName === item.name"
              @click="switchTheme(item.name)"
            />
            <template v-if="customItems.length > 0">
              <VListSubheader>{{ t("themes.custom-group") }}</VListSubheader>
              <VListItem
                v-for="item in customItems"
                :key="item.name"
                :title="item.displayName"
                :active="currentThemeName === item.name"
                @click="switchTheme(item.name)"
              />
            </template>
            <VDivider class="my-1" />
            <VListItem
              prepend-icon="mdi-cog-outline"
              :title="t('themes.manage')"
              @click="openThemeManager"
            />
          </VList>
        </VMenu>
        <VIconBtn
          icon="mdi-information-outline"
          variant="text"
          :aria-label="t('app.about')"
          @click="aboutRef?.open()"
        />
      </div>
    </div>
    <RouterView />
    <ThemeManagerDialog ref="themeManagerRef" />
    <SettingsDialog ref="settingsRef" />
    <AboutDialog ref="aboutRef" />
  </div>
</template>

<style lang="scss">
#app-root {
  width: 100%;
  height: 100%;
  // 自定义组件直接消费 Vuetify 主题变量（变量值为 RGB 三元组，需包 rgb()）
  background-color: rgb(var(--v-theme-background));
  color: rgb(var(--v-theme-on-background));
  transition:
    background-color 0.25s ease,
    color 0.25s ease;

  .top-right-actions {
    position: fixed;
    top: 1rem;
    right: 1rem;
    z-index: 100;
    animation: actions-enter 0.35s ease-out;
  }

  .frosted-btns {
    display: flex;
    gap: 0.25rem;
    padding: 0.25rem;
    border-radius: 0.5rem;
  }
}

@keyframes actions-enter {
  from {
    opacity: 0;
    transform: translateY(-0.75rem);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

.clipboard-countdown-bar {
  position: fixed;
  top: 0;
  left: 0;
  height: 0.25rem;
  z-index: 200;
  animation: clip-progress var(--timeout-duration) linear forwards;
}

@keyframes clip-progress {
  0% {
    width: 100%;
    background-color: #4caf5f;
  }
  100% {
    width: 0%;
    background-color: #f44336;
  }
}
</style>
