# M2D — Cross-Platform Network / Egress Enforcement

Status: M2D-A + M2D-B complete (controlled egress policy + transport).
Process network isolation backends remain `UNSUPPORTED` on all three platforms
after audit; this is an honest platform capability, not a blocker for the
controlled egress boundary.

Branch: `reconstruct_v2`

## 1. Two different boundaries

```text
Controlled Application Egress                 Arbitrary Process Network Isolation
(Apeireth controls the request)               (child controls its own sockets)
        |                                             |
Runtime -> Governance -> ToolCapability        ToolCapability -> ProcessExecutor
        |                                             |
EgressPolicy -> EgressTransport                Platform NetworkIsolation backend
        |                                             |
HTTP/DNS                                       Child process
```

They are different APIs and different security guarantees. A safe HTTP client
does not restrict `curl` inside a child process. A Job Object / process group
does not make an HTTP destination policy safe.

## 2. Donor audit

Source: `origin/master:reconstruction_v2/`.

| Donor primitive | Classification | Notes |
| --- | --- | --- |
| `apeireth-http-client/src/egress.rs` | STUB | `check_outbound` always `Ok`; no policy. |
| `apeireth-http-client/src/lib.rs` | STUB | `get`/`post` return fabricated responses. |
| `apeireth-gateway/src/egress.rs` | PARTIAL | Has default host allowlist, port allowlist, and IPv4 private/loopback check, but only validates IP literals; hostnames are not resolved before allow decision; wildcard suffix matching is naive. |
| `apeireth-tool-fetch/src/http_fetch.rs` | BROKEN for SSRF | Uses opaque client with `follow_redirects: true`; no redirect revalidation; no DNS pinning; no response bound beyond caller config. |
| `apeireth-tool-browser/src/fetch.rs` | BROKEN for SSRF | Opaque redirects; no per-hop destination validation. |
| `apeireth-sandbox-net` / companion `sandbox_net.rs` | PARTIAL | Loopback/domain/port allowlist concepts, not OS process isolation. |

Reused: URL/port/scheme validation ideas, IP classification approach.
Rejected: opaque redirect following, hostname-string-only checks, wildcard
suffix allowlists, unbounded response reads, ambient proxy inheritance.

## 3. Controlled egress implementation

Location: `crates/apeireth-tools-canonical/src/egress.rs`.

Pipeline:

```text
parse URL
  -> validate scheme (http/https), reject userinfo
  -> validate host + port against EgressPolicy
  -> resolve host to SocketAddrs
  -> validate every resolved address against EgressPolicy
  -> build reqwest client with no_proxy + resolve_to_addrs pinning
  -> GET with redirect policy none
  -> on 3xx: parse Location, re-run full policy for the new URL
  -> bound response size
```

### Policies

- `DenyAll`
- `PublicInternetOnly`
- `ExplicitAllowList(EgressAllowList)`
- `Unrestricted` (explicit opt-out)

### Address classes

IPv4 loopback, private, link-local, multicast, unspecified, broadcast are
classified via the Rust standard library. IPv6 loopback, ULA (`fc00::/7`),
link-local, multicast, unspecified are classified. IPv4-mapped IPv6 addresses
are classified by their embedded IPv4 address, so `::ffff:127.0.0.1` is
loopback and `::ffff:10.0.0.1` is private.

### DNS semantics

- IP literals bypass DNS and are validated directly.
- Hostnames are resolved once, then the resolved addresses are validated.
- `PublicInternetOnly` denies the destination if **any** resolved address is
  non-public (mixed public/private DNS result => deny).
- DNS failure is `ResolutionFailed`, never an implicit allow.

### DNS pinning

For hostname destinations the transport builds a per-request `reqwest::Client`
with `resolve_to_addrs(host, validated_addrs)` and `no_proxy()`. The URL host
is unchanged, so Host header and TLS SNI/certificate validation still target
the original hostname while the connection is pinned to validated addresses.

