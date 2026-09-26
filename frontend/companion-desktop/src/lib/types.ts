// Apeireth 桌面伙伴 — 核心共享类型定义 (Svelte 5 + Tauri 2)

export type ViewId = 'chat' | 'conversations' | 'activity' | 'tools' | 'memory' | 'settings';
// 2026-09-23 主人指示「最后我们都是要做的」：day/ocean/forest/paper 四主题
// 当日曾作为空心项删除（无实现点了无反应），现在真实现补全（令牌+背景见
// tokens.css/base.css/shell.css），从空心回归现役。
export type Theme = 'heritage-void' | 'essence' | 'night' | 'day' | 'ocean' | 'forest' | 'paper';

/** UI 配色方案 id（规范 §8 增补⑤）：只染 UI 高亮，不占存在金 */
export type Accent = 'presence-gold' | 'deep-space' | 'sage' | 'bone';
export type MemoryCategory =
  | '工作记忆'
  | '近期记忆'
  | '长期记忆'
  | '用户画像'
  | '知识'
  | '事实'
  | '偏好'
  | '事件'
  | '反馈'
  | '参考';

export type ConversationScope = 'global' | 'project';

export interface ToolCallDetails {
  id: string;
  name: string;
  args?: unknown;
  rawArgs?: string;
  status: 'pending' | 'running' | 'succeeded' | 'failed' | 'cancelled';
  resultSummary?: string;
  resultFull?: string;
  error?: string;
  durationMs?: number;
  startTime?: number;
  endTime?: number;
}

export interface ChatMessageEvent {
  id: string;
  kind: 'status' | 'tool' | 'task' | 'mcp' | 'memory' | 'agent' | 'error' | string;
  text: string;
  ts?: number;
  status?: 'pending' | 'running' | 'done' | 'failed' | 'skipped' | 'awaiting_approval' | string;
  action?: string;
  /** 工具风险等级 (T1-T3) */
  tier?: number;
  receipt?: string;
  taskId?: string;
  stepId?: string;
  toolCall?: ToolCallDetails;
}

export interface TaskCardInfo {
  taskId: string;
  title: string;
  status: string;
  detail?: string;
}

export interface ApprovalRequest {
  id: string;
  chain: string;
  rev: number;
  tool: string;
  args_preview: string;
  reason: string;
  status: 'pending' | 'approved' | 'expired';
  created_at: number;
  updated_at: number;
}

export interface ChatMessage {
  id: string;
  role: 'user' | 'assistant' | 'system';
  text: string;
  time: string;
  timestamp?: number;
  proactive?: string;
  events?: ChatMessageEvent[];
  error?: string;
  streaming?: boolean;
  aborted?: boolean;
  reasoning?: string;
  reasoningDurationMs?: number;
  provenance?: {
    count?: number;
    memories?: string[];
  };
  taskCard?: TaskCardInfo;
  toolCalls?: ToolCallDetails[];
  modelInfo?: {
    id: string;
    provider?: string;
  };
}

export interface Conversation {
  id: string;
  title: string;
  createdAt: number;
  updatedAt: number;
  messages: ChatMessage[];
  archived?: boolean;
  pinned?: boolean;
  scope: ConversationScope;
  projectId?: string;
  model?: string;
  /** 会话级工作区根目录; 未设时跟随全局默认 (设置页). 项目分组键. */
  workspace?: string;
  /** 创建会话时的伙伴人设（联系人分组键；name 冗余存, 人设删了组标签仍在）。 */
  personaId?: string;
  personaName?: string;
}

export interface ModelSetup {
  baseUrl: string;
  apiKey: string;
  model: string;
}

export interface SubsystemStatus {
  name: string;
  key: 'api' | 'companion' | 'memory' | 'tools' | 'events' | 'sessions';
  status: 'ok' | 'degraded' | 'offline' | 'unknown';
  endpoint: string;
  detail?: string;
  latencyMs?: number;
}

