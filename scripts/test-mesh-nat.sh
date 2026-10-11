#!/usr/bin/env bash
#
# Apeireth Mesh — Linux namespace NAT harness.
#
#   peer-a -- nat-a -- transit -- nat-b -- peer-b
#                            |
#                    observer / rendezvous
#
# WHAT THIS PROVES WHEN RUN ON LINUX:
#   TEST A  endpoint observation: each peer's observed public mapping comes
#           from the very socket QUIC then binds (single-owner handoff).
#   TEST B  simultaneous open: two peers holding each other's *verified*
#           presence establish a mutually authenticated direct QUIC link
#           through both NATs, driven by the production connection path
#           (MeshNode::connect_punched), with real UDP packets on both sides.
#   TEST C  convergence: the race produces one live logical link per side
#           (LINK_COUNT=1), never two.
#   TEST D  canonical remote invocation: a real tool.mesh.echo call crosses
#           the punched link and the *result content* is checked, with the
#           executing side counting exactly one execution.
#   TEST E  bounded failure: against a NAT that drops all inbound, the punch
#           fails inside its window on both sides — no hang, no infinite
#           retry. (Worst-case bound, NOT a model of any real NAT type.)
#
# WHAT THIS DOES NOT PROVE:
#   - Real home-router / mobile-network NAT behavior (this is nft
#     masquerade: endpoint-independent mapping). Other NAT types are
#     explicitly not covered.
#   - Relay fallback end to end (no relay server in this topology; the
#     relay path is covered by the wan_integration suites instead).
#
# STATUS: implemented. This script has NOT been executed on a Linux host
# with CAP_NET_ADMIN at the time of writing (developed on Windows). Run it
# before claiming any traversal result:
#
#   sudo scripts/test-mesh-nat.sh
#
# Prerequisites: Linux, root or CAP_NET_ADMIN, iproute2, nftables (or
# iptables), a working cargo toolchain.
#
set -euo pipefail

# ---------------------------------------------------------------- topology
NS_A="apeireth-pa"     # peer A
NS_NA="apeireth-na"    # NAT for A
NS_T="apeireth-tr"     # transit: observer / rendezvous
NS_NB="apeireth-nb"    # NAT for B
NS_B="apeireth-pb"     # peer B

VETH_A="veth-pa"; VETH_NA_IN="veth-na-in"
VETH_NA_OUT="veth-na-out"; VETH_T_A="veth-tr-a"
VETH_B="veth-pb"; VETH_NB_IN="veth-nb-in"
VETH_NB_OUT="veth-nb-out"; VETH_T_B="veth-tr-b"

A_IP="10.10.0.2";      A_GW="10.10.0.1"
B_IP="10.20.0.2";      B_GW="10.20.0.1"
NA_OUT_IP="192.0.2.10"
NB_OUT_IP="192.0.2.20"
T_IP="192.0.2.100"

OBSERVER_PORT="47000"
WORKDIR="$(mktemp -d)"
PUNCH_DIR="$WORKDIR/punch"          # TEST B/C/D presence exchange
HOSTILE_DIR="$WORKDIR/hostile"      # TEST E presence exchange
OBSERVER_PID=""
FW_BACKEND=""

log()  { printf '  %s\n' "$*"; }
die()  { printf 'ERROR: %s\n' "$*" >&2; exit 1; }

# ---------------------------------------------------------------- cleanup
cleanup() {
  local rc=$?
  set +e
  if [[ -n "$OBSERVER_PID" ]] && kill -0 "$OBSERVER_PID" 2>/dev/null; then
    kill "$OBSERVER_PID" 2>/dev/null
    wait "$OBSERVER_PID" 2>/dev/null
  fi
  # firewall state lives inside the namespaces, so deleting them removes it,
  # but clean explicitly first so a partially-built topology still tears down
  if [[ "$FW_BACKEND" == "nft" ]]; then
    ip netns exec "$NS_NA" nft delete table ip apeireth-nat 2>/dev/null
    ip netns exec "$NS_NB" nft delete table ip apeireth-nat 2>/dev/null
    ip netns exec "$NS_NA" nft delete table ip apeireth-hostile 2>/dev/null
  elif [[ "$FW_BACKEND" == "iptables" ]]; then
    ip netns exec "$NS_NA" iptables -t nat -F 2>/dev/null
    ip netns exec "$NS_NB" iptables -t nat -F 2>/dev/null
    ip netns exec "$NS_NA" iptables -F 2>/dev/null
  fi
  for ns in "$NS_A" "$NS_B" "$NS_NA" "$NS_NB" "$NS_T"; do
    ip netns del "$ns" 2>/dev/null
  done
  rm -rf "$WORKDIR"
  exit "$rc"
}
trap cleanup EXIT INT TERM

