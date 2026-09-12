// Typed IPC bridge for the desktop-side P0/P1 commands (wave-2 UI contract).
//
// This file only *adds* wrappers; it does not modify runtime.ts or the existing
// desktop-bridge.ts. Every wrapper mirrors the Rust commands registered in
// `src-tauri/src/lib.rs` (keychain.rs / workspace.rs / backend_supervisor.rs).
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