export interface RuntimeHealthReport {
  overall: 'connecting' | 'online' | 'degraded' | 'offline' | 'error';
  baseUrl: string;
  latencyMs?: number;
  lastChecked?: number;
  subsystems: SubsystemStatus[];
  model: string;
  error?: string;
}

export type HealthState = 'connecting' | 'online' | 'ready' | 'degraded' | 'generating' | 'error' | 'offline';

// ============================================================
// Runtime Capability Manifest — 后端能力发现契约
// Desktop 据此 gate UI 按钮, 不再 404-probing. 这是 information 不是 authorization.
// 未知字段保留 (forward compat); 未知 capability id 一律视为 unsupported.
// ============================================================

/** 单条能力声明 (稳定能力 ID 如 sessions.create). */
export interface Capability {
  id: string;
  supported: boolean;
  read?: boolean;
  write?: boolean;
  version?: number;
  operations?: string[];
  /**
   * 该能力**此时此刻**是否真正可调用 (动态, 受 provider/凭据/平台影响).
   * 向后兼容: 旧 manifest 无此字段 → 客户端按 available = supported 解释
   * (见 runtime.ts capabilityAvailable).
   */
  available?: boolean;
  /** 不可用原因 (machine-readable, 仅 available === false 时存在). */
  reason?: CapabilityAvailabilityReason;
  /** Explicit compatibility alias of another canonical capability id. */
  alias_of?: string;
}

/** Capability 不可用的 machine-readable 原因 (镜像 Rust AvailabilityReason). */
export type CapabilityAvailabilityReason =
  | 'provider_not_configured'
  | 'provider_unavailable'
  | 'platform_unsupported'
  | 'disabled_by_policy'
  | 'not_implemented'
  | 'not_exposed';

/** 一个能力组 (如 sessions / memory / permissions / trace). */
export interface CapabilityGroup {
  name: string;
  capabilities: Capability[];
}

/** Runtime 元信息 (仅 public 信息, 绝不含 secret/路径). */
export interface RuntimeInfo {
  service: string;
  version: string;
}

/** Capability Manifest — runtime 能力契约. */
export interface CapabilityManifest {
  schema_version: number;
  runtime: RuntimeInfo;
  capabilities: CapabilityGroup[];
  /** 是否为 legacy 兼容 profile (runtime 无原生 manifest 端点时客户端构造的保守声明). */
  legacy?: boolean;
}

export type ProviderProtocol = 'openai' | 'anthropic';

export interface ProviderConfig {
  protocol: ProviderProtocol;
  preset: string;
  baseUrl: string;
  apiKey: string;
  model: string;
  anthropicVersion?: string;
  /** Explicit diagnostics-only escape hatch; normal chat always uses Gateway. */
  debugDirect?: boolean;
}

/**
 * 伙伴人设 (Agent Persona) — 数据驱动身份, 可在设置里随时增删改, 无需重编译。
 * persona 文本作为 system 消息注入每次对话请求 (runtime.ts run() 统一注入)。
 */
export interface PersonaProfile {
  id: string;
  /** 显示名, 如 "阿佩瑞斯" */
  name: string;
  /** system 人设文本 (可为空 = 不注入) */
  persona: string;
  /** 可选: 该 Agent 固定使用的模型 (切换人设时同步切换模型) */
  model?: string;
}

/** 后端高级能力开关（注入侧车环境）。记忆核心族三件（preferenceLearning /
 *  proactiveRecall / memoryInjection）默认开，显式关时注入 `APEIRETH_ENABLE_*=0`；
 *  其余默认关 fail-closed。2026-10-10 W2/W3 收官批扩展：补齐 v1→v2 补漏落地的
 *  全部认知旋钮，与 crates/adapters/cli 的 APEIRETH_* env 一一对应。 */
