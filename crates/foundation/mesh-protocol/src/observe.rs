//! Server-reflexive endpoint observation — the "what address does the outside
//! world see me at?" half of NAT traversal.
//!
//! # Why this is not the rendezvous control channel
//!
//! The rendezvous control channel is TCP ([`crate::rendezvous`]). The source
//! address a server observes there says nothing about the *UDP* mapping a QUIC
//! endpoint will use: NATs map TCP and UDP independently, and a UDP mapping is
//! only opened by UDP traffic from that port. A node that wants to advertise a
//! candidate a peer can actually dial must therefore observe itself **over
//! UDP, from the same local socket it will dial from**.
//!
//! That is all this module is: one fixed-size probe, one fixed-size reply.
//!
//! ```text
//! probe (client -> observer, 64 bytes)
//!   [0..4)   magic   b"APOB"
//!   [4]      version
//!   [5..21)  nonce   (opaque, echoed back verbatim)
//!   [21..64) zero padding
//!
//! reply (observer -> client, 64 bytes)
//!   [0..4)   magic   b"APOB"
//!   [4]      version
//!   [5..21)  nonce   (the probe's, verbatim)
//!   [21]     family  4 | 6
//!   [22..24) port    big endian
//!   [24..40) address (IPv4 in the first four bytes when family = 4)
//!   [40..64) zero padding
//! ```
//!
//! # Properties
//!
//! - **Fixed size, both directions.** A 64-byte reply to a 64-byte probe is
//!   not an amplifier, so an observer cannot be turned into a reflection
//!   vector and a spoofed source address buys nothing.
//! - **Not an authority.** The observer reports the address it saw. That is a
//!   *hint*. It never grants trust and it cannot add, remove or alter a
//!   [`KeyFingerprint`](crate::KeyFingerprint): every candidate is still
//!   proven — or refused — by the pinned QUIC handshake before a link exists.
//! - **Nonce echo.** The reply carries the probe's nonce verbatim, so a client
//!   discards stale or off-path datagrams instead of adopting their address.
//! - **Pure data.** No sockets, no timers, no I/O: this crate stays
//!   transport-free and the caller owns every syscall.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

/// Datagram magic. A QUIC long-header first byte is `0xc0..=0xff` and a short
/// header is masked, so no QUIC packet can ever be mistaken for a probe.
pub const OBSERVE_MAGIC: [u8; 4] = *b"APOB";

/// Datagram layout version. A future layout change bumps this and observers
/// silently ignore what they do not understand.
pub const OBSERVE_VERSION: u8 = 1;

/// Probe nonce length. Random per probe; echoed back unchanged.
pub const OBSERVE_NONCE_BYTES: usize = 16;

/// Both datagrams are exactly this long. See the module docs for the layout.
pub const OBSERVE_DATAGRAM_BYTES: usize = 64;

const NONCE_OFFSET: usize = 5;
const FAMILY_OFFSET: usize = 21;
const PORT_OFFSET: usize = 22;
const ADDR_OFFSET: usize = 24;
const ADDR_BYTES: usize = 16;
const FAMILY_V4: u8 = 4;
const FAMILY_V6: u8 = 6;

/// Build a probe carrying `nonce`. Always [`OBSERVE_DATAGRAM_BYTES`] long.
pub fn encode_probe(nonce: &[u8; OBSERVE_NONCE_BYTES]) -> [u8; OBSERVE_DATAGRAM_BYTES] {
    let mut buf = [0u8; OBSERVE_DATAGRAM_BYTES];
    buf[0..4].copy_from_slice(&OBSERVE_MAGIC);
    buf[4] = OBSERVE_VERSION;
    buf[NONCE_OFFSET..NONCE_OFFSET + OBSERVE_NONCE_BYTES].copy_from_slice(nonce);
    buf
}

/// Parse a probe, returning its nonce.
///
/// `None` for anything that is not a well-formed probe: wrong length, magic or
/// version. An observer must answer only `Some`.
pub fn decode_probe(buf: &[u8]) -> Option<[u8; OBSERVE_NONCE_BYTES]> {
    if buf.len() != OBSERVE_DATAGRAM_BYTES
        || &buf[0..4] != OBSERVE_MAGIC.as_slice()
        || buf[4] != OBSERVE_VERSION
    {
        return None;
    }
    let mut nonce = [0u8; OBSERVE_NONCE_BYTES];
    nonce.copy_from_slice(&buf[NONCE_OFFSET..NONCE_OFFSET + OBSERVE_NONCE_BYTES]);
    Some(nonce)
}

