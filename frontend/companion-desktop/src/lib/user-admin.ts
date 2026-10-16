// 用户/域管理面板逻辑面（V1-b/d 上屏，D10「用户/域管理 UI 待上屏」清偿）。
//
// 契约面（docs/02-guides/frontend-data-contract.md）：
//   §10.6 UserAdminRow —— GET /v1/panel/users 读面 + PATCH/DELETE /v1/admin/users/:id 写面；
//   §10.7 DomainAdminRow —— GET /v1/panel/domains 读面；
//   §10.3 self_status 域节形状 ——「当前域节」按此同形渲染（counts 缺探测口显式 null + reason）。
//
// 分工（战场约束）：只读 fetch 面在 runtime.ts（fetchUsers / fetchDomains）；
// 本文件 = 纯投影 / 只读态判定 / CLI 指引（命令可复制）/ 管理写面请求执行
// （注入传输缝，零 UI、零全局态，node 测试直跑）。
//
// 双入口同源：面板管理写面与 `apeireth user` CLI 改的是同一张用户表，CLI 指引
// 常驻可复制；角色面不可用（501）/ 无权限（403）/ 网络失败 → 只读态 + CLI 指引，
// 不假造管理能力（零假旋钮）。

/** 角色词表（W3-② 冻结：owner / member / guest，面板与 CLI 同源）。 */
export const USER_ROLES = ['owner', 'member', 'guest'] as const;
export type UserRole = (typeof USER_ROLES)[number];

/** 用户管理行（契约 §10.6）。 */
export interface UserAdminRow {
  user_id: string;
  role: string;
  /** 域引用（W3-① 域面），无域时缺省。 */
  domain?: string;
  client_names: string[];
  client_count: number;
}

/** 域管理行（契约 §10.7）。 */
export interface DomainAdminRow {
  user_id: string;
  display_name: string;
  created_at: number;
  role: string;
  user_count?: number;
}

/** 域节计数（契约 §10.3：缺探测口的维显式 null + counts.reason）。 */
export interface DomainSectionCounts {
  sessions: number | null;
  memories: number | null;
  protected: number | null;
  reason: string | null;
}

/** 当前域节视图 = self_status 域节合同形状（契约 §10.3 同形消费）。 */
export interface DomainSectionView {
  user_id: string;
  display_name: string;
  role: string;
  created_at: number;
  counts: DomainSectionCounts;
}

/** 计数缺源原因（如实口径：面板读面无域内计数探测口）。 */
export const DOMAIN_COUNTS_NO_PROBE =
  '面板读面无域内计数探测口（sessions/memories/protected 仅 self_status 工具面自述）';

/** 用户管理面错误帧（{error:{code,message,solution}}，契约 §10.6 错误模型）。 */
export interface UserAdminErrorInfo {
  error: string;
  status: number;
  code: string;
}

export type UserAdminMutationResult = {ok: true; user: UserAdminRow} | ({ok: false} & UserAdminErrorInfo);

/** 管理写面注入传输缝（与 metrics.ts 同法：真传输由调用侧给，测试给记录桩）。 */
export interface UserAdminFetchDeps {
  authHeaders: () => Promise<Record<string, string>> | Record<string, string>;
  fetchJson: (
    url: string,
    init?: {method?: string; headers?: Record<string, string>; body?: string},
  ) => Promise<{ok: boolean; status: number; json: () => Promise<unknown>}>;
}

/** 严格归一用户行：形状不符的行直接丢（不编默认值冒充真值）。 */
export function normalizeUserRows(raw: unknown): UserAdminRow[] {
  if (!Array.isArray(raw)) return [];
  const rows: UserAdminRow[] = [];
  for (const item of raw) {
    if (!item || typeof item !== 'object') continue;
    const rec = item as Record<string, unknown>;
    const user_id = typeof rec.user_id === 'string' ? rec.user_id : '';
    const role = typeof rec.role === 'string' ? rec.role : '';
    if (user_id === '' || role === '') continue;
    const client_names = Array.isArray(rec.client_names)
      ? rec.client_names.filter((n): n is string => typeof n === 'string')
      : [];
    // client_count 缺席时用实得名单长度（实数，不是编造的 0）。
    const client_count =
      typeof rec.client_count === 'number' && Number.isFinite(rec.client_count)
        ? rec.client_count
        : client_names.length;
    const domain = typeof rec.domain === 'string' && rec.domain !== '' ? rec.domain : undefined;
    rows.push({user_id, role, domain, client_names, client_count});
  }
  return rows;
}

/** 严格归一域行（契约 §10.7）。 */
export function normalizeDomainRows(raw: unknown): DomainAdminRow[] {
  if (!Array.isArray(raw)) return [];
  const rows: DomainAdminRow[] = [];
  for (const item of raw) {
    if (!item || typeof item !== 'object') continue;
    const rec = item as Record<string, unknown>;
    const user_id = typeof rec.user_id === 'string' ? rec.user_id : '';
    const display_name = typeof rec.display_name === 'string' ? rec.display_name : '';
    if (user_id === '' || display_name === '') continue;
    const created_at = typeof rec.created_at === 'number' && Number.isFinite(rec.created_at) ? rec.created_at : 0;
    const role = typeof rec.role === 'string' ? rec.role : '';
    const user_count =
      typeof rec.user_count === 'number' && Number.isFinite(rec.user_count) ? rec.user_count : undefined;
    rows.push({user_id, display_name, created_at, role, user_count});
  }
  return rows;
}

/** 住户列表投影（卡片列表列：id / 角色 / 客户端数）。 */
export interface UserAdminListRow {
  id: string;
  role: string;
  clientCount: number;
  clientNames: string[];
  domain: string;
}