export interface CapabilityToggles {
  /** 工具: shell 命令（开启后每次调用仍走人工审批） */
  shell: boolean;
  /** 工具: shell 沙箱执行（AppContainer 断网 + 用户空间隔离；默认开，关闭 = 显式裸跑自担风险） */
  shellSandbox: boolean;
  /** 工具: 公网 GET-only fetch */
  fetch: boolean;
  /** 工具: 本地只读工具（file/search/repo 不经审批的读侧） */
  localReadTools: boolean;
  /** AfterTurn 器官链（9 organs） */
  organs: boolean;
  /** 偏好学习双索引写回 */
  preferenceLearning: boolean;
  /** 前瞻召回（闲置期主动浮现相关记忆） */
  proactiveRecall: boolean;
  /** 记忆注入（把召回内容注进上下文） */
  memoryInjection: boolean;
  /** 记忆固化（consolidation 提炼） */
  consolidation: boolean;
  /** 反思沉淀（reflexion 文件回流） */
  reflexion: boolean;
  /** 类型化召回（默认开；关闭 = 注入 APEIRETH_DISABLE_TYPED_RECALL） */
  typedRecall: boolean;
  /** 伙伴羁绊（TurnStart 关系状态注入 + 确定性演化，W2 §4.2） */
  partnerBond: boolean;
  /** 检索深度自适应 morphology（只收紧不放大，W2 §4.3） */
  morphologyRecall: boolean;
  /** morphology 温度（检索活跃度，默认 1.0） */
  morphologyTemperature: number;
  /** Dx-Check 教育工具（W2 §4.3） */
  education: boolean;
  /** 认知体操四算法洞察（betti/residual/river/kuramoto，W2 §4.4） */
  absorptionInsight: boolean;
  /** 图社区分诊（检索前置 Entity/Broad 路由，W3 §1） */
  communityTriage: boolean;
  /** onering 账本记账（回合留痕入 context_ledger，W3） */
  oneringLedger: boolean;
  /** AfterModelResponse 评审 */
  judge: boolean;
  /** AfterModelResponse 议会 */
  council: boolean;
  /** 议会顾问数 1-7（默认 3，Safety 恒首位） */
  councilAdvisors: number;
  /** 单顾问超时毫秒（默认 30000） */
  councilTimeoutMs: number;
  /** 三洋葱治理层 L3-L5 三段门（默认关，W3 工作项 6） */
  onionLayer: boolean;
  /** 子代理 git worktree 隔离（默认关） */
  worktreeSandbox: boolean;
  /** [Beta] 思考模式：reasoning_content 单独分流展示（默认关） */
  reasoningEnabled: boolean;
  /** [Beta] 思考模式生效的模型过滤器（逗号分隔子串，空 = 全部模型） */
  reasoningModelFilters: string;
  /** [Beta] reasoning 标签名（默认 think） */
  reasoningTag: string;
  /** 遗忘衰减强度倍率（有效遗忘半衰期 = 24h / 值；默认 1.0 = 现行为） */
  memoryFade: number;
  /** 好奇心强度倍率（好奇日预算 = 2000 × 值；默认 1.0） */
  curiosityStrength: number;
  /** 语气情绪饱和度倍率（情绪注入混合 × 值；0 = 纯关系基线；默认 1.0） */
  toneSaturation: number;
  /** 整合节奏·每 N 回合触发一次记忆整合（1 = 每回合 = 现行为；默认 1） */
  consolidationCadence: number;
  /** 从使用中学习（自学习自动微调体验参数；默认关） */
  selfTuning: boolean;
}

export const DEFAULT_CAPABILITY_TOGGLES: CapabilityToggles = {
  shell: false,
  shellSandbox: true,
  fetch: false,
  localReadTools: true,
  organs: false,
  preferenceLearning: true,
  proactiveRecall: true,
  memoryInjection: true,
  consolidation: false,
  reflexion: false,
  typedRecall: true,
  partnerBond: false,
  morphologyRecall: false,
  morphologyTemperature: 1.0,
  education: false,
  absorptionInsight: false,
  communityTriage: false,
  oneringLedger: false,
  judge: false,
  council: false,
  councilAdvisors: 3,
  councilTimeoutMs: 30000,
  onionLayer: false,
  worktreeSandbox: false,
  reasoningEnabled: false,
  reasoningModelFilters: '',
  reasoningTag: 'think',
  memoryFade: 1.0,
  curiosityStrength: 1.0,
  toneSaturation: 1.0,
  consolidationCadence: 1,
  selfTuning: false,
};

