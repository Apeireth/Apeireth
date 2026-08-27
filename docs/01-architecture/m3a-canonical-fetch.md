# M3A — Canonical Controlled Fetch Capability Port

> **现状 (2026-08-27)**：本文是 v1 时代（master 线/86-crate）或 reconstruct_v2 过程中的历史快照，正文保留原样。当前基线：默认分支 `main`、13-crate 工作区（`crates/foundation|engine|capabilities|adapters`，见根 `ARCHITECTURE.md` 与 `docs/01-architecture/architecture.md`）、tag `v2.0.0-alpha.1` @ `d6910cf7`；旧 86-crate 代码整体在 `legacy/`（workspace exclude）；v2 下一步见根 `ROADMAP.md` §4。

Status: COMPLETE

Branch: `reconstruct_v2`

## 1. Scope

`tool.fetch` is a deliberately narrow GET-only HTTP(S) text fetch capability.
It is not a browser, an API client, a crawler, or a file downloader.

The phase is also a canonical architecture acceptance test: master already had
Fetch functionality; M3A proves it can enter the frozen
Plugin / Governance / Tool / Egress / Runtime boundaries without importing
master architecture.

## 2. Master donor audit

Master donor SHA: `7f515aab37d1a9e58f1eedb0a92691cfe496f4d6`

| Donor component | Actual maturity | Useful behavior | Security gap | Strategy |
| --- | --- | --- | --- | --- |
| `reconstruction_v2/crates/apeireth-tools/src/builtin/fetch.rs` | BROKEN | Schema idea (`url`, optional `method`), status-as-result idea, medium risk label | Validates DNS then calls a plain `reqwest::Client`; second DNS lookup after validation; opaque redirects; no DNS pinning; no `no_proxy()`; supports POST; no body bound until after full `text()` read; `text()` returns empty on invalid UTF-8 instead of an error | DROP implementation; ADAPT narrow schema idea and status-as-result semantics |
| `reconstruction_v2/crates/apeireth-tool-fetch/src/http_fetch.rs` | BROKEN for SSRF | Error UX idea (`TooLarge`, `Http`), content-type capture, `final_url` metadata idea | Uses `apeireth-http-client` with opaque redirect following and no per-hop destination revalidation; POST/body support; HTML extraction and retry/cache/metrics are out of scope for M3A | DROP implementation; ADAPT `TooLarge`-style bounded-failure wording |
| `reconstruction_v2/crates/apeireth-tool-fetch/src/config.rs` | PARTIAL | Timeout/UA/body-limit config shape | Retries, cache, redirect toggle; no egress policy | DROP; canonical `FetchConfig` owns an `Arc<ControlledEgress>` and a fixed optional User-Agent |
| `crates/apeireth-tool-fetch` (legacy current-branch copy) | BROKEN for SSRF | None beyond donor above | Same as donor; duplicate HTTP stack and registry | DROP |

The donor's central SSRF gap is confirmed: destination validation is performed
by the tool, then a different, opaque HTTP client performs a fresh DNS lookup
and follows redirects without revalidation. Canonical Fetch must not do that.

## 3. Canonical architecture path

```text
Provider
  ↓ ToolCall
Runtime
  ↓
Governance
  ↓
PluginRegistry / ToolCapability
  ↓
FetchTool
  ↓
EgressPolicy
  ↓
EgressTransport
  ↓
DNS / HTTP(S)
```

Runtime is fetch-aware? NO. Runtime stores only opaque `FrozenInvocation`
payloads; it never sees Fetch URL, policy, or transport details.

## 4. Fetch contract

| Item | Value |
| --- | --- |
| Capability ID | `tool.fetch` |
| Tool name | `fetch` |
| Default enabled | DISABLED (`BuiltinToolsOptions.fetch = None`) |
| Risk | `medium` |
| Method | `GET` only |
| URL max | 8192 bytes |
| Timeout | transport default 30 s; configurable only through trusted `ControlledEgress` |
| Response max | transport default 1 MiB; configurable only through trusted `ControlledEgress` |
| Content types | `text/*`, `application/json`, `application/xml`, `application/xhtml+xml`, `application/javascript`, `application/*+json`, `application/*+xml`; missing Content-Type is accepted as text only for valid UTF-8 without NUL |
| Binary | Rejected. No base64, no save-to-disk |
| Cookies | None. No jar, no persistence, no `Set-Cookie` in results |
| Authentication | None. No Authorization, no bearer token, no Basic Auth |
| Headers | Fixed `accept: */*` and optional trusted `user-agent` only. No model-supplied headers |
| Proxy | Ambient proxy disabled by `ControlledEgress::no_proxy()` |
| Redirects | Transport-owned; every hop fully revalidated; HTTPS -> HTTP downgrade denied; max 10 by default |
| Retry | No hidden retry |

## 5. Configuration

```rust
pub struct FetchConfig {
    egress: Arc<ControlledEgress>,   // trusted policy + bounds live here
    user_agent: Option<String>,      // fixed factual UA, model cannot override
}
```

`FetchConfig` helpers: `public_internet_only()`, `explicit_allow_list(list)`,
`deny_all()`, `unrestricted()` (documented opt-out), and
`with_user_agent(...)`.

