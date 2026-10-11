# Mesh architecture and invariants

Status: living document. It records what the mesh guarantees today, the rules
every change must keep, and what is still missing. The phase documents
(`mesh-phase-0.md`, `mesh-phase-2c1.md`) describe how the pieces arrived; this
one describes the whole.

## One sentence

The mesh moves *where a capability call executes*. It never moves who owns the
capability, who may call it, or who is trusted.

## Layers

| Layer | Crate | Owns | Must not |
|---|---|---|---|
| Wire | `apeireth-mesh-protocol` | Envelope, adverts, pairing and rendezvous messages, error codes | Open sockets, make trust decisions |
| Node | `apeireth-mesh` | Peers, live trust policy, route table, call correlation, pairing ceremony, relay client | Own plugins, run governance of its own |
| Transport | `apeireth-mesh-quic` | Authenticated QUIC, framing | Interpret envelopes |
| Infrastructure | `apeireth-mesh-rendezvous` | Presence leases, relay circuits | Read traffic, hold trust, hold capabilities |
| Port | `apeireth-runtime` (`canonical/mesh.rs`) | Adapting the node to the runtime's remote-capability port | Cache ownership |

## Invariants

1. **`PluginManager` is the only local ownership authority.** The mesh derives
   its local catalog from `PluginManager::records()` on every query
   (`ManagerExecutor::catalog`). Remote adverts are read-only *projections*
   keyed by `(node, capability)`. A remote route never implies local
   ownership, never shadows a local capability, and selection is always
   explicit (`RouteSelection::Local` or `Remote(node)`).
2. **Governance is not bypassed.** An inbound call runs the executing node's
   canonical governance with `ActionSource::Remote { node }`. A decision that
   would need a human fails closed (`ApprovalRequired`); the mesh never mints
   an approval. Requester-side authorization uses the same hook.
3. **Discovery is not trust.** mDNS and rendezvous presence report that a node
   may exist. Only an explicit `KnownPeer` (from the pairing ceremony or
   configuration) admits it.
4. **Identity is a key.** A `NodeId` is a name; the Ed25519 key behind it is
   pinned by SHA-256 fingerprint. The TLS certificate key *is* the identity
   key. There is no trust on first use: a link that presents a key for a node
   with no pin is refused, and a pinned node must present exactly its pin.