# ---------------------------------------------------------------- prerequisites
require() { command -v "$1" >/dev/null 2>&1 || die "missing required tool: $1"; }

[[ "$(uname -s)" == "Linux" ]] || die "this harness needs Linux (found $(uname -s))"
[[ "$(id -u)" -eq 0 ]] || die "needs root, or CAP_NET_ADMIN with mount of /var/run/netns"
require ip
require cargo
if command -v nft >/dev/null 2>&1; then
  FW_BACKEND=nft
elif command -v iptables >/dev/null 2>&1; then
  FW_BACKEND=iptables
else
  die "needs nft or iptables"
fi
ip netns list >/dev/null 2>&1 || die "ip netns is not usable here"

echo "== Apeireth Mesh NAT harness =="
log "firewall backend: $FW_BACKEND"

# ---------------------------------------------------------------- build
echo "== building =="
cargo build -q -p apeireth-mesh-quic --example nat_probe || die "example build failed"
PROBE="$(find target -type f -name 'nat_probe' -perm -u+x 2>/dev/null | head -n1)"
[[ -n "$PROBE" ]] || PROBE="$(find target -type f -name 'nat_probe.exe' 2>/dev/null | head -n1)"
[[ -n "$PROBE" ]] || die "could not locate the nat_probe example binary"
log "probe: $PROBE"

# ---------------------------------------------------------------- namespaces + links
echo "== topology =="
for ns in "$NS_A" "$NS_NA" "$NS_T" "$NS_NB" "$NS_B"; do ip netns add "$ns"; done

link_pair() { # <veth-a> <ns-a> <veth-b> <ns-b>
  ip link add "$1" type veth peer name "$3"
  ip link set "$1" netns "$2"
  ip link set "$3" netns "$4"
}

link_pair "$VETH_A" "$NS_A" "$VETH_NA_IN" "$NS_NA"
link_pair "$VETH_NA_OUT" "$NS_NA" "$VETH_T_A" "$NS_T"
link_pair "$VETH_B" "$NS_B" "$VETH_NB_IN" "$NS_NB"
link_pair "$VETH_NB_OUT" "$NS_NB" "$VETH_T_B" "$NS_T"

# peer A / nat A inside
ip -n "$NS_A" addr add "${A_IP}/24" dev "$VETH_A"
ip -n "$NS_A" link set "$VETH_A" up
ip -n "$NS_A" link set lo up
ip -n "$NS_A" route add default via "$A_GW"
ip -n "$NS_NA" addr add "${A_GW}/24" dev "$VETH_NA_IN"
ip -n "$NS_NA" link set "$VETH_NA_IN" up
ip -n "$NS_NA" link set lo up

# peer B / nat B inside
ip -n "$NS_B" addr add "${B_IP}/24" dev "$VETH_B"
ip -n "$NS_B" link set "$VETH_B" up
ip -n "$NS_B" link set lo up
ip -n "$NS_B" route add default via "$B_GW"
ip -n "$NS_NB" addr add "${B_GW}/24" dev "$VETH_NB_IN"
ip -n "$NS_NB" link set "$VETH_NB_IN" up
ip -n "$NS_NB" link set lo up

# transit
ip -n "$NS_T" addr add "${T_IP}/24" dev "$VETH_T_A"
ip -n "$NS_T" addr add "192.0.2.101/24" dev "$VETH_T_B"   # same wire, second leg
ip -n "$NS_T" link set "$VETH_T_A" up
ip -n "$NS_T" link set "$VETH_T_B" up
ip -n "$NS_T" link set lo up

