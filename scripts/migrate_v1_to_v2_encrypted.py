#!/usr/bin/env python3
"""RC-11 v1 -> v2 加密文件迁移脚本 (Python)

真生产前必写 (per file_encrypted.rs:45-48 TODO + 子代理 F P1 #2 + 子代理 G ID_LEN_MAX).

**v1 格式** (RC-10 早期, commit `e2a5be08`):
    [sealed_len: u32 BE 4 bytes][sealed: N bytes]
    sealed 内部 = [iv (12)][ciphertext (N-12-16)][tag (16)]
    AAD v1 = service|type (简化, 无 record_id)

**v2 格式** (RC-10 line header, commit `38cc1039`):
    [id_len: u16 BE 2 bytes][id: id_len UTF-8][sealed_len: u32 BE 4 bytes][sealed: N bytes]
    AAD v2 = service|type|record_id (per-record tamper 保护, 子代理 C 建议 #5)

**关键**:
- v1 era 没有 record_id, 脚本**必为每条 record 生成 v2 record_id**:
  使用 UUID v5 (deterministic, from `uuid.NAMESPACE_DNS` + sha256(JSON content + index))
  → 相同 input 重跑迁移生成相同 id (idempotent, 接手人可重跑).
- 0 装诚实: 若 record_id 字节 > 65535 (ID_LEN_MAX), script reject + exit 1.
  不 silent truncation (子代理 G 独立判断, 2026-08-27).

**依赖**: Python 3.8+, `cryptography` (pip install cryptography).
可选 `uuid` (内置 stdlib).

**用法**:
    python scripts/migrate_v1_to_v2_encrypted.py \\
        --input  <v1_file.bin> \\
        --output <v2_file.bin> \\
        --master-key <32 bytes hex> \\
        --service <service_name> \\
        --type <record_type>

**0 装诚实标注** (子代理 D 教的 0 装原则):
- 假设 v1 输入**纯** old format, 不尝试识别 (true 0 装, 不假装"magic detect").
- master key 由调用方提供 (真生产从 KeyringSelector 取, dev 显式 hex).
- 跑前**先备份** v1 (脚本 0 删 v1, 调用方负责 backup).
- 跑后 v2 用 v2 `EncryptedFileBackend` 验 (建议另写 verify 脚本).
- 限制: 单 record_type per file (per line header `{record_type}.enc`).
"""

from __future__ import annotations

import argparse
import hashlib
import logging
import sys
import uuid
from pathlib import Path

from cryptography.hazmat.primitives.ciphers.aead import AESGCM

# 子代理 G 独立判断 (2026-08-27): migration script 必校验 id 长度 ≤ 65535 bytes
# (= `Self::ID_LEN_MAX` = u16 BE). 超出 reject + exit 1, 不 silent truncation.
ID_LEN_MAX = 65535

# AES-256-GCM 加密参数 (与 file_encrypted.rs:91-96 常量一致).
KEY_LEN = 32
IV_LEN = 12
TAG_LEN = 16

log = logging.getLogger("migrate_v1_to_v2")


def _parse_v1_records(data: bytes) -> list[bytes]:
    """Parse v1 binary: 循环读 `[sealed_len: 4 BE][sealed: N]`.

    Raises:
        ValueError: truncated input or sealed_len 超文件剩余.
    """
    records: list[bytes] = []
    pos = 0
    while pos < len(data):
        if pos + 4 > len(data):
            raise ValueError(
                f"truncated v1 input at pos {pos}: expected 4 bytes sealed_len, got {len(data) - pos}"
            )
        sealed_len = int.from_bytes(data[pos : pos + 4], byteorder="big", signed=False)
        pos += 4
        if sealed_len < IV_LEN + TAG_LEN:
            raise ValueError(
                f"v1 sealed_len too small at pos {pos - 4}: {sealed_len} bytes, min {IV_LEN + TAG_LEN}"
            )
        if pos + sealed_len > len(data):
            raise ValueError(
                f"truncated v1 sealed at pos {pos}: expected {sealed_len} bytes, got {len(data) - pos}"
            )
        sealed = data[pos : pos + sealed_len]
        pos += sealed_len
        records.append(sealed)
    return records


