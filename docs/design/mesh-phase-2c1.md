# Mesh Phase 2C1 — Secure WAN Rendezvous + Encrypted Relay

> Correctness path for WAN Mesh. Relay answers *how to reach*; rendezvous answers
> *where*; neither answers *who to trust* — that stays with the local
> `TrustStore`, and cryptography proves who is on the other end.

## 0. Scope

Two trusted nodes (already paired via the Phase 2B `TrustStore`) reach each
other over the public internet when neither is directly dialable:

```text
Home Node A ──outbound──▶ Rendezvous / Relay ◀──outbound── Remote Node B
```

The intermediate service is **connectivity infrastructure, not a peer**: it owns
no `ModuleId`, no `CapabilityId`, no `ModuleRegistry`, no `CapabilityRegistry`,
no `KnownPeer` trust; it never executes tools/providers; and because Mesh traffic
is end-to-end encrypted by the two nodes, it never decrypts a `MeshEnvelope`.

**Deliberately absent (Phase 2C2+):** STUN, TURN protocol compatibility, UDP hole
punching, ICE, UPnP, NAT-PMP, PCP, DHT, automatic scheduling, distributed memory,
agent migration. Relay is the correctness path; traversal may later become an
*optimization*, never a prerequisite.

## A. Current direct-connect path (Phase 2A/2B)

```text
MeshTransport::dial(endpoint: &str) -> Arc<dyn MeshConnection>
        │
        ▼
MeshNode::connect(link, expected) / MeshNode::accept(link)
        │  Hello / HelloAck  (mesh handshake, above the link)
        │  verify_pinned_identity(link.peer_fingerprint() == TrustStore pin)
        ▼
capability advert, RPC, heartbeats  (unchanged)
```

Direct QUIC (`engine/mesh-quic`) parses the endpoint as a `SocketAddr` and
terminates TLS with the certificate key **==** the mesh identity key, so the TLS
handshake *is* the identity proof and `peer_fingerprint()` is the pinned key.

## B. What requires a `SocketAddr`

Only `QuicMeshTransport::connect`'s `endpoint.parse::<SocketAddr>()` and
`QuicMeshListener::bind`. The `MeshTransport::dial(endpoint: &str)` trait is
string-keyed — the seam a relay needs.

## C. What assumes the peer is directly reachable

The dial endpoint is a reachable `host:port`; there is no rendezvous layer, and a
`KnownPeer` stores one direct endpoint string.

## D. Where a relay fits without touching Runtime

As a new `MeshTransport` / `MeshListener` pair (`RelayMeshTransport` /
`RelayMeshListener`) returning a `RelayMeshConnection` (a `MeshConnection`).
`MeshNode::connect/accept` run over it **unchanged**. A composite
`PeerConnector` (Direct → Relay) is also just a `MeshTransport`. Nothing in
`execute.rs`, `ProviderRouter`, `ToolRouteTable`, or `ModuleManager` learns
whether a peer is direct or relayed.

## E. What must remain end-to-end authenticated

`NodeIdentity` verification (fingerprint ↔ pin), the Hello/HelloAck handshake,
`CapabilityAdvert`, and every `MeshEnvelope` plaintext — all between A and B,
never terminated at the relay.

## F. Rendezvous protocol

A signed, expiring, self-describing reachability claim
(`mesh-protocol::rendezvous::PresenceRecord`), Ed25519 over a domain-separated
payload (`APEIRETH-MESH-PRESENCE-V1`) covering NodeId, public key (fingerprint),
audience, session id, expiry, relay capability, and nonce. Short TTL (30–120 s),
renewed by heartbeat. **Presence is not trust** — a client verifies the signature
*and* pins the fingerprint against its own `TrustStore`; a presence record can
never create/modify/erase a pin.

## G. Relay transport

**Option B** (chosen): A and B each dial *out* to the relay; the server pairs them
into a `RelayCircuit` and forwards only opaque end-to-end ciphertext frames. On
top of the tunnel the two nodes run a `SecureChannel` (X25519 ephemeral ECDH +
Ed25519 identity signatures + ChaCha20-Poly1305 AEAD) and present a
`RelayMeshConnection`. The relay's own TLS (when deployed) authenticates only
client↔infrastructure and is never mistaken for peer identity.

Option A (forward raw QUIC) was rejected: it would keep the E2E boundary tied to
the transport and risk the relay terminating Mesh identity. Option B makes the
"relay cannot inspect plaintext" property structural.

## H. Threat model

The relay/rendezvous server is **untrusted** for confidentiality and identity: it
can lie about presence, drop/reorder/inspect metadata, or replay stale records —
but without a node's private key it cannot impersonate a peer, and it cannot
decrypt Mesh payloads (they are E2E AEAD ciphertext). It is trusted only for
*availability*. Relay identity (TLS) is scoped to `client ↔ infrastructure` and
never conflated with `peer ↔ peer` identity.

## Invariants

- Rendezvous/relay is **not** a Node: no `ModuleId`/`CapabilityId`/registry/trust.
- `A` and `B` trust each other; neither trusts the relay.
- `NodeId` remains the canonical identity; presence/session/circuit/relay tokens
  are never identities.
- Relay is the correctness path; direct traversal (later) is an optimization.