### Redirects

- `reqwest` redirect policy is `none`.
- Every 3xx hop is parsed, scheme/port validated, resolved, and address-validated.
- Relative and absolute `Location` are supported through `url::Url::join`.
- Malformed `Location` is `RedirectDenied`.
- HTTPS -> HTTP downgrade is denied.
- Maximum 10 hops (`TooManyRedirects`).

### Transport bounds

- Timeout: 30s default (`connect_timeout` + `read_timeout` + total timeout).
- Response body: 1 MiB default; `ResponseTooLarge` if exceeded.
- No request body support (GET only).
- Ambient `HTTP_PROXY` / `HTTPS_PROXY` / `ALL_PROXY` are disabled via
  `no_proxy()`. Proxy-based restricted transport is explicitly unsupported.

### Errors

`InvalidUrl`, `UnsupportedScheme`, `ResolutionFailed`, `DestinationDenied`,
`RedirectDenied`, `ConnectionFailed`, `TlsFailed`, `ResponseTooLarge`,
`TooManyRedirects`, `Timeout`.

### Tests

Pure policy tests cover public/private/loopback/link-local/IPv6/mapped IPv6,
mixed DNS results, allowlist exactness, malformed URL, unsupported scheme, and
userinfo rejection. Transport tests use a local loopback listener and an
injected fake resolver to prove:
- pinned resolver is the contacted address (hostname does not resolve in real DNS)
- redirect to denied destination fails before contact
- local transport still works when ambient proxy variables are set

## 4. Process network isolation audit

Existing `ProcessExecutor` continues to report `NetworkIsolation = UNSUPPORTED`
on every platform; requiring `NetworkIsolation = Enforced` still fails closed
before child spawn.

| Platform | Mechanism audited | Conclusion | Reason |
| --- | --- | --- | --- |
| Windows | AppContainer | DEFERRED / UNSUPPORTED | Arbitrary Win32 child AppContainer launch without network capability, while preserving CREATE_SUSPENDED + JobObject integration, was not proven on the normal-user runner. |
| Windows | WFP / firewall rules | DEFERRED | Requires admin/service/global side effects; unsuitable as a default unprivileged backend. |
| Linux | user + network namespace (`CLONE_NEWUSER` + `CLONE_NEWNET`) | DEFERRED / UNSUPPORTED | Requires runtime detection and correct uid_map/gid_map/setgroups handling; not validated on this runner; would need a real child `connect` failure test. |
| Linux | seccomp socket/connect deny | DEFERRED | A half syscall list is not isolation; no mature in-workspace policy exists. |
| macOS | public/stable subprocess sandbox | UNSUPPORTED | No public, stable, arbitrary-subprocess network sandbox primitive was found; private APIs are excluded by rule. |
| macOS | pf / Network Extension | DEFERRED / UNSUPPORTED | Root/global or entitlement/app packaging requirements are not canonical default backends. |

Process-level allowlist (per-hostname/domain egress) for arbitrary child
processes is intentionally out of scope for M2D; only deny-all would be
attempted first.

## 5. Capability matrix

| Capability | Windows | Linux | macOS |
| --- | --- | --- | --- |
| Controlled application egress | ENFORCED (application-owned, all OSes) | ENFORCED | ENFORCED |
| Process `NetworkIsolation` | UNSUPPORTED | UNSUPPORTED | UNSUPPORTED |
| Filesystem isolation | UNSUPPORTED | UNSUPPORTED | UNSUPPORTED |

## 6. Remaining gaps

- Controlled egress supports GET only; POST/HEAD and request bodies are future.
- Explicit proxy support is deferred; restricted transport disables ambient proxies.
- Process-level network isolation is not available on any platform in this phase.
- `PublicInternetOnly` treats documentation IPv6 ranges as public; if stronger
  classification is needed it belongs to a future policy revision.
