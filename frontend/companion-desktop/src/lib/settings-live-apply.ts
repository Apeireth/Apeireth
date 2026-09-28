// 设置面「三级即效」语义的纯函数件（无副作用）。
//
// 设置页没有「保存」按钮——每类控件只有一条生效时机（三级即效语义）：
//   1. 开关 / 滑杆 / 预设：点（或松手）即生效，走 apply 路径；
//   2. 文本输入：失焦或回车提交（服务商组整组提交，避免半截配置）；
//   3. 危险动作：即点 + 二次确认弹层，确认后即效。
// 本模块把「提交判定 / 整组写回 / 危险动作确认文案」拆成纯函数，供
// SettingsView / ThemeSettingsPanel 与映射测试共用同一份实现。
// 开关与旋钮的映射见 capability-apply.ts。

import type {
  ApeirethConfig,
  PersonaProfile,
  ProviderConfig,
  ProviderProtocol,
} from './types';

// ---------------------------------------------------------------------------
// 文本提交判定（第 2 级）
// ---------------------------------------------------------------------------

/** 文本输入的回车 = 提交（失焦提交由整组 focusout 判定负责）。 */
export function isTextCommitKey(key: string): boolean {
  return key === 'Enter';
}

/**
 * 整组失焦判定：下一个焦点仍在组内 = 组内移动焦点，不提交；
 * 离开组（或落到页面外，relatedTarget 为空）= 整组提交。
 */
export function focusLeavesGroup(nextTargetInsideGroup: boolean): boolean {
  return nextTargetInsideGroup !== true;
}

/** 与上次提交快照一致 = 空提交：不打扰运行时，只清「未保存」微提示。 */
export function isSameAsCommitted(current: string, committed: string): boolean {
  return current === committed;
}

/** 文本字段旁注状态：未提交显「未保存」，提交成功闪 ✓。 */
export type TextCommitFlag = 'dirty' | 'ack' | null;

export function textCommitFlag(dirty: boolean, ack: boolean): TextCommitFlag {
  if (dirty === true) return 'dirty';
  if (ack === true) return 'ack';
  return null;
}

// ---------------------------------------------------------------------------
// 整组写回（叠加不覆盖：以 config 为底，只换本次提交面的字段）
// ---------------------------------------------------------------------------

/** 服务商组草稿：端点 + 模型 + 密钥 + 协议头（整组一个提交原子）。 */
export interface ProviderGroupDraft {
  protocol: ProviderProtocol;
  preset: string;
  baseUrl: string;
  apiKey: string;
  model: string;
  anthropicVersion: string;
}

/** 服务商组的提交快照：trim 后比较，纯空白差异不算改动。 */
export function providerGroupSnapshot(group: ProviderGroupDraft): string {
  return JSON.stringify({
    protocol: group.protocol,
    preset: group.preset,
    baseUrl: group.baseUrl.trim(),
    apiKey: group.apiKey.trim(),
    model: group.model.trim(),
    anthropicVersion: group.anthropicVersion.trim(),
  });
}

/**
 * 服务商组整组写回：端点 / 模型 / 密钥 / 协议头一次性落位（OpenAI 与
 * Anthropic 两份镜像缓冲同步换），其余配置字段不动。整组一次提交——
 * 半截配置（只改了端点没带模型）永远不会被推到运行时。
 */
export function configWithProviderGroup(
  config: ApeirethConfig,
  group: ProviderGroupDraft,
  fallbackModel?: string,
): ApeirethConfig {
  const baseUrl = group.baseUrl.trim();
  const apiKey = group.apiKey.trim();
  const model = group.model.trim();
  const anthropicVersion = group.anthropicVersion.trim();
  const provider: ProviderConfig = {
    protocol: group.protocol,
    preset: group.preset,
    baseUrl,
    apiKey,
    model,
    anthropicVersion: group.protocol === 'anthropic' ? anthropicVersion : undefined,
  };
  return {
    ...config,
    model: model || fallbackModel || config.model,
    provider,
    openaiConfig:
      group.protocol === 'openai'
        ? {preset: group.preset, baseUrl, apiKey, model}
        : config.openaiConfig,
    anthropicConfig:
      group.protocol === 'anthropic'
        ? {preset: group.preset, baseUrl, apiKey, model, anthropicVersion}
        : config.anthropicConfig,
  };
}

/** 网关服务地址提交：只换 baseUrl（其余字段不动）。 */
export function configWithGatewayUrl(config: ApeirethConfig, url: string): ApeirethConfig {
  return {...config, baseUrl: url.trim()};
}

/** 人设组写回：整组人设 + 当前伙伴 id 一次性落位（其余字段不动）。 */
export function configWithPersonas(
  config: ApeirethConfig,
  personas: PersonaProfile[],
  activePersonaId: string,
): ApeirethConfig {
  return {
    ...config,
    personas: personas.map((p) => ({...p})),
    activePersonaId,
  };
}

/** 人设组的提交快照（含当前伙伴 id）。 */
export function personasSnapshot(personas: PersonaProfile[], activePersonaId: string): string {
  return JSON.stringify({personas, activePersonaId});
}

// ---------------------------------------------------------------------------
// 危险动作二次确认（第 3 级）
// ---------------------------------------------------------------------------

/**
 * 危险动作 = 删数据 / 清记忆 / 断开连接类：点按钮只打开确认弹层，
 * 确认后才即效。登记表是唯一文案来源（测试镜像校验覆盖面）。
 */
export type DangerActionKey = 'clearLocalData' | 'deleteStoredKey' | 'removePersona' | 'clearCustomBg';

export interface DangerActionConfirm {
  title: string;
  message: string;
  confirmText: string;
}

export const DANGER_ACTION_CONFIRMATIONS: Record<DangerActionKey, DangerActionConfirm> = {
  clearLocalData: {
    title: '清空本地所有会话',
    message: '确定要清空本地保存的所有会话记录吗？此操作无法撤销。',
    confirmText: '确认清空',
  },
  deleteStoredKey: {
    title: '删除已存密钥',
    message: '将从系统钥匙串删除该服务商的 API 密钥，之后请求会因缺少密钥被拒绝。确定删除吗？',
    confirmText: '确认删除',
  },
  removePersona: {
    title: '删除该伙伴',
    message: '将从配置里删除这个伙伴及其人设文本，确认后立即生效。确定删除吗？',
    confirmText: '确认删除',
  },
  clearCustomBg: {
    title: '清除已传图片',
    message: '将删除本机浏览器数据库里已上传的背景图片，确认后立即生效且无法恢复。确定清除吗？',
    confirmText: '确认清除',
  },
};

/** 危险动作必须走二次确认（登记表里没有的动作不许即点即删）。 */
export function requiresDangerConfirm(action: DangerActionKey): boolean {
  return Object.prototype.hasOwnProperty.call(DANGER_ACTION_CONFIRMATIONS, action);
}
