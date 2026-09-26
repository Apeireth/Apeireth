// Provider HTTP transport — webview zero-egress relay.
//
// Under the Tauri shell the webview must not open sockets to provider APIs:
// the window CSP pins `connect-src` to loopback, and provider requests are
// relayed by the Rust side through the `provider_request` IPC command. In the
// dev browser (no Tauri) the call falls back to plain `fetch`, so `pnpm dev`
// and the non-desktop web build keep working unchanged.
//
// Transport selection follows the existing bridge pattern (desktop-bridge.ts):
// `isDesktop()` on the injected Tauri global, `@tauri-apps/api` loaded through
// a dynamic import that is only reached inside Tauri.
//
// The relay is non-streaming (buffered body, capped and timed out on the Rust
// side). Streaming chat paths keep their own channel and are not routed here.

import {isDesktop} from './desktop-bridge.ts';

/** Mirrors the Rust `ProviderResponse` payload (snake_case on the wire). */
export interface ProviderResponsePayload {
  status: number;
  body_text: string;
  content_type: string | null;
}

function headerRecord(headers: HeadersInit | undefined): Record<string, string> {
  const out: Record<string, string> = {};
  if (!headers) return out;
  for (const [key, value] of new Headers(headers).entries()) out[key] = value;
  return out;
}

function bodyString(body: BodyInit | null | undefined): string | undefined {
  if (body === null || body === undefined) return undefined;
  if (typeof body === 'string') return body;
  throw new Error('providerFetch: only string request bodies are relayed over IPC');
}

function responseFromPayload(payload: ProviderResponsePayload): Response {
  const headers = new Headers();
  if (payload.content_type) headers.set('content-type', payload.content_type);
  // 204/205/304 carry no body by definition; empty text maps to the null body
  // fetch would have produced, so `text()`/`json()` behave identically.
  const empty =
    payload.body_text === '' || payload.status === 204 || payload.status === 205 || payload.status === 304;
  return new Response(empty ? null : payload.body_text, {status: payload.status, headers});
}

async function relayViaTauri(url: string, init: RequestInit): Promise<Response> {
  const {invoke} = await import('@tauri-apps/api/core');
  const payload = await invoke<ProviderResponsePayload>('provider_request', {
    url,
    method: init.method ?? 'GET',
    headers: headerRecord(init.headers),
    body: bodyString(init.body),
  });
  return responseFromPayload(payload);
}

/**
 * fetch-compatible entry point for provider API calls.
 *
 * Tauri: relayed through `provider_request` (Rust sends the request).
 * Otherwise: plain `fetch`, identical to the previous behavior.
 *
 * `init.signal` is honored on the fetch path only; the IPC relay cannot be
 * cancelled mid-flight and is bounded by the Rust-side timeout instead.
 */
export async function providerFetch(input: string, init: RequestInit = {}): Promise<Response> {
  if (!isDesktop()) return fetch(input, init);
  return relayViaTauri(input, init);
}
