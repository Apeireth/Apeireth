"""Prepare a reviewable cosign public-key document update.

The input is public material. This script never reads or generates a private key.
It validates the exact ECDSA P-256 SubjectPublicKeyInfo shape used by the release
contract, computes its DER fingerprint, and replaces stable marked sections so a
later rotation updates the existing record instead of appending a second trust root.
"""

from __future__ import annotations

import argparse
import base64
import binascii
from datetime import date
import hashlib
from pathlib import Path
import re


PUBLIC_BEGIN = "<!-- COSIGN_ACTIVE_PUBLIC_KEY_BEGIN -->"
PUBLIC_END = "<!-- COSIGN_ACTIVE_PUBLIC_KEY_END -->"
METADATA_BEGIN = "<!-- COSIGN_ACTIVE_METADATA_BEGIN -->"
METADATA_END = "<!-- COSIGN_ACTIVE_METADATA_END -->"

EC_P256_ALGORITHM = bytes.fromhex(
    "06072a8648ce3d0201"  # id-ecPublicKey
    "06082a8648ce3d030107"  # prime256v1 / secp256r1
)


def _read_tlv(data: bytes, offset: int, expected_tag: int) -> tuple[bytes, int]:
    if offset >= len(data) or data[offset] != expected_tag:
        raise ValueError("unexpected DER tag")
    offset += 1
    if offset >= len(data):
        raise ValueError("truncated DER length")
    first = data[offset]
    offset += 1
    if first < 0x80:
        length = first
    else:
        count = first & 0x7F
        if count == 0 or count > 4 or offset + count > len(data):
            raise ValueError("invalid DER length")
        encoded = data[offset : offset + count]
        if encoded[0] == 0:
            raise ValueError("non-canonical DER length")
        length = int.from_bytes(encoded, "big")
        offset += count
        if length < 0x80:
            raise ValueError("non-canonical DER length")
    end = offset + length
    if end > len(data):
        raise ValueError("truncated DER value")
    return data[offset:end], end


def parse_public_key(pem_text: str) -> tuple[str, bytes]:
    match = re.fullmatch(
        r"\s*-----BEGIN PUBLIC KEY-----\s*\n"
        r"(?P<body>[A-Za-z0-9+/=\r\n]+)"
        r"\n-----END PUBLIC KEY-----\s*",
        pem_text,
    )
    if not match:
        raise ValueError("expected one PUBLIC KEY PEM block")
    compact = re.sub(r"\s+", "", match.group("body"))
    try:
        der = base64.b64decode(compact, validate=True)
    except (binascii.Error, ValueError) as exc:
        raise ValueError("invalid PUBLIC KEY base64") from exc

    outer, outer_end = _read_tlv(der, 0, 0x30)
    if outer_end != len(der):
        raise ValueError("trailing data after SubjectPublicKeyInfo")
    algorithm, offset = _read_tlv(outer, 0, 0x30)
    if algorithm != EC_P256_ALGORITHM:
        raise ValueError("public key must be ECDSA P-256 SubjectPublicKeyInfo")
    point, offset = _read_tlv(outer, offset, 0x03)
    if offset != len(outer):
        raise ValueError("unexpected SubjectPublicKeyInfo fields")
    if len(point) != 66 or point[0] != 0 or point[1] != 0x04:
        raise ValueError("public key must contain one uncompressed P-256 point")
    field_prime = 0xFFFFFFFF00000001000000000000000000000000FFFFFFFFFFFFFFFFFFFFFFFF
    curve_b = 0x5AC635D8AA3A93E7B3EBBD55769886BC651D06B0CC53B0F63BCE3C3E27D2604B
    x = int.from_bytes(point[2:34], "big")
    y = int.from_bytes(point[34:66], "big")
    if x >= field_prime or y >= field_prime:
        raise ValueError("P-256 point coordinate is outside the field")
    if pow(y, 2, field_prime) != (pow(x, 3, field_prime) - 3 * x + curve_b) % field_prime:
        raise ValueError("public key point is not on P-256")

    wrapped = "\n".join(compact[index : index + 64] for index in range(0, len(compact), 64))
    canonical = f"-----BEGIN PUBLIC KEY-----\n{wrapped}\n-----END PUBLIC KEY-----"
    return canonical, der


def replace_marked(content: str, begin: str, end: str, body: str) -> str:
    if content.count(begin) != 1 or content.count(end) != 1:
        raise ValueError(f"document must contain exactly one {begin}/{end} marker pair")
    pattern = re.compile(re.escape(begin) + r".*?" + re.escape(end), re.DOTALL)
    replacement = f"{begin}\n{body.rstrip()}\n{end}"
    updated, count = pattern.subn(lambda _: replacement, content, count=1)
    if count != 1:
        raise ValueError(f"could not replace {begin}/{end} section")
    return updated


def update_document(pubkey_path: Path, document_path: Path, recorded_date: str) -> str:
    if date.fromisoformat(recorded_date).isoformat() != recorded_date:
        raise ValueError("recorded date must use YYYY-MM-DD")

    canonical_pem, der = parse_public_key(pubkey_path.read_text(encoding="ascii"))
    fingerprint = hashlib.sha256(der).hexdigest()
    content = document_path.read_text(encoding="utf-8")
    content = replace_marked(
        content,
        PUBLIC_BEGIN,
        PUBLIC_END,
        f"```pem\n{canonical_pem}\n```",
    )
    metadata = "\n".join(
        [
            "| 项 | 值 |",
            "|---|---|",
            f"| 登记日期（UTC） | `{recorded_date}` |",
            "| 算法 | `ECDSA P-256 / SubjectPublicKeyInfo` |",
            f"| Key ID（SHA-256 of DER SPKI） | `{fingerprint}` |",
            "| 状态 | `REVIEW REQUIRED — merge approval activates this candidate` |",
        ]
    )
    content = replace_marked(content, METADATA_BEGIN, METADATA_END, metadata)
    document_path.write_text(content, encoding="utf-8", newline="\n")
    return fingerprint


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--pubkey", type=Path, required=True)
    parser.add_argument("--document", type=Path, required=True)
    parser.add_argument("--recorded-date", required=True)
    args = parser.parse_args()

    fingerprint = update_document(args.pubkey, args.document, args.recorded_date)
    print(f"updated cosign public-key review record; fingerprint={fingerprint}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