Default policy when enabled without an explicit transport: constructors
prefer `PublicInternetOnly`. `BuiltinToolsOptions.fetch` defaults to `None`,
so production bootstrap gains no network capability silently.

Model-controllable security config? NO. The model supplies only a URL.

## 6. Request schema

```json
{
  "type": "object",
  "properties": {
    "url": {
      "type": "string",
      "minLength": 1,
      "maxLength": 8192,
      "description": "HTTP(S) URL to fetch with GET. URL userinfo, unsupported schemes, and non-http(s) schemes are rejected."
    }
  },
  "required": ["url"],
  "additionalProperties": false
}
```

Unknown parameters are rejected by the tool.

## 7. Response contract

Success value:

```json
{
  "url": "https://example.com/final",
  "status": 200,
  "content_type": "text/plain; charset=utf-8",
  "body": "hello fetch",
  "bytes": 11,
  "redirects": 0
}
```

- `url` is the final effective URL with any fragment stripped.
- 4xx/5xx are successful tool executions carrying factual `status`.
- 204/304/empty bodies are empty text, not errors.
- `body` is raw UTF-8 source content; no Markdown fences, no HTML stripping.

## 8. Egress security

| Concern | Behavior |
| --- | --- |
| Scheme validation | `http` / `https` only; `file:`, `data:`, etc. rejected |
| Private IP | Rejected under `PublicInternetOnly`; explicit allow-list controls private targets |
| IPv6 | Loopback, ULA, link-local, multicast, unspecified classified |
| DNS | Resolved once per hop; mixed public/private result fails closed |
| DNS pinning | `resolve_to_addrs` pins the validated addresses for each hop |
| Redirects | Manual per-hop revalidation; no opaque redirect following |
| HTTPS downgrade | Denied |
| TLS | Certificate and hostname verification always on |
| Proxy | Ambient proxy disabled |
| Body limit | Transport stream bound before Fetch decodes |
| Timeout | Connect + read + total bound |

Fetch itself performs no IP classification and no DNS lookup for policy.

## 9. Credentials

Authorization: NONE
Cookie: NONE
Proxy-Authorization: NONE
Ambient credentials: NOT SENT
URL userinfo: REJECTED before transport, never logged

## 10. Content handling

| Content type | Behavior |
| --- | --- |
| `text/plain` | Returned as UTF-8 text |
| `text/html` | Returned raw; no HTML extraction |
| `application/json` | Returned raw text; no semantic mutation |
| `application/xml` | Returned raw text |
| Missing Content-Type | Valid UTF-8 without NUL -> accepted as text; otherwise rejected |
| Non-UTF8 | UTF-8/ASCII charsets accepted; other declared charsets rejected with `unsupported charset=` |
| Binary (`image/*`, `audio/*`, `video/*`, `application/octet-stream`, zip, pdf) | Rejected as `unsupported media type` |
| PDF | Rejected; no parsing, no OCR |

## 11. HTTP status semantics

| Status | Behavior |
| --- | --- |
| 2xx | Factual success with body |
| 3xx | Transport follows and revalidates; final status is returned |
| 4xx | Factual fetch success with status and body |
| 5xx | Factual fetch success with status and body |
| 429 | Factual result with status and body; no auto sleep/retry |

## 12. Frozen Invocation

Implemented: YES

Why: `tool.fetch` can be placed under `RequireApproval` by ordinary
governance configuration. An approved fetch must not silently widen if trusted
configuration changes while the approval is pending.

Fields:

```text
version, method, url, timeout_ms, max_response_bytes, max_redirects,
egress_policy, user_agent
```

Config drift behavior: execution reconstructs a `ControlledEgress` from the
frozen `egress_policy` and frozen bounds. Current configuration is not used.

DNS behavior: DNS is intentionally not frozen. The actual connection is
resolved and revalidated at execution time by the controlled transport; the
security property is that resolved addresses must satisfy the approved policy,
not that they match a stale preview.

Claim-before-effect: unchanged; the generic M2C approval lifecycle persists the
claim before dispatch, so HTTP contact happens only after approval.

## 13. Trust marking

Fetched remote content is untrusted external content. Destination policy
controls *where* Apeireth connects; it does not control *what* the remote
server says. Prompt-injection governance remains a separate context/input
security layer.

## 14. Out of scope / deferred

Browser, DOM, JavaScript, cookies, POST/body, authenticated HTTP, binary
download, PDF parsing, HTML extraction, Markdown conversion, crawler, robots,
persistent HTTP session, MCP HTTP, filesystem save, content provenance.

## 15. Security wording

This capability provides policy-enforced destination validation, DNS-pinned
controlled transport, and redirect target revalidation. It is not marketed as
"SSRF-proof".

## 16. Source guards

- `tests/architecture_invariants.rs` verifies Fetch production source does not
  contain `reqwest::`, `tokio::net::lookup_host`, `Command::new("curl")`,
  `Command::new("wget")`, `ProcessExecutor`, or hand-rolled SSRF helpers.
- Runtime canonical invariant verifies Runtime source does not mention
  `tool.fetch`, `FetchTool`, `EgressTransport`, or `EgressPolicy`.