5. **Rendezvous and relay are untrusted.** They can deny service. They cannot
   read relayed traffic (end-to-end encrypted between the nodes) or impersonate
   a peer (the nodes authenticate each other's pinned keys end to end).
6. **Replies, cancels and executions are bound to their peer.** A reply is
   honoured only from the peer the call went to; a `Cancel` reaches only the
   invocation its own sender started (`(origin, message id)` keys); a message
   id already executing for a peer is a protocol violation.
7. **Everything is bounded.** Pending calls per peer, concurrent inbound
   executions per peer, concurrent handshakes, caller wait time, frame size,
   rendezvous connections (total and per IP), pairing sources.
8. **Diagnostics name shapes, not contents.** Errors never interpolate payloads
   (tool arguments, results, prompts, completions).
9. **Nothing is silently replaced.** A pinned key is never overwritten by
   pairing (`KeyMismatch`; recovery is `forget` then a new ceremony). A node
   identity file is never regenerated because of an I/O error.
10. **No hand-rolled cryptography.** Signatures, hashing, AEAD and key
    agreement come from `ring`/`rustls`/`sha2`. Security checks are not relaxed
    to make tests pass; relaxed verifiers live only under `testing`.

## Connection lifecycle

```text
transport link ──► Hello / HelloAck ──► trust + pin check ──► Active
                   (version, identity)   (live TrustPolicy)    │
                                                               ▼
                                  advert exchange ◄──► invocations, cancels,
                                                       heartbeats
Active ─ heartbeat silence ─► Unreachable ─ redial (maintain_connection) ─► Active
Active ─ revoke_peer / shutdown ─► Closed
```

- Duplicate connections converge deterministically: the connection initiated
  by the lexicographically smaller `NodeId` wins on both ends. A dead
  incumbent always yields. A failed second dial never evicts a live link.
- `revoke_peer` is immediate: untrust, drop the pin, close the link, fail
  pending calls, drop the projection. Every inbound envelope re-checks trust.
  Persisted removal is `TrustStore::forget`; the two are separate calls.
- `admit_peer` makes a freshly paired peer connectable without a restart.
- `shutdown` closes links, fails pending calls, cancels executions, and stops
  the serve and heartbeat loops.
- `maintain_connection` redials with bounded jittered backoff and ends on
  shutdown, revocation, or an `Untrusted` refusal.

## Transport security

- Production QUIC configs are mutual TLS pinned to identity keys.
- The **pairing channel** is mutual TLS too, with no pin set: the peer must
  prove possession of an Ed25519 key (handshake signature verified against the
  certificate's own SPKI). The pairing protocol then binds its claims to that
  proven key. Only Ed25519 handshake signatures are offered.
- The certificate fingerprint is read from `tbsCertificate`'s
  `subjectPublicKeyInfo` by a strict DER walker, never by searching bytes. A
  certificate carrying a victim's key elsewhere cannot pass a pin.

## Pairing (protocol v2)

Both devices derive a 6-digit short authentication string over a
domain-separated hash of the full transcript (both node ids, both channel key
fingerprints, both nonces, version). Nonces are exchanged commit-then-reveal:
the initiator sends `SHA-256(domain, id, nonce)`, the responder answers with
its nonce, and only then does the initiator reveal. Whoever revealed last could
otherwise grind a nonce until two MITM sessions show the same code. Trust is
persisted only after both users confirm; any failure leaves nothing behind.
`pair_outbound` refuses a responder that names a node other than the one
dialled. `PairingOptions::with_limiter` applies admission control to inbound
ceremonies.

## Endpoint observation (the first half of NAT traversal)

A node behind a NAT cannot learn its own public UDP address by reasoning — only
someone outside can look and tell it. `apeireth-mesh-protocol::observe` is that
question: one fixed-size 64-byte probe, one fixed-size 64-byte reply carrying
the source address the observer actually saw. Both directions are the same
size, so an observer is not a usable reflector, and a malformed datagram is
never answered.

The answer is a **hint, never a grant**. It cannot add, remove or alter a pin,
and it grants no trust: the address it names still has to survive the pinned
QUIC handshake before a link exists. The rendezvous server offers it only when
configured (`ServerConfig::with_observe_addr`), and a node that never observes
anything still works over the direct and relayed paths — observation is an
optimization, never a prerequisite.

Observation runs over UDP on its own socket, and that socket is the *same*
socket QUIC will dial from. The control channel is TCP, and a TCP-mapped address
says nothing about the UDP mapping a QUIC endpoint will use; and a candidate
observed from socket A while QUIC dials from socket B describes a mapping
nothing will ever use.

So the lifetime is single-owner:

```text
UdpSocket::bind
      |
  observe()           probe and reply both on this socket
      |
  into_quic_client()  the socket is MOVED into quinn
      v
  quinn::Endpoint     quinn owns every datagram from here
```

`PreparedMeshEndpoint` (in `apeireth-mesh-quic`) owns the bound socket.
`observe()` borrows it and probes from its own local port; `into_quic_client()`
consumes `self` and hands the socket to `QuicMeshTransport::from_bound_socket`,
which takes it exactly as given and re-binds nothing. Observation therefore
happens strictly *before* the handoff, so no second reader ever competes with
quinn for the same datagrams — "observe after handoff" is not expressible,
not merely discouraged.

## Endpoint candidates

A presence record used to say *who* is reachable and never *where*. It now
carries `EndpointCandidate { kind, addr }` with
`EndpointCandidateKind { Host, ServerReflexive }`, and the list sits inside the
signed payload:

- the signed bytes are **binary** — family tag, exact-width address,
  fixed-width port — so `::1` and `0:0:0:0:0:0:0:1`, one address, cannot yield
  two verifying encodings;
- candidates are canonicalized before signing (validated, sorted by kind
  preference then family then bytes then port, de-duplicated, bounded by
  `PRESENCE_MAX_CANDIDATES`) and `verify()` requires that canonical order, so a
  valid signature cannot be replayed over a reordered or padded list;
- unspecified, multicast and IPv4-broadcast addresses are refused, as is a zero
  port. Loopback, private and link-local addresses are allowed: a host candidate
  is exactly "the LAN address I can be reached at";
- relay is deliberately **not** a candidate kind. A relay is an intermediary
  with its own identity and allocation metadata, not an address the destination
  answers at, and freezing it as one would bake in a wrong wire representation;
- `session_id` is the presence generation. There is no second `generation`
  field: a new session is a new incarnation, and a version 1 record cannot
  verify under the version 2 domain at all.

`PresenceRecord::dialable_candidates()` and `apeireth_mesh::dial_plan()` are the
only places signed candidates become addresses to dial. Neither verifies:
verification happens once, where the record arrives, so the dial path below can
never see an unverified address.

## Dialing

`MeshTransport::dial_candidates` tries a plan in order with a per-attempt
deadline, so one dead address cannot stall an attempt. `PeerConnector` keeps the
strategy order decisive — an earlier strategy gets every candidate before a
later one is considered at all — which is what preserves "direct before relay"
once the direct side has several addresses.

A candidate is *where* to dial. *Who* answers is proved by the pinned handshake
above this boundary, and an identity that does not match the pin still fails
closed there.

## Rendezvous server
- First key wins while a lease lives; refreshes must advance the expiry.
- Leases are capped at the protocol maximum; expired presence is pruned.
- A connection holds one registration. Withdraw, circuit open/accept/close and
  relay frames must come from the owning connection or circuit endpoint.
- Per-node circuit counts are derived from the circuit tables.
- Timeouts for registration, idle and write; limits per IP and in total.
- Clients verify lookup results (node, audience, signature, relay-capable)
  before dialing; trust is still decided by the pin after the end-to-end
  handshake.

## Not covered by this document's guarantees

- **MCP and other dynamic tools are not advertised over the mesh.** They live
  in the runtime's own `CapabilityRegistry` (module-id strings), not in
  `PluginManager`, so the mesh cannot see them. Exposing them would first need
  an ownership story for them.
- **NAT traversal is implemented end to end in the connection path, but no
  NAT has ever been traversed by this code.** The mechanics exist and are
  driven by the production flow: `MeshNode::connect_punched` runs trust gate
  → presence gate → `PunchCoordinator` → simultaneous open over one dual-role
  socket → the ordinary pinned handshake → the ordinary convergence, and the
  Linux namespace harness (`scripts/test-mesh-nat.sh`, `nat_probe punch`)
  exercises that same path against nft-masquerade NATs with a real remote
  invocation. What is missing is *execution and measurement*: the harness was
  written on a Windows host and has not been run, and nothing here may be
  called "NAT traversal supported" until it has been run on Linux and, after
  that, against a real NAT. Loopback proves the path, not the traversal.
- **The rendezvous control channel is plain TCP.** An on-path attacker can drop
  or delay traffic and race a captured registration. Confidentiality and peer
  identity are unaffected (end-to-end), availability is.
- **Adverts carry no generation.** They are full snapshots on an ordered
  stream, so reordering is not a concern today; a transport without ordering
  would need one.
- **No structured logging.** Errors are typed; connection lifecycle is not yet
  traced.
- **Inbound pairing is not exposed by the CLI** (the ceremony and its limiter
  exist in the library).
- **Per-peer inbound policy** beyond governance (for example "node X may call
  only tools") is not modelled; governance decides.

## Known repository failures unrelated to the mesh

Both fail on a clean upstream checkout of the same base:

- `apeireth-core` `release_manifest_tests::test_release_version_is_2_0_0_rc_1`
  pins `rc.1` while the workspace is `rc.2`.
- `apeireth-tools-canonical` `apply_patch::tests::apply_with_gate_records_observations_and_enforces_version_cas`.


## Running the full workspace suite

A full cargo test --workspace with cargo's default job count dies silently
part-way through linking on a 16 GB / 12-core machine. Each link.exe for a test
binary peaks around 1-2 GB, and fanned out across every core the memory pressure
kills linkers with no error text at all - which reads as an unexplained
"link.exe ... exit code 1". The same pressure grows H:\pagefile.sys, and that is
what fills H:, not the target directory on its own. Bound the job count and keep
the target dir off H::

    CARGO_TARGET_DIR=E:\apeireth-mesh-verify-target
    cargo test --workspace --no-fail-fast -j 4

With -j 4 the run completes: 172 suites, 4865 passed, 2 failed, 21 ignored. The
two failures are exactly the pre-existing pair listed above, and they were
verified as pre-existing rather than assumed: at baseline origin/main the
workspace already reported 2.0.0-rc.2 while crates/foundation/core/src/lib.rs
already asserted 2.0.0-rc.1, no replay commit ever changed a version line, and
no replay commit touched crates/capabilities/tools at all.