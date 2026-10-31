// Typed IPC bridge for the desktop-side P0/P1 commands (wave-2 UI contract).
//
// This file only *adds* wrappers; it does not modify runtime.ts or the existing
// desktop-bridge.ts. Every wrapper mirrors the Rust commands registered in
// `src-tauri/src/lib.rs` (keychain.rs / workspace.rs / backend_supervisor.rs).
//
// The gateway token face (resolveGatewayToken / gatewayAuthHeaders) also lives
// here: it composes the IPC token with the web-build env injection and hands
// callers one auth-header seam for every gateway request.
//
// Like desktop-bridge.ts, it loads `@tauri-apps/api` through a dynamic import
// that is only reached when the Tauri global is present, so the web bundle
// never evaluates it. Outside Tauri every entry point degrades to a null/false
// result.

function isDesktop(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

async function invoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  const {invoke} = await import('@tauri-apps/api/core');
  return invoke<T>(command, args);
}

/** Invoke and return `null` outside Tauri or on failure (never throws). */
async function invokeOptional<T>(
  command: string,
  args?: Record<string, unknown>,
): Promise<T | null> {
  if (!isDesktop()) return null;
  try {
    return (await invoke<T>(command, args)) ?? null;
  } catch (error) {
    console.warn(`[tauri-bridge] ${command} failed:`, error);
    return null;
  }
}

// ============================================================
// Task A — provider key keychain persistence (P0-1)
// ============================================================

/** Read a provider key from the OS keychain; `null` when not stored. */
export function getProviderKey(provider: string): Promise<string | null> {
  return invokeOptional<string>('get_provider_key', {provider});
}

/** Persist a provider key to the OS keychain (never plaintext on disk). */
export async function setProviderKey(provider: string, key: string): Promise<boolean> {
  if (!isDesktop()) return false;
  try {
    await invoke('set_provider_key', {provider, key});
    return true;
  } catch (error) {
    console.warn('[tauri-bridge] set_provider_key failed:', error);
    return false;
  }
}

/** Remove a provider key from the OS keychain. */
export async function deleteProviderKey(provider: string): Promise<boolean> {
  if (!isDesktop()) return false;
  try {
    await invoke('delete_provider_key', {provider});
    return true;
  } catch (error) {
    console.warn('[tauri-bridge] delete_provider_key failed:', error);
    return false;
  }
}

/** Whether a provider key is currently stored in the OS keychain. */
export async function hasProviderKey(provider: string): Promise<boolean> {
  return (await invokeOptional<boolean>('has_provider_key', {provider})) ?? false;
}

// ============================================================
// Desktop pet — auto-takeover preference (设置 › 外观 › 形象)
// ============================================================

/**
 * Whether the pet window may auto-take over the bottom-right corner when the
 * main window is minimized to tray. Default false (off) — see
 * src-tauri/src/pet_prefs.rs. Always false outside the desktop shell.
 */
export async function getPetEnabled(): Promise<boolean> {
  return (await invokeOptional<boolean>('get_pet_enabled')) ?? false;
}

/**
 * Persist the pet auto-takeover preference (writes pet-prefs.json in Rust).
 * Turning it off also hides/destroys a pet window currently on screen.
 * Resolves false when not running in the desktop shell.
 */
export async function setPetEnabled(enabled: boolean): Promise<boolean> {
  return (await invokeOptional<boolean>('set_pet_enabled', {enabled})) ?? false;
}

// ============================================================
// Task B — no-restart config apply (P1-1 desktop side)
// ============================================================

/**
 * The gateway's effective config as returned by `GET /v1/admin/config`.
 * The body is passed through unchanged (JSON shape owned by the gateway), so it
 * is typed `unknown` until wave-2 pins the schema.
 */
export type GatewayEffectiveConfig = unknown;

/** Fetch the gateway's effective config for the settings UI to echo back. */
export function getGatewayEffectiveConfig(): Promise<GatewayEffectiveConfig | null> {
  return invokeOptional<GatewayEffectiveConfig>('get_gateway_effective_config');
}

/**
 * The loopback access token the desktop shell hands the UI over IPC. The
 * gateway's token gate covers the whole API face (only `/health` is exempt),
 * so every gateway request from the desktop UI must carry it.
 *
 * The desktop shell mints one per backend spawn (or passes an explicit env
 * value through); it crosses IPC in memory only — never persisted, never
 * logged. Empty string means "nothing to present": the gateway declares
 * itself unprotected and callers send no Authorization header. Fetch it per
 * call rather than caching — a backend restart rotates the token.
 */