/// Build the reply to a probe with `nonce`, reporting the `observed` source.
///
/// Always [`OBSERVE_DATAGRAM_BYTES`] long — never larger than the probe that
/// provoked it.
pub fn encode_reply(
    nonce: &[u8; OBSERVE_NONCE_BYTES],
    observed: SocketAddr,
) -> [u8; OBSERVE_DATAGRAM_BYTES] {
    let mut buf = [0u8; OBSERVE_DATAGRAM_BYTES];
    buf[0..4].copy_from_slice(&OBSERVE_MAGIC);
    buf[4] = OBSERVE_VERSION;
    buf[NONCE_OFFSET..NONCE_OFFSET + OBSERVE_NONCE_BYTES].copy_from_slice(nonce);
    match observed.ip() {
        IpAddr::V4(v4) => {
            buf[FAMILY_OFFSET] = FAMILY_V4;
            buf[ADDR_OFFSET..ADDR_OFFSET + 4].copy_from_slice(&v4.octets());
        }
        IpAddr::V6(v6) => {
            buf[FAMILY_OFFSET] = FAMILY_V6;
            buf[ADDR_OFFSET..ADDR_OFFSET + ADDR_BYTES].copy_from_slice(&v6.octets());
        }
    }
    buf[PORT_OFFSET..PORT_OFFSET + 2].copy_from_slice(&observed.port().to_be_bytes());
    buf
}

/// Parse a reply, returning `(nonce, observed address)`.
///
/// The caller **must** check that the nonce matches a probe it actually sent;
/// this function only guarantees the datagram is well formed.
pub fn decode_reply(buf: &[u8]) -> Option<([u8; OBSERVE_NONCE_BYTES], SocketAddr)> {
    if buf.len() != OBSERVE_DATAGRAM_BYTES
        || &buf[0..4] != OBSERVE_MAGIC.as_slice()
        || buf[4] != OBSERVE_VERSION
    {
        return None;
    }
    let mut nonce = [0u8; OBSERVE_NONCE_BYTES];
    nonce.copy_from_slice(&buf[NONCE_OFFSET..NONCE_OFFSET + OBSERVE_NONCE_BYTES]);
    let port = u16::from_be_bytes([buf[PORT_OFFSET], buf[PORT_OFFSET + 1]]);
    if port == 0 {
        return None;
    }
    let mut raw = [0u8; ADDR_BYTES];
    raw.copy_from_slice(&buf[ADDR_OFFSET..ADDR_OFFSET + ADDR_BYTES]);
    let ip = match buf[FAMILY_OFFSET] {
        FAMILY_V4 => IpAddr::V4(Ipv4Addr::new(raw[0], raw[1], raw[2], raw[3])),
        FAMILY_V6 => IpAddr::V6(Ipv6Addr::from(raw)),
        _ => return None,
    };
    Some((nonce, SocketAddr::new(ip, port)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nonce(seed: u8) -> [u8; OBSERVE_NONCE_BYTES] {
        [seed; OBSERVE_NONCE_BYTES]
    }

    #[test]
    fn a_probe_round_trips_its_nonce() {
        let probe = encode_probe(&nonce(7));
        assert_eq!(probe.len(), OBSERVE_DATAGRAM_BYTES);
        assert_eq!(decode_probe(&probe), Some(nonce(7)));
    }

    #[test]
    fn an_ipv4_reply_round_trips_the_observed_address() {
        let observed: SocketAddr = "203.0.113.9:51820".parse().unwrap();
        let reply = encode_reply(&nonce(3), observed);
        assert_eq!(reply.len(), OBSERVE_DATAGRAM_BYTES);
        assert_eq!(decode_reply(&reply), Some((nonce(3), observed)));
    }

    #[test]
    fn an_ipv6_reply_round_trips_the_observed_address() {
        let observed: SocketAddr = "[2001:db8::1]:51820".parse().unwrap();
        let reply = encode_reply(&nonce(9), observed);
        assert_eq!(decode_reply(&reply), Some((nonce(9), observed)));
    }

    #[test]
    fn a_reply_is_never_larger_than_the_probe_that_provoked_it() {
        let probe = encode_probe(&nonce(0));
        let reply = encode_reply(&nonce(0), "198.51.100.7:1".parse().unwrap());
        assert!(reply.len() <= probe.len());
    }

    #[test]
    fn garbage_is_never_a_valid_probe() {
        assert_eq!(decode_probe(&[]), None);
        assert_eq!(decode_probe(b"too short"), None);
        assert_eq!(decode_probe(&[0u8; OBSERVE_DATAGRAM_BYTES]), None);
        let mut wrong_version = encode_probe(&nonce(1));
        wrong_version[4] = OBSERVE_VERSION.wrapping_add(1);
        assert_eq!(decode_probe(&wrong_version), None);
    }

    #[test]
    fn garbage_is_never_a_valid_reply() {
        assert_eq!(decode_reply(&[]), None);
        assert_eq!(decode_reply(&[0u8; OBSERVE_DATAGRAM_BYTES]), None);
        let mut unknown_family = encode_reply(&nonce(1), "127.0.0.1:5".parse().unwrap());
        unknown_family[FAMILY_OFFSET] = 99;
        assert_eq!(decode_reply(&unknown_family), None);
        let mut zero_port = encode_reply(&nonce(2), "127.0.0.1:5".parse().unwrap());
        zero_port[PORT_OFFSET..PORT_OFFSET + 2].copy_from_slice(&0u16.to_be_bytes());
        assert_eq!(decode_reply(&zero_port), None);
    }

    #[test]
    fn a_quic_long_header_byte_is_not_mistaken_for_a_probe() {
        let mut buf = encode_probe(&nonce(4));
        buf[0] = 0xc3;
        assert_eq!(decode_probe(&buf), None);
    }
}
