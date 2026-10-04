#!/usr/bin/env bash
#
# Apeireth Mesh — Linux namespace NAT harness.
#
#   peer-a -- nat-a -- transit -- nat-b -- peer-b
#                            |
#                    observer / rendezvous
#
# STATUS: implemented, NOT EXECUTED. This host is Windows. The observation half
# below is real and complete; the traversal half is deliberately marked TODO,
# because there is no punch driver wired to a pair of VerifiedPresence records
# yet. Running it today proves the topology, the blocked private path, and that
# an observed address comes from the socket QUIC will use -- it does NOT prove
# NAT traversal, and must never be reported as if it did.
#
# Prerequisites: Linux, root or CAP_NET_ADMIN, iproute2, nftables (or iptables),
# a working cargo toolchain.
#
# Usage:  sudo scripts/test-mesh-nat.sh
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
  elif [[ "$FW_BACKEND" == "iptables" ]]; then
    ip netns exec "$NS_NA" iptables -t nat -F 2>/dev/null
    ip netns exec "$NS_NB" iptables -t nat -F 2>/dev/null
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

# ---------------------------------------------------------------- observation from each peer
echo "== observation =="
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

# ---------------------------------------------------------------- TODO: traversal
echo
echo "== traversal: NOT IMPLEMENTED =="
cat <<'TODO'
  The steps below are deliberately absent, because there is no punch driver
  wired to a pair of VerifiedPresence records yet. Until they exist and run
  here, nothing may be reported as "NAT traversal validated".

    TODO punch session:   one coordination session bound to (A session, B session)
    TODO simultaneous dial: both peers dial the other's signed candidates from
                            the socket observed above, while both accept
    TODO convergence:      two authenticated QUIC connections resolve to one
                            logical Mesh connection on both ends
    TODO mesh request:     one real capability invocation A -> B, result checked
TODO
echo
echo "== evidence =="
cat <<EVIDENCE
  A private:      $A_LOCAL
  A observed:     $A_OBS
  A quic local:   $A_QUIC

  B private:      $B_LOCAL
  B observed:     $B_OBS
  B quic local:   $B_QUIC

  private reachability: blocked (checked with ping, both directions)
  punch session:  not implemented
  candidate path: not implemented
  authenticated peer identity: not implemented
  Mesh request:   not implemented

  VERDICT: topology + observation + observed-socket identity verified.
           NAT traversal NOT verified.
EVIDENCE