export function getGatewayToken(): Promise<string | null> {
  return invokeOptional<string>('get_gateway_token');
}

// ============================================================
// Gateway token face — three-source token resolution (desktop / web build)
// ============================================================

/**
 * The gateway token gate's explicit "run without protection" sentinel
 * (`APEIRETH_GATEWAY_TOKEN=none` on the gateway side). Seeing it means the
 * operator declared the gateway unprotected: there is no token to present,
 * so callers send no Authorization header.
 */
export const UNPROTECTED_TOKEN_SENTINEL = 'none';

/** The two readable token sources: IPC (desktop shell) and env (web build). */
export interface GatewayTokenSources {
  /** IPC face: desktop-shell `get_gateway_token` (minted per sidecar spawn). */
  ipc: () => Promise<string | null>;
  /** env face: `VITE_APEIRETH_GATEWAY_TOKEN`, injected at web build time. */
  env: () => string | null;
}

/**
 * Three-source token resolution (pure, behavior-pinned): IPC (desktop) wins,
 * then env (web build injection), then none — the same precedence the desktop
 * shell uses when it resolves the sidecar's token. Blank values are skipped
 * layer by layer (never send an empty Bearer); the unprotected sentinel means
 * "explicitly unprotected" and resolves to nothing to present outright (no
 * fallthrough to a lower layer).
 */
export function pickGatewayTokenFace(
  ipcToken: string | null,
  envToken: string | null,
): string | null {
  const chosen = [ipcToken, envToken]
    .map((raw) => (typeof raw === 'string' ? raw.trim() : ''))
    .find((token) => token !== '');
  if (!chosen || chosen === UNPROTECTED_TOKEN_SENTINEL) return null;
  return chosen;
}

/** env face reader: web build injection; absent env = no token (gate semantics unchanged). */
function webBuildGatewayToken(): string | null {
  // `import.meta.env.VITE_APEIRETH_GATEWAY_TOKEN` 是 web 构建期静态替换面
  // (构建注入值直接编译进包); typeof 探针先行, Node 测试环境
  // (import.meta.env 缺席) 安静回落无令牌, 不炸。
  if (typeof import.meta.env === 'undefined') return null;
  const raw = import.meta.env.VITE_APEIRETH_GATEWAY_TOKEN;
  return typeof raw === 'string' && raw.trim() ? raw.trim() : null;
}

/** Default token sources: IPC first, then the web-build env injection. */
export const defaultGatewayTokenSources: GatewayTokenSources = {
  ipc: () => getGatewayToken(),
  env: () => webBuildGatewayToken(),
};

/**
 * Resolve the current loopback access token (three sources, fetched per call
 * and never cached — a sidecar restart rotates the token). `null` = nothing
 * to present: the gateway token gate is unset (compat path) or explicitly
 * unprotected, and callers send no Authorization header.
 */
export async function resolveGatewayToken(
  sources: GatewayTokenSources = defaultGatewayTokenSources,
): Promise<string | null> {
  return pickGatewayTokenFace(await sources.ipc(), sources.env());
}

/**
 * Gateway request auth headers: `Authorization: Bearer <token>` when a token
 * face is available, and nothing at all otherwise (the unset-gate compat path
 * keeps its exact previous behavior). Credentials (config.apiKey /
 * provider.apiKey) are never inbound auth and never enter this seam.
 */
export async function gatewayAuthHeaders(
  sources: GatewayTokenSources = defaultGatewayTokenSources,
): Promise<Record<string, string>> {
  const token = await resolveGatewayToken(sources);
  return token ? {Authorization: `Bearer ${token}`} : {};
}

// ============================================================
// Task C — workspace directory selection (P1-4 desktop side)
// ============================================================

/** The current workspace directory, or an empty string when unset. */
export function getWorkspaceDir(): Promise<string | null> {
  return invokeOptional<string>('get_workspace_dir');
}

/**
 * Validate, persist and apply a workspace directory; returns the new normalized
 * path, or null outside Tauri / on validation failure.
 */
export function setWorkspaceDir(dir: string): Promise<string | null> {
  return invokeOptional<string>('set_workspace_dir', {dir});
}

/** Common workspace candidates: home, documents, desktop, last-used. */
export function listWorkspaceSuggestions(): Promise<string[] | null> {
  return invokeOptional<string[]>('list_workspace_suggestions');
}
