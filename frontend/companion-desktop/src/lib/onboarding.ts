// 首启引导（OOBE）—— 应用内分步弹窗的数据层与纯逻辑。
//
// 职责（与 FirstRunWizard.svelte 分工：本模块零 DOM，可被 Node 直接 import 测试）：
//   - 分流模型：普通用户（预设值 + 使用中自动调参）/ 专业用户（只做介绍）；
//   - 分步顺序（依次提示）：欢迎/介绍 → 分流 →（普通：API 设置 → 自我描述词 →
//     预设确认；专业：参考卡）；
//   - 普通用户预设：RECOMMENDED_CAPABILITY_PRESET 六旋钮 + selfTuning（自动调参）
//     ——危险能力（shell/fetch/文件写）**绝不**进任何预设；
//   - 自我描述词落点：用户印象（user-impression）——"他如何理解你"，随每次对话
//     作为 system 自我参考注入（与 runtime 注入链同一数据源）。
//
// 专业用户轨道**只做介绍**：不写任何预设 / 配置 / 印象。

import {
  DEFAULT_CAPABILITY_TOGGLES,
  RECOMMENDED_CAPABILITY_PRESET,
  type ApeirethConfig,
  type CapabilityToggles,
  type ProviderConfig,
} from './types.ts';
import {activePersonaOf, loadConfig} from './runtime.ts';
import {
  emptyUserImpression,
  saveUserImpression,
  IMPRESSION_MAX_CHARS,
  type UserImpression,
} from './user-impression.ts';

/** 用户分流。 */
export type OnboardingTier = 'casual' | 'pro';

/** 引导步骤（依次提示的顺序）。 */
export type OnboardingStep = 'welcome' | 'tier' | 'api' | 'identity' | 'preset' | 'pro-guide';

/** 普通用户轨道：欢迎 → 分流 → API 设置 → 自我描述词 → 预设确认。 */
export const CASUAL_STEPS: ReadonlyArray<OnboardingStep> = [
  'welcome',
  'tier',
  'api',
  'identity',
  'preset',
];

/** 专业用户轨道：欢迎 → 分流 → 参考卡（只做介绍）。 */
export const PRO_STEPS: ReadonlyArray<OnboardingStep> = ['welcome', 'tier', 'pro-guide'];

/** 按分流取步序（未分流 = 按普通用户预览步序）。 */
export function stepsFor(tier: OnboardingTier | null): ReadonlyArray<OnboardingStep> {
  return tier === 'pro' ? PRO_STEPS : CASUAL_STEPS;
}

/** 简单介绍要点（欢迎页）。 */
export const INTRO_BULLETS: ReadonlyArray<string> = [
  '对话：多会话、流式出字，审批与审计都在本地留痕',
  '治理：危险能力（shell / fetch / 文件写）默认关闭，逐次人工审批',
  '记忆：本地记忆库，核心记忆族可一键开启',
  '伙伴：阿佩瑞斯带着人设与用户印象与你相处，换人设即换档案',
];

/** 引导预告（欢迎页底部）。 */
export const INTRO_PLAN: ReadonlyArray<string> = [
  '① 选择使用方式（普通用户 / 专业用户）',
  '② 连接模型服务（端点、模型、密钥）',
  '③ 自我描述词（他如何理解你）',
  '④ 普通用户预设值（使用中自动调整参数）；专业用户只做介绍',
];

/** 专业用户参考卡（只做介绍，不写任何预设）。 */
export const PRO_GUIDE_LINES: ReadonlyArray<string> = [
  '设置 ›「连接」：服务商、端点、模型、密钥（密钥只进系统钥匙串）',
  '设置 ›「伙伴档案」：人设、用户印象、记忆倾向、性情旋钮与成长日志',
  '设置 ›「能力与安全」：工具与高级能力开关（危险项默认关，逐次人工审批）',
  '设置 ›「外观」：主题、强调色、背景',
  '预算旋钮（回合轮数 / 工具调用 / 上下文预算）留空 = 后端默认，填写即显式生效',
];

/**
 * 普通用户预设：推荐配置六旋钮 + 自动调参（selfTuning）。
 * 危险能力绝不进预设（shell / fetch / fileWrite 等保持 base 原值）。
 */
export function casualCapabilities(
  autoTune: boolean,
  base: CapabilityToggles = DEFAULT_CAPABILITY_TOGGLES,
): CapabilityToggles {
  const next: CapabilityToggles = {...base};
  for (const key of RECOMMENDED_CAPABILITY_PRESET) {
    next[key] = true;
  }
  next.selfTuning = autoTune;
  return next;
}

/** 自我描述词 → 用户印象条目（"他如何理解你"的初始档案）。 */
export function impressionFromSelfDescription(
  selfDescription: string,
  personaId: string,
  personaName: string,
): UserImpression {
  const entry = emptyUserImpression(personaId, personaName);
  entry.summary = selfDescription.trim().slice(0, IMPRESSION_MAX_CHARS);
  entry.source = 'manual';
  entry.updatedAt = Date.now();
  return entry;
}

/**
 * 把自我描述词写进当前伙伴的用户印象（随每次对话注入 system 自我参考）。
 * 返回落点身份（与 runtime 注入侧的 key 口径一致）。
 */
export function writeSelfDescription(selfDescription: string): {
  personaId: string;
  personaName: string;
} {
  const persona = activePersonaOf(loadConfig());
  const personaId = persona?.id?.trim() || 'default';
  const personaName = persona?.name ?? '';
  saveUserImpression(impressionFromSelfDescription(selfDescription, personaId, personaName));
  return {personaId, personaName};
}

/** 向导收集的连接选择（密钥只进钥匙串，绝不落盘明文）。 */
export interface WizardProviderChoice {
  presetId: string;
  baseUrl: string;
  model: string;
  apiKey: string;
}

/** 普通用户完成页 → App 侧合并配置（`completeFirstRun` 浅合并）。 */
export function buildWizardConfig(
  choice: WizardProviderChoice,
  autoTune: boolean,
): ApeirethConfig {
  const provider: ProviderConfig = {
    protocol: 'openai',
    preset: choice.presetId,
    baseUrl: choice.baseUrl,
    apiKey: choice.apiKey,
    model: choice.model,
  };
  return {
    // 桌面端会由 BackendSupervisor 重解析真实端口
    baseUrl: 'http://127.0.0.1:8080',
    apiKey: '',
    model: choice.model,
    provider,
    openaiConfig: {
      preset: choice.presetId,
      baseUrl: choice.baseUrl,
      apiKey: choice.apiKey,
      model: choice.model,
    },
    capabilities: casualCapabilities(autoTune),
  };
}
