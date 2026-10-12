# Apeireth Mesh RC2 Status

> 冻结时点快照（2026-10-11）。权威现状以 [`mesh-architecture.md`](mesh-architecture.md)
> 为准；本文只是一页纸摘要。证据记录见
> [`mesh-nat-e2e-report.md`](mesh-nat-e2e-report.md)。分支 `replay/mesh-on-rc2`，
> 未 push。

## Transport

- **Direct QUIC: verified.** Pinned mutual TLS (certificate key IS the identity
  key), mesh handshake, duplicate-connection convergence — loopback integration
  and hardening suites.
- **Punching: experimental.** `connect_punched()` is wired into the production
  connection path and verified on Windows loopback only. **Not verified under
  any real NAT**; the Linux namespace harness exists but has not been run.
  Best-effort direct connectivity, no compatibility guarantee.
- **Relay: implemented, not orchestrated.** The encrypted relay path
  (rendezvous presence + E2E circuits) exists and is covered by the WAN
  integration suites, but no flow automatically falls back from a failed
  direct attempt to the relay.

## Security

- Identity: verified (Ed25519 node identity, restricted atomic key file).
- Trust: verified (explicit known peers, no TOFU, live revocation, durable
  pinning).
- Authorization boundary: preserved. Inbound calls run the executing node's
  canonical governance (`ActionSource::Remote`); the mesh never mints an
  approval; punched links pass the same pinned handshake as any other link.

## Testing

- Mesh regression: passed (mesh / mesh-protocol / mesh-quic / mesh-rendezvous
  suites, 211 tests, 0 failures).
- Workspace regression: 4935 passed, 2 failed, 21 ignored — both failures are
  the pre-existing baseline pair (`apeireth-core` release-version assertion
  `rc.1` vs workspace `rc.2`; `apeireth-tools-canonical` `apply_patch` CAS
  test), unchanged and unrelated to the mesh.
- Loopback E2E (production punch path): passed — presence → punch →
  authenticated QUIC → convergence → remote `tool.mesh.echo` with result
  content checked and exactly one execution.
- Real NAT: not tested.

## Known limitations

- No automatic transport fallback (direct → punch → relay is a documented
  future state machine, not current behaviour).
- No real NAT verification (no Linux harness run, no router, no mobile
  network).
- No production gateway discovery integration: `connect_punched`'s first
  caller is the test harness; production dialing remains the static-endpoint
  `maintain_connection` path plus inbound `serve`.
- Multi-peer caveat: a punch's accept half can pick up a connection from a
  different (already pinned) peer; the handshake then fails closed (an
  availability question, never a trust one).
- The two baseline test failures above remain open and are not mesh-related.

## Frozen classification

- Stable / verified: node identity, trust model, pairing, authenticated QUIC
  transport, mesh handshake, remote runtime invocation, presence exchange,
  fail-closed validation, deterministic connection convergence.
- Experimental: NAT traversal (candidate exchange, presence validation, punch
  coordination, `connect_punched`, shared dual-role socket, QUIC
  connect/accept convergence) — loopback verified only.
- Not implemented: automatic transport orchestration, the direct → punch →
  relay state machine, production gateway discovery integration.
