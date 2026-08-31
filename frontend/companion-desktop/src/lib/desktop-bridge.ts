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

/** Absolute path of the log directory. */
export function getLogDirectory(): Promise<string | null> {
  return invokeOptional<string>('get_log_directory');
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