export interface UserAdminListView {
  rows: UserAdminListRow[];
  count: number;
}

export function userAdminView(rows: UserAdminRow[]): UserAdminListView {
  return {
    rows: rows.map((r) => ({
      id: r.user_id,
      role: r.role,
      clientCount: r.client_count,
      clientNames: r.client_names.slice(),
      domain: r.domain ?? '',
    })),
    count: rows.length,
  };
}

/** 住户卡工作模式：manage（读面可用=调用方具备管理视角）| read-only（只读态）。 */
export interface UserPanelMode {
  mode: 'manage' | 'read-only';
  reason: string;
}

export function userPanelMode(outcome: {ok: boolean; status?: number; code?: string; error?: string}): UserPanelMode {
  if (outcome.ok) return {mode: 'manage', reason: ''};
  const status = outcome.status ?? 0;
  if (status === 501 || outcome.code === 'no_role_face') {
    return {mode: 'read-only', reason: '角色面未就绪（no_role_face），住户卡只读，引导走 CLI 建首用户'};
  }
  if (status === 403 || outcome.code === 'permission_denied') {
    return {mode: 'read-only', reason: '无管理权限（permission_denied），住户卡只读展示'};
  }
  return {
    mode: 'read-only',
    reason: `住户列表读面不可用（${outcome.error || `HTTP ${status}`}），住户卡只读展示`,
  };
}

/**
 * CLI 指引（命令可复制）：面板管理写面与 `apeireth user` CLI 同源改同一张用户表。
 * 生成的是具体可运行命令（角色/ID 就位），不是占位模板。
 */
export function userCliGuidance(userId: string, role: string): string[] {
  return [
    `apeireth user role --user-id ${userId} --role ${role}`,
    `apeireth user remove --user-id ${userId}`,
    `apeireth user list`,
  ];
}

/** CLI 引导入口（用户表为空时 `user add` 是引导命令，与 CLI 用法同源）。 */
export function userCliBootstrap(userId: string): string {
  return `apeireth user add --user-id ${userId} --role owner`;
}

/** 错误帧归一（{error:{code,message,solution}} 与裸 message 都收）。 */
function errorInfo(data: unknown, status: number, fallback: string): UserAdminErrorInfo {
  const frame = (data as {error?: {code?: string; message?: string}} | null)?.error;
  return {
    error: frame?.message || (data as {message?: string} | null)?.message || fallback || `HTTP ${status}`,
    status,
    code: frame?.code || (status === 0 ? 'network_error' : 'http_error'),
  };
}

/** owner 视角角色调整：PATCH /v1/admin/users/:user_id {role}（契约 §10.6）。 */
export async function patchUserRole(
  baseUrl: string,
  userId: string,
  role: string,
  deps: UserAdminFetchDeps,
): Promise<UserAdminMutationResult> {
  if (!(USER_ROLES as readonly string[]).includes(role)) {
    return {ok: false, error: `role 需为 ${USER_ROLES.join(' | ')} 之一`, status: 0, code: 'invalid_role'};
  }
  const base = baseUrl.replace(/\/+$/, '');
  try {
    const res = await deps.fetchJson(`${base}/v1/admin/users/${encodeURIComponent(userId)}`, {
      method: 'PATCH',
      headers: {'Content-Type': 'application/json', ...(await deps.authHeaders())},
      body: JSON.stringify({role}),
    });
    const data = await res.json().catch(() => null);
    if (!res.ok) return {ok: false, ...errorInfo(data, res.status, '角色调整被拒')};
    const user = normalizeUserRows([(data as {user?: unknown} | null)?.user])[0];
    if (!user) return {ok: false, ...errorInfo(data, res.status, '角色调整响应形状不符（缺 user 行）')};
    return {ok: true, user};
  } catch (caught) {
    return {
      ok: false,
      error: caught instanceof Error ? caught.message : String(caught),
      status: 0,
      code: 'network_error',
    };
  }
}

/** owner 视角移除住户：DELETE /v1/admin/users/:user_id（契约 §10.6）。 */
export async function removeUser(
  baseUrl: string,
  userId: string,
  deps: UserAdminFetchDeps,
): Promise<UserAdminMutationResult> {
  const base = baseUrl.replace(/\/+$/, '');
  try {
    const res = await deps.fetchJson(`${base}/v1/admin/users/${encodeURIComponent(userId)}`, {
      method: 'DELETE',
      headers: {'Content-Type': 'application/json', ...(await deps.authHeaders())},
    });
    const data = await res.json().catch(() => null);
    if (!res.ok) return {ok: false, ...errorInfo(data, res.status, '移除住户被拒')};
    return {
      ok: true,
      user: {user_id: userId, role: '', client_names: [], client_count: 0},
    };
  } catch (caught) {
    return {
      ok: false,
      error: caught instanceof Error ? caught.message : String(caught),
      status: 0,
      code: 'network_error',
    };
  }
}

/**
 * 当前域节：域行 → self_status 域节合同形状（契约 §10.3）。
 * counts 三维全部显式 null + reason：面板读面（/v1/panel/domains）无域内计数探测口，
 * 那三维只在 self_status 工具面自述——不拿 0 冒充真值计数。
 */
export function domainSectionView(row: DomainAdminRow): DomainSectionView {
  return {
    user_id: row.user_id,
    display_name: row.display_name,
    role: row.role,
    created_at: row.created_at,
    counts: {
      sessions: null,
      memories: null,
      protected: null,
      reason: DOMAIN_COUNTS_NO_PROBE,
    },
  };
}