# NAT outside legs
ip -n "$NS_NA" addr add "${NA_OUT_IP}/24" dev "$VETH_NA_OUT"
ip -n "$NS_NA" link set "$VETH_NA_OUT" up
ip -n "$NS_NA" route add default via "$T_IP"
ip -n "$NS_NB" addr add "${NB_OUT_IP}/24" dev "$VETH_NB_OUT"
ip -n "$NS_NB" link set "$VETH_NB_OUT" up
ip -n "$NS_NB" route add default via "${T_IP%%.*}.$((${T_IP##*.}))" 2>/dev/null || ip -n "$NS_NB" route add default via "192.0.2.101"

# forwarding inside the NAT namespaces
ip netns exec "$NS_NA" sysctl -qw net.ipv4.ip_forward=1
ip netns exec "$NS_NB" sysctl -qw net.ipv4.ip_forward=1

# ---------------------------------------------------------------- NAT: masquerade only
# Outbound traffic is SNATed so a peer's private address is never visible, and
# inbound traffic is *not* forwarded: a mapping exists only while outbound
# traffic created it. That is what makes this a NAT and not a router.
echo "== NAT ($FW_BACKEND, masquerade; no inbound forwarding) =="
if [[ "$FW_BACKEND" == "nft" ]]; then
  for ns in "$NS_NA" "$NS_NB"; do
    out="$VETH_NA_OUT"; [[ "$ns" == "$NS_NB" ]] && out="$VETH_NB_OUT"
    ip netns exec "$ns" nft add table ip apeireth-nat
    ip netns exec "$ns" nft add chain ip apeireth-nat postrouting \
      '{ type nat hook postrouting priority srcnat; policy accept; }'
    ip netns exec "$ns" nft add rule ip apeireth-nat postrouting \
      "oifname \"$out\" masquerade"
  done
else
  for ns in "$NS_NA" "$NS_NB"; do
    ip netns exec "$ns" iptables -t nat -A POSTROUTING -j MASQUERADE
  done
fi

# ---------------------------------------------------------------- proof: private path blocked
echo "== private path must be blocked =="
if ip netns exec "$NS_A" ping -c1 -W2 "$B_IP" >/dev/null 2>&1; then
  die "peer A reached peer B's private address ($B_IP): this is not a NAT topology"
fi
log "A ($A_IP) -> B ($B_IP): blocked"
if ip netns exec "$NS_B" ping -c1 -W2 "$A_IP" >/dev/null 2>&1; then
  die "peer B reached peer A's private address ($A_IP): this is not a NAT topology"
fi
log "B ($B_IP) -> A ($A_IP): blocked"

# ---------------------------------------------------------------- observer in transit
echo "== observer =="
ip netns exec "$NS_T" "$PROBE" observer "${T_IP}:${OBSERVER_PORT}" \
  > "$WORKDIR/observer.log" 2>&1 &
OBSERVER_PID=$!
for _ in $(seq 1 50); do
  grep -q '^OBSERVER=' "$WORKDIR/observer.log" 2>/dev/null && break
  sleep 0.1
done
grep -q '^OBSERVER=' "$WORKDIR/observer.log" || die "observer did not start (see $WORKDIR/observer.log)"
log "$(grep '^OBSERVER=' "$WORKDIR/observer.log")"

# ---------------------------------------------------------------- TEST A: observation
echo "== TEST A: endpoint observation =="
for spec in "a:$NS_A:$A_IP" "b:$NS_B:$B_IP"; do
  name="${spec%%:*}"; rest="${spec#*:}"; ns="${rest%%:*}"; ip="${rest##*:}"
  ip netns exec "$ns" "$PROBE" peer "${ip}:0" "${T_IP}:${OBSERVER_PORT}" \
    > "$WORKDIR/peer-$name.log" 2>&1 || true
  sed 's/^/  '"$name"' /' "$WORKDIR/peer-$name.log"
done

A_LOCAL="$(sed -n 's/^LOCAL=//p' "$WORKDIR/peer-a.log")"
A_OBS="$(sed -n 's/^OBSERVED=//p' "$WORKDIR/peer-a.log")"
A_QUIC="$(sed -n 's/^QUIC_LOCAL=//p' "$WORKDIR/peer-a.log")"
B_LOCAL="$(sed -n 's/^LOCAL=//p' "$WORKDIR/peer-b.log")"
B_OBS="$(sed -n 's/^OBSERVED=//p' "$WORKDIR/peer-b.log")"
B_QUIC="$(sed -n 's/^QUIC_LOCAL=//p' "$WORKDIR/peer-b.log")"