/**
 * 「推荐配置」一键预设开启的能力键（与 SettingsView 能力中心按钮共用）。
 * 记忆核心族三件 + 记忆固化/反思沉淀/器官链；**绝不包含** shell / fetch
 * （危险能力不进任何预设）。键名到后端 env 的映射见 SettingsView 的
 * 能力注册表（每行标注 env 芯片），测试镜像校验两侧一致。
 */
export type BooleanCapabilityKey = {
  [K in keyof CapabilityToggles]: CapabilityToggles[K] extends boolean ? K : never;
}[keyof CapabilityToggles];

export const RECOMMENDED_CAPABILITY_PRESET: ReadonlyArray<BooleanCapabilityKey> = [
  'proactiveRecall',
  'preferenceLearning',
  'memoryInjection',
  'consolidation',
  'reflexion',
  'organs',
];

export interface ApeirethConfig {
  baseUrl: string;
  apiKey: string;
  model: string;
  theme?: Theme;
  provider?: ProviderConfig;
  openaiConfig?: {
    preset?: string;
    baseUrl: string;
    apiKey: string;
    model: string;
  };
  anthropicConfig?: {
    preset?: string;
    baseUrl: string;
    apiKey: string;
    model: string;
    anthropicVersion?: string;
  };
  /** 后端高级能力开关（持久化于本地配置，不含 secret） */
  capabilities?: CapabilityToggles;
  /** 多 Agent 人设列表 (持久化于本地配置, 不含 secret) */
  personas?: PersonaProfile[];
  /** 当前激活人设 id (缺省时取列表第一个) */
  activePersonaId?: string;
  /** 自定义上传背景开关（规范 §8 增补④）：true = 用 IndexedDB 里的上传图
   *  盖住主题默认背景；图本体在 IndexedDB（bg-store.ts），不落 config。 */
  customBg?: boolean;
  /** UI 配色方案（规范 §8 增补⑤）：缺省 = 存在金（视觉现状不变） */
  accent?: Accent;
}

export interface ActivityItem {
  id: string;
  timestamp: number;
  category: 'conversation' | 'agent' | 'tool' | 'memory' | 'workflow' | 'runtime' | 'error';
  title: string;
  summary: string;
  source: 'sse' | 'audit' | 'runtime' | 'local';
  severity: 'info' | 'success' | 'warning' | 'error';
  detail?: string;
  raw?: unknown;
  /** Phase 5: 关联的 trace_id (SSE trace 事件携带). */
  traceId?: string;
}

export interface MemoryEpisodeItem {
  id: string;
  timestamp: number;
  role: string;
  content: string;
  sessionId: string;
  category?: string;
  stream?: string;
  importance?: number;
  tags?: string[];
  /** Phase 3 governance: 是否受保护 (防自动遗忘). */
  protected?: boolean;
  /** Phase 3 governance: active / forgotten. */
  status?: 'active' | 'forgotten';
}

export interface ToolItem {
  name: string;
  description?: string;
  argsSchema?: unknown;
  source?: 'builtin' | 'mcp' | 'extension';
  permission?: 'none' | 'prompt' | 'granted' | 'restricted';
  lastUsed?: number;
  available: boolean;
}

export interface ApprovalRequestItem {
  id: string;
  chain?: string;
  rev?: number;
  tool: string;
  reason?: string;
  args_preview?: string;
  summary?: string;
  requestedAt?: number;
  params?: unknown;
  command_text?: string;
  arguments_summary?: string;
  status: 'pending' | 'approved' | 'expired' | 'rejected';
}


