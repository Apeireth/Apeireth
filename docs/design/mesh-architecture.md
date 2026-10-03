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

Observation runs over UDP on its own socket. The control channel is TCP, and a
TCP-mapped address says nothing about the UDP mapping a QUIC endpoint will use,
so a candidate is only meaningful if it was observed from the socket that will
do the dialing. Closing that gap — dialing from the very port that was observed
— is what turns a list of candidates into a hole punch.

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
- **NAT traversal is not implemented end to end.** Peers behind NAT reach each
  other directly only when one side is reachable, and otherwise through the
  relay. The seam exists (`PeerConnector` strategies), and the first piece is
  now in place: `apeireth-mesh-protocol::observe` defines the server-reflexive
  observation datagram and the rendezvous server answers it on a UDP socket
  (`ServerConfig::with_observe_addr`), so a node can learn the public UDP
  address it should advertise. Still missing: carrying that candidate inside the
  signed presence record, a shared-socket QUIC endpoint (dial from the very port
  that was observed), simultaneous dial over a candidate list, and the
  node-level wiring that picks among candidates. **Loopback tests prove the
  plumbing only — they do not prove a real NAT is traversed**, and nothing here
  may be called "NAT traversal supported" until it is measured against one.
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