def _open_v1_sealed(
    key: bytes, service: str, record_type: str, sealed: bytes
) -> bytes:
    """解密 v1 sealed bytes (AAD = service|type, 无 record_id)."""
    if len(sealed) < IV_LEN + TAG_LEN:
        raise ValueError(
            f"sealed too short: {len(sealed)} bytes, min {IV_LEN + TAG_LEN}"
        )
    iv = sealed[:IV_LEN]
    ct_with_tag = sealed[IV_LEN:]
    aad = f"{service}|{record_type}".encode("utf-8")
    aesgcm = AESGCM(key)
    plaintext = aesgcm.decrypt(iv, ct_with_tag, aad)
    return plaintext


def _seal_v2(
    key: bytes, service: str, record_type: str, record_id: str, plaintext: bytes
) -> bytes:
    """Encrypt v2: AAD = service|type|record_id, 返 [iv 12][ct][tag 16].

    Raises:
        ValueError: record_id 字节长度超 ID_LEN_MAX.
    """
    id_bytes = record_id.encode("utf-8")
    if len(id_bytes) > ID_LEN_MAX:
        raise ValueError(
            f"record_id too long: {len(id_bytes)} bytes, max {ID_LEN_MAX}"
        )
    iv = uuid.uuid4().bytes[:IV_LEN]  # 12 bytes random (per-record IV)
    aad = f"{service}|{record_type}|{record_id}".encode("utf-8")
    aesgcm = AESGCM(key)
    ct_with_tag = aesgcm.encrypt(iv, plaintext, aad)
    out = iv + ct_with_tag
    return out


def _generate_record_id(plaintext: bytes, index: int, namespace: uuid.UUID) -> str:
    """Deterministic UUID v5 record_id from plaintext content + index.

    0 装诚实: 用 uuid v5 (deterministic) 而非 v4 (random) → 重跑迁移生成相同 id.
    Namespace 用 `uuid.NAMESPACE_DNS` (well-known stdlib 常量).
    """
    # hash 包含 plaintext + index 防相同 content 两条 record 撞 id
    h = hashlib.sha256()
    h.update(plaintext)
    h.update(index.to_bytes(8, byteorder="big", signed=True))
    digest = h.digest()
    # uuid v5 expects 16-byte digest
    return str(uuid.uuid5(namespace, digest.hex()))