[[ -n "$A_OBS" && -n "$B_OBS" ]] || die "observation failed (see $WORKDIR/peer-*.log)"
[[ "$A_OBS" != "$A_LOCAL" ]] || die "peer A observed its own private address: no NAT mapping?"
[[ "$B_OBS" != "$B_LOCAL" ]] || die "peer B observed its own private address: no NAT mapping?"
[[ "$A_QUIC" == "$A_OBS" ]] || die "QUIC did not keep the observed port on A ($A_OBS vs $A_QUIC)"
[[ "$B_QUIC" == "$B_OBS" ]] || die "QUIC did not keep the observed port on B ($B_OBS vs $B_QUIC)"
log "TEST A: PASS (mapping exists; QUIC binds the observed socket)"

# ---------------------------------------------------------------- TEST B/C/D: punch E2E
# Both peers: observe -> sign presence with the server-reflexive candidate ->
# exchange -> MeshNode::connect_punched (simultaneous open over the one
# socket, pinned handshake, convergence) -> node-a runs a real remote
# tool.mesh.echo and node-b counts its executions.
echo "== TEST B/C/D: simultaneous open + convergence + remote invocation =="
mkdir -p "$PUNCH_DIR"
PUNCH_START="$(date +%s)"

ip netns exec "$NS_A" "$PROBE" punch "${A_IP}:0" "${T_IP}:${OBSERVER_PORT}" \
  "$PUNCH_DIR" node-a node-b > "$PUNCH_DIR/a.log" 2>&1 &
PUNCH_A_PID=$!
ip netns exec "$NS_B" "$PROBE" punch "${B_IP}:0" "${T_IP}:${OBSERVER_PORT}" \
  "$PUNCH_DIR" node-b node-a > "$PUNCH_DIR/b.log" 2>&1 &
PUNCH_B_PID=$!

PUNCH_A_RC=0; PUNCH_B_RC=0
wait "$PUNCH_A_PID" || PUNCH_A_RC=$?
wait "$PUNCH_B_PID" || PUNCH_B_RC=$?
PUNCH_ELAPSED=$(( $(date +%s) - PUNCH_START ))
sed 's/^/  a /' "$PUNCH_DIR/a.log"
sed 's/^/  b /' "$PUNCH_DIR/b.log"

grep -q '^PUNCH_RESULT=ok$'   "$PUNCH_DIR/a.log" || die "TEST B: peer A did not punch through (rc=$PUNCH_A_RC)"
grep -q '^PUNCH_RESULT=ok$'   "$PUNCH_DIR/b.log" || die "TEST B: peer B did not punch through (rc=$PUNCH_B_RC)"
grep -q '^LINK_COUNT=1$'      "$PUNCH_DIR/a.log" || die "TEST C: peer A does not hold exactly one link"
grep -q '^LINK_COUNT=1$'      "$PUNCH_DIR/b.log" || die "TEST C: peer B does not hold exactly one link"
grep -q '^PEER_STATE=Active$' "$PUNCH_DIR/a.log" || die "TEST B: peer A's link is not Active"
grep -q '^PEER_STATE=Active$' "$PUNCH_DIR/b.log" || die "TEST B: peer B's link is not Active"
grep -q '^E2E_RESULT=ok$'     "$PUNCH_DIR/a.log" || die "TEST D: node A's remote invocation failed"
grep -q '^E2E_RENDER={"e2e":"nat-harness-punch"}$' "$PUNCH_DIR/a.log" \
  || die "TEST D: remote result content is wrong (not the echoed arguments)"
grep -q '^E2E_EXEC_COUNT=1$'  "$PUNCH_DIR/b.log" \
  || die "TEST D: node B did not execute the capability exactly once"
log "TEST B: PASS (direct QUIC through both NATs, mutually authenticated)"
log "TEST C: PASS (one live logical link per side)"
log "TEST D: PASS (remote invocation content verified, exactly one execution)"