export function categoryToWire(category: MemoryCategory | string): string {
  const map: Record<string, string> = {
    工作记忆: 'working',
    近期记忆: 'recent',
    长期记忆: 'long_term',
    用户画像: 'profile',
    知识: 'knowledge',
    事实: 'fact',
    偏好: 'preference',
    事件: 'event',
    反馈: 'feedback',
    参考: 'reference',
  };
  return map[category] || category;
}

export function categoryFromWire(wire: string): MemoryCategory {
  const map: Record<string, MemoryCategory> = {
    working: '工作记忆',
    recent: '近期记忆',
    long_term: '长期记忆',
    profile: '用户画像',
    knowledge: '知识',
    fact: '长期记忆',
    preference: '用户画像',
    event: '近期记忆',
    feedback: '近期记忆',
    reference: '知识',
  };
  return map[wire] || '长期记忆';
}

export function importanceStars(value: number): 1 | 2 | 3 {
  if (value >= 0.75) return 3;
  if (value >= 0.4) return 2;
  return 1;
}

// ============================================================
// Wave-1 前端数据层契约 (与后端并行代理严格一致)
// ============================================================

/** 模型列表条目 (GET /v1/models 的 data[] 投影). */
export interface ModelInfo {
  id: string;
  ownedBy?: string;
  description?: string;
}

/** 后端错误帧 machine-readable 错误码. */
export type ErrorCode =
  | 'auth_missing_key'
  | 'auth_invalid_key'
  | 'provider_unreachable'
  | 'provider_error'
  | 'invalid_request'
  | 'session_not_found'
  | 'rate_limited'
  | 'review_rejected'
  | 'turn_not_converged'
  | 'internal';

/** 后端错误帧 (错误响应体 / SSE error 帧的规范形状). */
export interface ApiErrorFrame {
  message: string;
  code?: ErrorCode;
  solution?: string;
}

/** 会话级设置 (GET/PATCH /v1/sessions/{id}/settings). */
export interface SessionSettings {
  model: string | null;
  permission_preset: 'read_only' | 'standard' | 'full';
  /** 审批是否在本会话内记住 (backend 契约, 默认 false). */
  approval_remember?: boolean;
}

/** 管理配置 patch (POST /v1/admin/config). */
export interface AdminConfigPatch {
  provider?: string;
  base_url?: string;
  api_key?: string;
  model?: string;
  capabilities?: Record<string, boolean>;
}

export interface GuardStatus {
  enabled: boolean;
  fast_guard_active: boolean;
  chain_guard_active: boolean;
  /** introspection.rs serde(default) 字段：新后端恒序列化，旧后端可能缺省。 */
  intent_guard_active?: boolean;
  cross_turn_monitoring_active?: boolean;
  active_chains: number;
  total_evaluations: number;
  total_allowed: number;
  total_denied: number;
  total_approval_required: number;
  dataset_recording_enabled: boolean;
  ml_classifier_available?: boolean;
  ml_model_version?: string | null;
}

export interface GuardEvent {
  timestamp_ms: number;
  session_id: string;
  trace_id: string;
  round: number;
  capability_id: string;
  stage: 'fast_guard' | 'chain_guard' | 'decision_fusion';
  decision: string;
  risk_score: number;
  reasons: string[];
  evidence: string[];
}

export interface GuardDryRunRequest {
  session_id?: string;
  capability_id: string;
  arguments: unknown;
  declared_scope?: string;
}

export interface GuardDryRunResponse {
  decision: string;
  stage: 'fast_guard' | 'chain_guard' | 'decision_fusion';
  risk_score: number;
  reasons: string[];
  evidence: string[];
}

export interface WorkbenchToolExecution {
  id: string;
  name: string;
  status: string;
  latency_ms?: number;
  error?: string;
}

export interface WorkbenchMemoryProvenance {
  recalled_count: number;
  governance_filtered: number;
  layers: string[];
}

export interface WorkbenchTurn {
  session_id: string;
  goal: string;
  agent_status: string;
  tools: WorkbenchToolExecution[];
  memory: WorkbenchMemoryProvenance;
  guard_verdict?: string;
  updated_at: number;
}
