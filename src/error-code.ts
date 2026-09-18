/**
 * 后端 ErrorCode 的前端定义。
 *
 * 与后端 error_code.rs 的枚举项一一对应，采用 serde
 * tag = "variant"、content = "data" 的序列化格式；
 * variant 和 data 直接透传给 i18n 模块做文案插值（见 i18n 的 error-code 模块）。
 */
import { isPlainObject, isString } from "lodash";

/** ErrorCode 变体名（与后端 ErrorCode 枚举项一一对应） */
export type ErrorCodeVariant =
  | "AttachmentTooLarge"
  | "CanvasNameAlreadyExists"
  | "CanvasToDataNodeEdge"
  | "DatabaseError"
  | "DataCorruptionCanvasRefMissing"
  | "DataCorruptionDanglingShadow"
  | "DataCorruptionEdgeEndpointMissing"
  | "DataCorruptionMissingShadow"
  | "DataCorruptionNodeFieldValueInvalidUtf8"
  | "DataCorruptionNodeNotShadow"
  | "DataCorruptionShadowChainCycle"
  | "DataCorruptionShadowEdgeDirectionMismatch"
  | "DataCorruptionShadowNeighborIsShadow"
  | "DataCorruptionShadowOriginTypeMismatch"
  | "DataVersionMismatch"
  | "DuplicateDictionaryId"
  | "DuplicateNodeFieldName"
  | "EdgeDeleteDisconnectsNodes"
  | "EdgeSameHandle"
  | "EdgeWouldFormCycle"
  | "EmptyCanvasName"
  | "EmptyDictionaryValue"
  | "EmptyFileName"
  | "EmptyFilePath"
  | "EmptyImportedCanvasList"
  | "EmptyNodeFieldName"
  | "EmptyPassword"
  | "EmptyPreferenceName"
  | "EmptyRegistryName"
  | "EmptyTemplateName"
  | "EmptyUserDatabaseName"
  | "FailToCreateDirectory"
  | "FailToDecrypt"
  | "FailToDeserializeAction"
  | "FailToDeserializeDatabase"
  | "FailToEncrypt"
  | "FailToOpenConnection"
  | "FailToReadDirectory"
  | "FailToReadFile"
  | "FailToRemoveDirectory"
  | "FailToRemoveFile"
  | "FailToSerializeAction"
  | "FailToSerializeDatabase"
  | "FailToTryExists"
  | "FailToWriteFile"
  | "InvalidAttachmentId"
  | "InvalidCanvasId"
  | "InvalidCiphertext"
  | "InvalidDictionaryId"
  | "InvalidEdgeId"
  | "InvalidExportMode"
  | "InvalidExportTargetPath"
  | "InvalidImportedCanvasIndex"
  | "InvalidImportedCanvasRefIndex"
  | "InvalidImportedEdgeIndex"
  | "InvalidKdbxFileExtension"
  | "InvalidNodeId"
  | "InvalidHandle"
  | "InvalidPath"
  | "InvalidShadowEdge"
  | "InvalidTemplateId"
  | "InvalidUserDatabaseId"
  | "MultipleDataVersion"
  | "NoAttachmentWithSuchId"
  | "NoCanvasWithSuchId"
  | "NoDataVersion"
  | "NoDictionaryWithSuchId"
  | "NoEdgeWithSuchId"
  | "NoNodeWithSuchId"
  | "NoTemplateWithSuchId"
  | "NoUserDatabaseWithSuchId"
  | "NodeDeleteDisconnectsNodes"
  | "NodeIsCanvasDataNode"
  | "NodeIsShadow"
  | "NodeNotInSameCanvas"
  | "NodeSetHasExternalEdges"
  | "RootCanvasCannotBeDeleted"
  | "ShadowToShadowEdge"
  | "TemplateNameAlreadyExists"
  | "UserDatabaseMustBeArchivedBeforeDelete"
  | "UserDatabaseNameAlreadyExists"
  | "UserDatabaseNotOpen"
  | "AttachmentNotInSameNode"
  | "AttachmentDeleted"
  | "FailToCompress"
  | "FailToDecompress"
  | "ClipboardError"
  | "InvalidBackupFile"
  | "UnsupportedBackupVersion"
  | "BackupTooManyShardsLost"
  | "FailToPackBackup"
  | "FailToUnpackBackup"
  | "InvalidLogTimeRange";

/** 后端 ErrorCode 的前端表示 */
export type ErrorCode = { variant: ErrorCodeVariant; data?: Record<string, unknown> };

/**
 * 判断未知错误是否为后端返回的 ErrorCode，可选地限定变体名。
 * @param raw 待判断的错误对象
 * @param variant 限定的变体名，不传则只判断是否为 ErrorCode
 * @returns 是否匹配
 */
export function isErrorCode(
  raw: unknown,
  variant?: ErrorCodeVariant,
): raw is ErrorCode {
  if (!isPlainObject(raw)) return false;
  const candidate = raw as { variant?: unknown };
  if (!isString(candidate.variant)) return false;
  return variant === undefined || candidate.variant === variant;
}