def migrate_file(
    v1_path: Path,
    v2_path: Path,
    master_key: bytes,
    service: str,
    record_type: str,
    dry_run: bool = False,
) -> int:
    """读 v1 加密文件 → 解密 v1 → 生成 record_id → 重签 v2 → 写 v2.

    Args:
        v1_path: 老 v1 加密文件路径.
        v2_path: 新 v2 加密文件输出路径.
        master_key: 32 bytes master key.
        service: AAD service 字段 (e.g. "apeireth").
        record_type: AAD type 字段 (e.g. "episodes").
        dry_run: True 仅 log + 0 写.

    Returns:
        0 = 全部 records 迁移成功, 1 = 有 errors (callers 用作 exit code).

    Raises:
        ValueError: master key 长度错, v1 解析错, ID_LEN_MAX 边界违.
    """
    if len(master_key) != KEY_LEN:
        raise ValueError(
            f"master_key must be {KEY_LEN} bytes, got {len(master_key)}"
        )

    log.info("reading v1 file: %s (%d bytes)", v1_path, v1_path.stat().st_size)
    v1_data = v1_path.read_bytes()

    if not v1_data:
        log.warning("v1 file empty, writing empty v2 file")
        if not dry_run:
            v2_path.write_bytes(b"")
        return 0

    # Parse v1 records
    try:
        v1_records = _parse_v1_records(v1_data)
    except ValueError as e:
        log.error("v1 parse failed: %s", e)
        return 1

    log.info("v1 records parsed: %d", len(v1_records))

    # Decrypt v1 + generate record_id + re-seal v2
    v2_bytes = bytearray()
    namespace = uuid.NAMESPACE_DNS
    skipped = 0
    errors = 0

    for idx, sealed_v1 in enumerate(v1_records):
        try:
            plaintext = _open_v1_sealed(master_key, service, record_type, sealed_v1)
        except Exception as e:
            log.error(
                "record %d: v1 decrypt failed (AAD mismatch or corrupt): %s", idx, e
            )
            errors += 1
            continue

        record_id = _generate_record_id(plaintext, idx, namespace)
        id_bytes = record_id.encode("utf-8")
        if len(id_bytes) > ID_LEN_MAX:
            # 子代理 G 独立判断: 必 reject, 不 silent truncation
            log.error(
                "record %d: generated record_id too long: %d bytes > ID_LEN_MAX %d",
                idx,
                len(id_bytes),
                ID_LEN_MAX,
            )
            errors += 1
            continue

        try:
            sealed_v2 = _seal_v2(master_key, service, record_type, record_id, plaintext)
        except Exception as e:
            log.error("record %d: v2 seal failed: %s", idx, e)
            errors += 1
            continue

        # v2 frame: [id_len: 2 BE][id][sealed_len: 4 BE][sealed]
        id_len = len(id_bytes).to_bytes(2, byteorder="big", signed=False)
        sealed_len = len(sealed_v2).to_bytes(4, byteorder="big", signed=False)
        v2_bytes.extend(id_len)
        v2_bytes.extend(id_bytes)
        v2_bytes.extend(sealed_len)
        v2_bytes.extend(sealed_v2)

    if errors > 0:
        log.error("migration had %d errors, aborting write", errors)
        return 1

    log.info(
        "migration ok: %d records -> %d bytes (skipped %d)",
        len(v1_records),
        len(v2_bytes),
        skipped,
    )

    if not dry_run:
        v2_path.parent.mkdir(parents=True, exist_ok=True)
        v2_path.write_bytes(bytes(v2_bytes))
        log.info("wrote v2 file: %s (%d bytes)", v2_path, len(v2_bytes))

    return 0


def _parse_master_key(hex_str: str) -> bytes:
    """Parse master key from 64-char hex string → 32 bytes."""
    s = hex_str.strip()
    if s.startswith("0x") or s.startswith("0X"):
        s = s[2:]
    raw = bytes.fromhex(s)
    if len(raw) != KEY_LEN:
        raise ValueError(
            f"master_key hex must decode to {KEY_LEN} bytes, got {len(raw)}"
        )
    return raw


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="RC-11 v1 -> v2 加密文件迁移 (AES-256-GCM, per-record AAD tamper 保护)"
    )
    parser.add_argument("--input", required=True, type=Path, help="v1 加密文件路径")
    parser.add_argument("--output", required=True, type=Path, help="v2 加密文件输出路径")
    parser.add_argument(
        "--master-key",
        required=True,
        help="32-byte master key as 64-char hex (真生产从 KeyringSelector 拿, dev 显式 hex)",
    )
    parser.add_argument(
        "--service",
        required=True,
        help="AAD service 字段 (e.g. 'apeireth')",
    )
    parser.add_argument(
        "--type",
        dest="record_type",
        required=True,
        help="AAD record_type 字段 (e.g. 'episodes', 'thought_stream', ...)",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="仅 log + 校验, 0 写 v2",
    )
    parser.add_argument(
        "--verbose",
        "-v",
        action="store_true",
        help="DEBUG level logging",
    )

    args = parser.parse_args(argv)

    logging.basicConfig(
        level=logging.DEBUG if args.verbose else logging.INFO,
        format="%(asctime)s %(levelname)s %(name)s: %(message)s",
    )

    try:
        master_key = _parse_master_key(args.master_key)
    except ValueError as e:
        log.error("invalid master_key: %s", e)
        return 1

    if not args.input.exists():
        log.error("input file does not exist: %s", args.input)
        return 1

    return migrate_file(
        v1_path=args.input,
        v2_path=args.output,
        master_key=master_key,
        service=args.service,
        record_type=args.record_type,
        dry_run=args.dry_run,
    )


if __name__ == "__main__":
    sys.exit(main())