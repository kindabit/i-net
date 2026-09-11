<!--
  日志行为内容渲染组件。

  按行为类型渲染单条日志的名称与详情文案，置于日志列表项的默认插槽中使用；
  节点字段编辑行为额外渲染逐字段变更列表（含掩码字段值与空值处理）。
-->
<script setup lang="ts">
import { computed } from "vue";
import { t, te } from "@/i18n";
import type { LogAction, NodeFieldChange } from "@/api-types";
import { fieldTypeDisplayName, isFieldTypeMasked } from "@/field-types";

interface Props {
  /** 日志行为及其数据 */
  action: LogAction;
  /** 是否明文显示掩码字段值（false 时掩码字段值显示为圆点） */
  showSensitive: boolean;
}

const props = defineProps<Props>();

/** 节点字段编辑行为的数据载荷（对应后端 Action::NodeFieldsModify 的 data 形状） */
type NodeFieldsModifyData = {
  /** 节点标题 */
  node_title: string;
  /** 逐字段的变更列表 */
  changes: NodeFieldChange[];
};

/**
 * 节点字段编辑行为的数据；其它行为类型时为 null。
 * 模板据此切换节点字段编辑的专属渲染分支。
 */
const nodeFieldsModifyData = computed(() =>
  props.action.variant === "NodeFieldsModify"
    ? (props.action.data as NodeFieldsModifyData)
    : null,
);

/**
 * 将日志行为的数据转换为 i18n 插值参数。
 * @param action 日志行为
 * @returns 键值对形式的插值参数对象
 */
function detailParams(action: LogAction): Record<string, string> {
  return Object.fromEntries(
    Object.entries(action.data).map(([k, v]) => [k, String(v)]),
  );
}

/**
 * 格式化日志中的字段变更值：无值显示空值文案；掩码且未开启明文显示时显示圆点；否则直接显示值原文。
 * @param value 字段值字符串
 * @param masked 该字段类型是否掩码显示
 * @returns 用于展示的字符串
 */
function formatChangeValue(value: unknown, masked: boolean): string {
  const str = value as string | null;
  if (str === null || str === undefined) return t("log.empty-value");
  if (masked && !props.showSensitive)
    return "••••••••";
  return str;
}

/**
 * 渲染单条字段变更的日志文案。
 * @param change 节点字段变更对象
 * @returns 可直接展示的文案字符串
 */
function renderChange(change: NodeFieldChange): string {
  switch (change.variant) {
    case "Added": {
      const typeName = fieldTypeDisplayName(change.data.field_type);
      const masked = isFieldTypeMasked(change.data.field_type);
      const value = formatChangeValue(change.data.value, masked);
      return t("log.node-fields-change-added", {
        name: change.data.name,
        type: typeName,
        value,
      });
    }
    case "Modified": {
      const oldTypeName = fieldTypeDisplayName(change.data.old_field_type);
      const newTypeName = fieldTypeDisplayName(change.data.new_field_type);
      // 新旧类型任一为掩码类型时，新旧值均掩码显示。
      const masked =
        isFieldTypeMasked(change.data.old_field_type) ||
        isFieldTypeMasked(change.data.new_field_type);
      const oldValue = formatChangeValue(change.data.old_value, masked);
      const newValue = formatChangeValue(change.data.new_value, masked);
      return t("log.node-fields-change-modified", {
        name: change.data.name,
        oldType: oldTypeName,
        newType: newTypeName,
        oldValue,
        newValue,
      });
    }
    case "Removed": {
      const typeName = fieldTypeDisplayName(change.data.field_type);
      const masked = isFieldTypeMasked(change.data.field_type);
      const oldValue = formatChangeValue(change.data.old_value, masked);
      return t("log.node-fields-change-removed", {
        name: change.data.name,
        type: typeName,
        oldValue,
      });
    }
  }
}
</script>

<template>
  <template v-if="nodeFieldsModifyData">
    <div>
      <div class="text-body-2 font-weight-medium">
        {{ t("log.action.NodeFieldsModify.name") }}
      </div>
      <div class="text-body-2">
        {{
          t("log.action.NodeFieldsModify.detail", {
            node_title: nodeFieldsModifyData.node_title,
          })
        }}
      </div>
      <div class="log-changes ml-4 mt-1">
        <div
          v-for="(change, ci) in nodeFieldsModifyData.changes"
          :key="ci"
          class="text-caption log-change-item"
        >
          {{ renderChange(change) }}
        </div>
      </div>
    </div>
  </template>
  <template v-else>
    <VListItemTitle>
      {{
        te(`log.action.${action.variant}.name`)
          ? t(`log.action.${action.variant}.name`)
          : action.variant
      }}
    </VListItemTitle>
    <VListItemSubtitle>
      {{
        te(`log.action.${action.variant}.detail`)
          ? t(`log.action.${action.variant}.detail`, detailParams(action))
          : ""
      }}
    </VListItemSubtitle>
  </template>
</template>

<style lang="scss" scoped>
.log-changes {
  .log-change-item {
    padding-top: 0.125rem;
    padding-bottom: 0.125rem;
    color: rgba(var(--v-theme-on-surface), var(--v-medium-emphasis-opacity));
  }
}
</style>