# ---------------------------------------------------------------- TEST E: bounded failure
# A NAT that drops ALL inbound: neither half of the punch can complete. The
# attempt must fail inside its bounded window on BOTH sides — this is a
# worst-case bound, not a model of any real NAT type (real NATs forward
# replies for mappings they created).
echo "== TEST E: bounded failure under a hostile inbound filter =="
if [[ "$FW_BACKEND" == "nft" ]]; then
  ip netns exec "$NS_NA" nft add table ip apeireth-hostile
  ip netns exec "$NS_NA" nft add chain ip apeireth-hostile forward \
    '{ type filter hook forward priority 0; policy drop; }'
  # A's own outbound is the only thing that gets through.
  ip netns exec "$NS_NA" nft add rule ip apeireth-hostile forward \
    iifname "$VETH_NA_IN" oifname "$VETH_NA_OUT" accept
else
  ip netns exec "$NS_NA" iptables -P FORWARD DROP
  ip netns exec "$NS_NA" iptables -A FORWARD -i "$VETH_NA_IN" -o "$VETH_NA_OUT" -j ACCEPT
fi
log "nat-a now drops all inbound traffic"

mkdir -p "$HOSTILE_DIR"
HOSTILE_START="$(date +%s)"
ip netns exec "$NS_A" "$PROBE" punch "${A_IP}:0" "${T_IP}:${OBSERVER_PORT}" \
  "$HOSTILE_DIR" node-a node-b > "$HOSTILE_DIR/a.log" 2>&1 &
HOSTILE_A_PID=$!
ip netns exec "$NS_B" "$PROBE" punch "${B_IP}:0" "${T_IP}:${OBSERVER_PORT}" \
  "$HOSTILE_DIR" node-b node-a > "$HOSTILE_DIR/b.log" 2>&1 &
HOSTILE_B_PID=$!

HOSTILE_A_RC=0; HOSTILE_B_RC=0
wait "$HOSTILE_A_PID" || HOSTILE_A_RC=$?
wait "$HOSTILE_B_PID" || HOSTILE_B_RC=$?
HOSTILE_ELAPSED=$(( $(date +%s) - HOSTILE_START ))
sed 's/^/  a /' "$HOSTILE_DIR/a.log"
sed 's/^/  b /' "$HOSTILE_DIR/b.log"

# Both sides must fail, and fail *bounded*: well inside the punch window
# (10s) plus the presence-exchange wait, not hang.
BOUND=$(( 12 + 32 ))
[[ "$HOSTILE_ELAPSED" -lt "$BOUND" ]] \
  || die "TEST E: the failed punch was not bounded (${HOSTILE_ELAPSED}s >= ${BOUND}s)"
grep -q '^PUNCH_RESULT=fail:punch:' "$HOSTILE_DIR/a.log" \
  || die "TEST E: peer A did not report a bounded punch failure"
grep -q '^PUNCH_RESULT=fail:punch:' "$HOSTILE_DIR/b.log" \
  || die "TEST E: peer B did not report a bounded punch failure"
log "TEST E: PASS (both sides failed inside ${HOSTILE_ELAPSED}s, bounded, no hang)"

# ---------------------------------------------------------------- evidence + verdict
echo
echo "== evidence =="
cat <<EVIDENCE
  topology:        peer-a -- nat-a(snat) -- transit -- nat-b(snat) -- peer-b
                   observer/rendezvous in transit; private paths blocked
                   (ping, both directions)
  A private:       $A_LOCAL
  A observed:      $A_OBS
  A quic local:    $A_QUIC
  B private:       $B_LOCAL
  B observed:      $B_OBS
  B quic local:    $B_QUIC

  TEST A observation:        PASS
  TEST B simultaneous open:  PASS (both peers punched through)
  TEST C convergence:        PASS (LINK_COUNT=1 on both sides)
  TEST D remote invocation:  PASS (result content verified; 1 execution)
  TEST E bounded failure:    PASS (hostile inbound filter; failed in ${HOSTILE_ELAPSED}s)

  punch path:       MeshNode::connect_punched (production entry point)
  transport:        one dual-role QUIC socket per peer (observed == dialed)
  authentication:   pinned mutual TLS (Ed25519 identity keys)

  NOT covered here:
    - real NAT device behavior (this is nft masquerade, endpoint-independent
      mapping; address/port-dependent variants are not simulated)
    - relay fallback end to end (no relay server in this topology; see the
      wan_integration suites)
    - reconnect after link loss (covered by loopback suites)
EVIDENCE
echo
echo "VERDICT: simulated-NAT E2E verified — direct QUIC + convergence + remote invocation across two SNATs."
echo "         This is evidence for L2 only; it says nothing about real routers or mobile networks."
