// Optional native bridge to the Tauri shell.
//
// The desktop BackendSupervisor allocates an ephemeral port at launch, so the
// gateway address is only known at runtime. A persisted `baseUrl` from a prior
// session is therefore stale in packaged mode: it would point at a port nothing
// is listening on. The supervisor is authoritative, and this module is how the
// frontend asks it.
//
// Every entry point degrades to a null/false result outside Tauri, so the web
// build keeps working with its configured external gateway URL. Nothing here is
// imported at module scope by shared code — `@tauri-apps/api` is loaded through
// a dynamic import that is only reached when the Tauri global is present.

import type {CapabilityToggles} from './types';

/** Backend lifecycle states, mirroring the Rust `BackendState` enum. */
export type BackendState = 'Stopped' | 'Starting' | 'Ready' | 'Failed' | 'Stopping';

/** Who owns the backend process. The desktop never stops an external one. */
export type BackendOwnership = 'OwnedByDesktop' | 'External';

/**
 * Safe backend status, mirroring the Rust `BackendInfo`. Carries no secrets:
 * a Rust-side test asserts the serialized payload contains no credential
 * fields.
 */
export interface BackendStatus {
  state: BackendState;
  ownership: BackendOwnership;
  pid: number | null;
  endpoint: string | null;
  port: number | null;
  restart_count: number;
  last_exit_code: number | null;
  last_error: string | null;
  backend_version: string | null;
}

/**
 * Whether this build is running inside the Tauri shell.
 *
 * Checked via the injected global rather than by attempting an import, so the
 * web bundle never evaluates `@tauri-apps/api` at all.
 */
export function isDesktop(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

/**
 * Invoke a Tauri command, or return null when not running under Tauri.
 *
 * Failures are returned as null rather than thrown: a diagnostics probe must
 * never be able to break startup. The reason is logged for the console.
 */
async function invokeOptional<T>(command: string, args?: Record<string, unknown>): Promise<T | null> {
  if (!isDesktop()) return null;
  try {
    const {invoke} = await import('@tauri-apps/api/core');
    return (await invoke<T>(command, args)) ?? null;
  } catch (error) {
    console.warn(`[desktop-bridge] ${command} failed:`, error);
    return null;
  }
}

/** Current backend status, or null in web mode / on failure. */
export function getBackendStatus(): Promise<BackendStatus | null> {
  return invokeOptional<BackendStatus>('get_backend_status');
}

/** Ask the shell to restart the owned backend. */
export function restartBackend(): Promise<string | null> {
  return invokeOptional<string>('restart_backend');
}

/**
 * Provider environment injected into the sidecar, mirroring the Rust
 * `BackendProviderEnv`. Only the families the user actually configured are
 * populated; keys cross IPC in memory and are never persisted (Rust side
 * strips them before any disk write).
 */
export interface BackendProviderEnv {
  openai_api_key?: string;
  openai_url?: string;
  openai_models?: string;
  minimax_api_key?: string;
  minimax_url?: string;
  minimax_models?: string;
  anthropic_api_key?: string;
  anthropic_url?: string;
  anthropic_models?: string;
}

/**
 * Map the Settings-UI provider config onto the canonical sidecar env family:
 * anthropic protocol → APEIRETH_ANTHROPIC_*; MiniMax endpoints →
 * APEIRETH_API_*; everything else (OpenAI / DeepSeek / Ollama / custom) →
 * the OpenAI-compatible family (OPENAI_API_KEY + APEIRETH_OPENAI_*).
 */
export function backendProviderEnvFromConfig(config: {
  model?: string;
  provider?: {protocol?: string; preset?: string; baseUrl?: string; apiKey?: string; model?: string} | null;
}): BackendProviderEnv | null {
  const provider = config.provider;
  if (!provider) return null;
  const baseUrl = (provider.baseUrl || '').trim();
  const model = (provider.model || config.model || '').trim();
  const key = (provider.apiKey || '').trim();

  if (provider.protocol === 'anthropic' || baseUrl.includes('minimaxi.com')) {
    return {
      anthropic_api_key: key || undefined,
      anthropic_url: baseUrl || undefined,
      anthropic_models: model || undefined,
    };
  }
  if (provider.preset === 'minimax' || baseUrl.includes('minimax.chat')) {
    return {
      minimax_api_key: key || undefined,
      minimax_url: baseUrl || 'https://api.minimax.chat/v1',
      minimax_models: model || undefined,
    };
  }
  return {
    openai_api_key: key || undefined,
    openai_url: baseUrl || undefined,
    openai_models: model || undefined,
  };
}

/**
 * Push the provider environment to the shell. The supervisor stores it and
 * restarts the backend when it changed, so the caller must re-adopt the
 * endpoint. Returns the live endpoint (post-restart) or null in web mode /
 * on failure.
 */
export async function applyBackendProviderEnv(env: BackendProviderEnv): Promise<string | null> {
  const info = await invokeOptional<BackendStatus>('apply_backend_provider_env', {env});
  return info?.endpoint ?? null;
}

/**
 * Advanced-capability toggles, mirroring the Rust `BackendCapabilityEnv`.
 * Fail-closed: the supervisor injects `"1"` only for true values; absent
 * means OFF in the canonical CLI. Numeric knobs use 0 = "do not inject"
 * so the backend keeps its own default.
 *
 * 记忆核心族三件（preference_learning / proactive_recall / memory_injection）
 * 例外：CLI 侧默认开，所以开关必须双向显式注入（开 = "1"、关 = "0"），
 * 否则「关」会被 CLI 的默认开覆盖。缺省（未配）按默认开映射。
 *
 * 2026-10-10 W2/W3 收官批：补齐补漏落地的全部认知旋钮（partner_bond /
 * morphology / education / absorption / community / onering / onion /
 * worktree + 记忆流四件 + 议会数值旋钮）。
 */
export interface BackendCapabilityEnv {
  enable_shell: boolean;
  /** true → 注入 APEIRETH_SHELL_SANDBOX=0（显式裸跑）；false → 不注入（后端默认沙箱开）。 */
  shell_sandbox_off: boolean;
  enable_fetch: boolean;
  enable_local_read_tools: boolean;
  disable_typed_recall: boolean;
  enable_organs: boolean;
  enable_preference_learning: boolean;
  enable_proactive_recall: boolean;
  enable_memory_injection: boolean;
  enable_consolidation: boolean;
  enable_reflexion: boolean;
  enable_partner_bond: boolean;
  enable_morphology_recall: boolean;
  enable_education: boolean;
  enable_absorption_insight: boolean;
  enable_community_triage: boolean;
  enable_onering_ledger: boolean;
  cognitive_judge: boolean;
  cognitive_council: boolean;
  /** 1-7；0 = 不注入（后端默认 3）。 */
  council_advisors: number;
  /** 毫秒；0 = 不注入（后端默认 30000）。 */
  council_timeout_ms: number;
  /** 检索温度；0 = 不注入（后端默认 1.0）。 */
  morphology_temperature: number;
  enable_onion_layer: boolean;
  enable_worktree_sandbox: boolean;
  /** [Beta] 思考模式（reasoning_content 分流展示）；字符串字段空 = 不注入。 */
  reasoning_enabled: boolean;
  reasoning_model_filters: string;
  reasoning_tag: string;
  /** 性格养成体验旋钮：恒注入当前值（缺省/非法回基线，越界钳到 [min,max]），
   *  保持 JS number 由 Rust 侧格式化。 */
  tune_memory_fade: number;
  tune_curiosity_strength: number;
  tune_tone_saturation: number;
  /** 整合节奏·每 N 回合；≥1 整数（1 = 每回合 = 现行为）。 */
  tune_consolidation_cadence: number;
  /** 「从使用中学习」：仅 true 注入 "1"（后端 "1" 才开，默认关，fail-closed）。 */
  enable_self_tuning: boolean;
}

/** Map the config's capability toggles onto the canonical knob names.
 *  记忆核心族三件缺省 = 开（`!== false`）：未配 capabilities 时不把默认开的
 *  核心记忆误映射成显式关。 */
export function capabilityEnvFromConfig(toggles: CapabilityToggles | undefined | null): BackendCapabilityEnv {
  const advisors = Math.min(7, Math.max(1, Math.round(toggles?.councilAdvisors ?? 3)));
  const timeout = Math.max(1000, Math.round(toggles?.councilTimeoutMs ?? 30000));
  const temperature = toggles?.morphologyTemperature ?? 1.0;
  /** 性格养成体验旋钮：缺省/非有限回基线，越界钳到 [min,max]（整数取整在调用侧）。 */
  const tuneKnob = (v: unknown, baseline: number, min: number, max: number): number => {
    const n = typeof v === 'number' && Number.isFinite(v) ? v : baseline;
    return Math.min(max, Math.max(min, n));
  };
  return {
    enable_shell: toggles?.shell === true,
    // 反向语义：UI 默认开沙箱；显式关沙箱才注入 =0（fail-closed）。
    shell_sandbox_off: toggles?.shellSandbox === false,
    enable_fetch: toggles?.fetch === true,
    // 本地只读三件套同默认开语义：缺省 = 开、显式 false = 关（注入侧双向显式）。
    enable_local_read_tools: toggles?.localReadTools !== false,
    disable_typed_recall: toggles?.typedRecall === false,
    enable_organs: toggles?.organs === true,
    // 记忆核心族：缺省 = 开、显式 false = 关（注入侧由 Rust 显式发 1/0）。
    enable_preference_learning: toggles?.preferenceLearning !== false,
    enable_proactive_recall: toggles?.proactiveRecall !== false,
    enable_memory_injection: toggles?.memoryInjection !== false,
    enable_consolidation: toggles?.consolidation === true,
    enable_reflexion: toggles?.reflexion === true,
    enable_partner_bond: toggles?.partnerBond === true,
    enable_morphology_recall: toggles?.morphologyRecall === true,
    enable_education: toggles?.education === true,
    enable_absorption_insight: toggles?.absorptionInsight === true,
    enable_community_triage: toggles?.communityTriage === true,
    enable_onering_ledger: toggles?.oneringLedger === true,
    cognitive_judge: toggles?.judge === true,
    cognitive_council: toggles?.council === true,
    council_advisors: toggles?.council === true ? advisors : 0,
    council_timeout_ms: toggles?.council === true ? timeout : 0,
    morphology_temperature: toggles?.morphologyRecall === true && temperature > 0 ? temperature : 0,
    enable_onion_layer: toggles?.onionLayer === true,
    enable_worktree_sandbox: toggles?.worktreeSandbox === true,
    reasoning_enabled: toggles?.reasoningEnabled === true,
    reasoning_model_filters: toggles?.reasoningEnabled === true ? (toggles?.reasoningModelFilters ?? '').trim() : '',
    reasoning_tag: toggles?.reasoningEnabled === true ? (toggles?.reasoningTag ?? 'think').trim() : '',
    // 性格养成体验旋钮：四个数值恒发（当前值），「从使用中学习」fail-closed。
    tune_memory_fade: tuneKnob(toggles?.memoryFade, 1.0, 0.25, 4.0),
    tune_curiosity_strength: tuneKnob(toggles?.curiosityStrength, 1.0, 0.25, 4.0),
    tune_tone_saturation: tuneKnob(toggles?.toneSaturation, 1.0, 0.0, 2.0),
    tune_consolidation_cadence: Math.round(tuneKnob(toggles?.consolidationCadence, 1, 1, 10)),
    enable_self_tuning: toggles?.selfTuning === true,
  };
}

/**
 * Apply provider env + capability toggles in one IPC call so a settings save
 * that changed both restarts the backend exactly once. Either part may be
 * null (no-op). Returns the live endpoint or null in web mode / on failure.
 */
export async function applyBackendConfig(
  provider: BackendProviderEnv | null,
  capabilities: BackendCapabilityEnv | null,
): Promise<string | null> {
  const info = await invokeOptional<BackendStatus>('apply_backend_config', {provider, capabilities});
  return info?.endpoint ?? null;
}

/** Absolute path of the log directory. */
export function getLogDirectory(): Promise<string | null> {
  return invokeOptional<string>('get_log_directory');
}

/**
 * 「从使用中学习」自动调整记录（只读）：后端写入数据目录的 tuning-log.jsonl，
 * 此处只读展示。param 取值：memory_fade / curiosity_strength /
 * tone_saturation / consolidation_cadence。非桌面环境或读取失败返回 null，
 * 调用侧据此诚实标注「仅桌面版可读」，不伪造空记录。
 */
export interface TuningLogEntry {
  seq: number;
  param: string;
  previous: number;
  next: number;
  reason: string;
  at_epoch_ms: number;
}

/** 读取学习日志（最新在后）；null = 非桌面版 / 读取失败。 */
export function readTuningLog(): Promise<TuningLogEntry[] | null> {
  return invokeOptional<TuningLogEntry[]>('read_tuning_log');
}

/** Reveal the log directory in the platform file manager. */
export async function openLogDirectory(): Promise<boolean> {
  if (!isDesktop()) return false;
  try {
    const {invoke} = await import('@tauri-apps/api/core');
    await invoke('open_log_directory');
    return true;
  } catch (error) {
    console.warn('[desktop-bridge] open_log_directory failed:', error);
    return false;
  }
}

/**
 * Resolve the gateway base URL the frontend should actually use.
 *
 * In packaged desktop mode the supervisor's endpoint wins over any persisted
 * value, because the port is chosen fresh at each launch. `fallback` is
 * returned in web mode, before the backend is Ready, or if the probe fails —
 * so this is always safe to await during startup.
 *
 * Polls briefly rather than resolving once: `start()` runs concurrently with
 * the webview, so the first probe often lands while the state is still
 * `Starting` and no endpoint is published yet.
 */
export async function resolveBackendEndpoint(
  fallback: string,
  options: {attempts?: number; delayMs?: number} = {},
): Promise<string> {
  if (!isDesktop()) return fallback;

  const attempts = options.attempts ?? 20;
  const delayMs = options.delayMs ?? 400;

  for (let attempt = 0; attempt < attempts; attempt += 1) {
    const status = await getBackendStatus();

    // A published endpoint is authoritative as soon as it exists.
    if (status?.endpoint) return status.endpoint;

    // Terminal states will not produce an endpoint; stop early and let the
    // caller surface the failure rather than stalling startup.
    if (status && (status.state === 'Failed' || status.state === 'Stopped')) break;

    // No bridge at all (null) means nothing more to wait for.
    if (!status) break;

    if (attempt < attempts - 1) {
      await new Promise((resolve) => setTimeout(resolve, delayMs));
    }
  }

  return fallback;
}
