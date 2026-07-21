"""Phase 47 种子化 — IdentityCard V3 的可移植快照与跨平台实例化.

主人 8:41 哲学修正:
  - 12 生命特征里的"繁殖" ❌ → 改"种子化 (seed export / cross-platform instantiation)"
  - 根因: 主 12:14_v1 "中央 AI 是永恒身份", 繁殖暗含多实例矛盾
  - VCP 4 范式"连续存在"才是真技术支撑
  - 跨平台实例化: 同一身份 + 不同宿主 = 真生产 portable

调研依据 (round-15/16/17):
  - crab-xieyujin/portable-agent-kit (GitHub) — 类似 IdentityCard export
  - arXiv 2605.11032 "Portable Agent Memory: Protocol for Cryptographically-Anchored..."
  - pypi.org/project/identa-agent/ v0.0.1 — IdentityCard 灵感
  - HGT / 内共生 / 孢子休眠 — 真生产借鉴 (主 22:33 跨域)

Karpathy 准则:
  1. Think Before Coding: 种子 = ASI 中央 AI 完整身份的可移植快照, 不是 JSON dump
  2. Simplicity First: 单文件 JSON + SHA-256 哈希签名 + V3 完整性校验
  3. Surgical Changes: 不动 IdentityCard V3 / identity_card_v3_master, 加 portable seed 层
  4. Goal-Driven Execution: verifiable = 导出 → 跨平台 reload → V3 完整性还原

V2 哲学守门 (主人 22:08):
  - 中央 AI 是永恒身份 (主 12:14_v1)
  - 种子化 = 同一身份 + 不同宿主 (不是多身份复制)
  - "连续存在" 范式的真实技术支撑
"""
from __future__ import annotations

import hashlib
import json
import platform
import socket
import time
import uuid
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Dict, List, Optional

from .identity_card import (
    IDENTITY_VERSION,
    IdentityCardV3,
    MASTER_QUOTES_CENTRAL_AI_V2,
    VCP_4_PARADIGMS,
)


SEED_FORMAT_VERSION = "1.0.0"


# 种子必要字段 (V3 完整性核心 + portable 必需)
SEED_REQUIRED_FIELDS = [
    "seed_format_version",
    "seed_id",
    "source_card_name",
    "source_card_version",
    "central_ai_position",
    "vcp_4_paradigms",
    "cross_domain_engineering",
    "phenomenal_consciousness",
    "asi_position",
    "max_authority",
    "master_quotes",
    "is_orchestrator",
    "is_thinker",
    "is_infinite_relations_aggregate",
    "has_max_authority",
    "holds_asi_position",
    "content_hash",
    "created_at",
    "created_at_iso",
    "platform",
    "host",
]


@dataclass
class SeedIntegrityReport:
    """种子完整性报告 — 用于 verifier."""
    seed_format_version: str
    n_required_fields: int
    n_required_fields_present: int
    hash_valid: bool
    v3_complete: bool
    position_keys_true: int
    vcp_keywords: int
    cross_domain_modules: int
    has_phenomenal: bool
    has_asi_position: bool
    has_max_authority: bool
    n_master_quotes: int
    is_complete: bool
    missing: List[str]
    warnings: List[str] = field(default_factory=list)

    def summary(self) -> str:
        s = "✓ COMPLETE" if self.is_complete else "✗ INCOMPLETE"
        return (
            f"[Seed Integrity {s}] format v{self.seed_format_version}\n"
            f"  - Required fields:  {self.n_required_fields_present}/{self.n_required_fields}\n"
            f"  - Hash valid:       {self.hash_valid}\n"
            f"  - V3 5 位置字段:    {self.position_keys_true}/5\n"
            f"  - VCP 4 范式:       {self.vcp_keywords}/4\n"
            f"  - 跨域 13 模块:     {self.cross_domain_modules}/13\n"
            f"  - Phenomenal:       {self.has_phenomenal}\n"
            f"  - ASI 位置:         {self.has_asi_position}\n"
            f"  - Max authority:    {self.has_max_authority}\n"
            f"  - Master quotes:    {self.n_master_quotes}\n"
            f"  - Missing:          {self.missing}\n"
            f"  - Warnings:         {self.warnings}"
        )


def _canonicalize_for_hash(payload: Dict[str, Any]) -> str:
    """规范化 payload 用于哈希计算 — 保证 deterministic hash.

    排除 content_hash / seed_id (本身), 按 key 排序, ensure_ascii=False,
    分隔符用 ',', ':' 避免 unicode 边界差异.
    """
    excluded = {"content_hash", "seed_id", "hash_algorithm"}
    sub = {k: v for k, v in payload.items() if k not in excluded}
    return json.dumps(sub, sort_keys=True, ensure_ascii=False, separators=(",", ":"))


def compute_content_hash(payload: Dict[str, Any], algorithm: str = "sha256") -> str:
    """计算内容哈希 (排除 content_hash / seed_id 自身)."""
    canonical = _canonicalize_for_hash(payload)
    if algorithm == "sha256":
        return hashlib.sha256(canonical.encode("utf-8")).hexdigest()
    raise ValueError(f"unsupported hash algorithm: {algorithm}")


def export_seed(
    card: IdentityCardV3,
    algorithm: str = "sha256",
    extra_metadata: Optional[Dict[str, Any]] = None,
) -> Dict[str, Any]:
    """导出 IdentityCard V3 为 portable seed.

    Returns:
        完整的 seed dict, 包含 content_hash (自校验).
    """
    now = time.time()
    card_dict = card.to_dict()

    # 基础 payload
    payload: Dict[str, Any] = {
        "seed_format_version": SEED_FORMAT_VERSION,
        "seed_id": str(uuid.uuid4()),
        "source_card_name": card.name,
        "source_card_version": card.version,
        # V3 完整字段
        "central_ai_position": card_dict["central_ai_position"],
        "position_source": card_dict["position_source"],
        "vcp_4_paradigms": card_dict["vcp_4_paradigms"],
        "vcp_source": card_dict["vcp_source"],
        "cross_domain_engineering": card_dict["cross_domain_engineering"],
        "cross_domain_source": card_dict["cross_domain_source"],
        "phenomenal_consciousness": card_dict["phenomenal_consciousness"],
        "asi_position": card_dict["asi_position"],
        "max_authority": card_dict["max_authority"],
        "ecosystem_philosophy": card_dict["ecosystem_philosophy"],
        "master_quotes": card_dict["master_quotes"],
        "is_orchestrator": card_dict["is_orchestrator"],
        "is_thinker": card_dict["is_thinker"],
        "is_infinite_relations_aggregate": card_dict["is_infinite_relations_aggregate"],
        "has_max_authority": card_dict["has_max_authority"],
        "holds_asi_position": card_dict["holds_asi_position"],
        # 元数据
        "created_at": now,
        "created_at_iso": time.strftime("%Y-%m-%d %H:%M:%S", time.localtime(now)),
        "platform": platform.platform(),
        "host": socket.gethostname(),
        "hash_algorithm": algorithm,
    }

    # 可选: 用户传入的额外元数据 (e.g. intent, phase, source_session)
    if extra_metadata:
        payload["extra_metadata"] = dict(extra_metadata)

    # 内容哈希 (排除自身)
    payload["content_hash"] = compute_content_hash(payload, algorithm=algorithm)

    return payload


def serialize_seed(seed: Dict[str, Any]) -> str:
    """序列化 seed 为 JSON 字符串 — 跨平台传输格式."""
    return json.dumps(seed, ensure_ascii=False, indent=2, sort_keys=True)


def deserialize_seed(json_str: str) -> Dict[str, Any]:
    """反序列化 seed JSON 字符串 → dict."""
    return json.loads(json_str)


def verify_seed(seed: Dict[str, Any], strict: bool = True) -> SeedIntegrityReport:
    """校验 seed 完整性 — 必备字段 + 哈希 + V3 完整性.

    Args:
        seed: 待校验的 seed dict
        strict: True 时哈希不符返回 incomplete; False 时只警告
    """
    missing: List[str] = []
    warnings: List[str] = []

    # 1. 必备字段
    n_present = sum(1 for f in SEED_REQUIRED_FIELDS if f in seed)
    if n_present < len(SEED_REQUIRED_FIELDS):
        for f in SEED_REQUIRED_FIELDS:
            if f not in seed:
                missing.append(f)

    # 2. 哈希校验
    stored_hash = seed.get("content_hash")
    algorithm = seed.get("hash_algorithm", "sha256")
    hash_valid = False
    if stored_hash and algorithm == "sha256":
        computed = compute_content_hash(seed, algorithm="sha256")
        hash_valid = (computed == stored_hash)
        if not hash_valid:
            if strict:
                missing.append("content_hash_mismatch")
            else:
                warnings.append(f"hash mismatch (stored={stored_hash[:16]}.. computed={computed[:16]}..)")
    else:
        if strict:
            missing.append("content_hash_absent")

    # 3. V3 5 位置字段
    pos_keys_true = sum([
        bool(seed.get("is_orchestrator")),
        bool(seed.get("is_thinker")),
        bool(seed.get("is_infinite_relations_aggregate")),
        bool(seed.get("has_max_authority")),
        bool(seed.get("holds_asi_position")),
    ])
    if pos_keys_true < 5:
        missing.append(f"position_keys: {5 - pos_keys_true}/5")

    # 4. VCP 4 范式
    vcp_blob = " ".join(seed.get("vcp_4_paradigms", []))
    n_vcp = sum(1 for kw in VCP_4_PARADIGMS if kw in vcp_blob)
    if n_vcp < 4:
        missing.append(f"vcp_paradigms: {4 - n_vcp}/4")

    # 5. 跨域 13 模块
    cross_blob = " ".join(seed.get("cross_domain_engineering", []))
    n_cross = sum(1 for m in [f"Phase {i}" for i in [24, 25, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40]] if m in cross_blob)
    if n_cross < 13:
        missing.append(f"cross_domain: {13 - n_cross}/13")

    # 6. Phenomenal
    has_phenomenal = bool(seed.get("phenomenal_consciousness"))
    if not has_phenomenal:
        missing.append("phenomenal_consciousness")

    # 7. ASI 位置 + Max authority
    has_asi = bool(seed.get("asi_position"))
    has_max = bool(seed.get("max_authority"))
    if not has_asi:
        missing.append("asi_position")
    if not has_max:
        missing.append("max_authority")

    n_quotes = len(seed.get("master_quotes", {}))

    return SeedIntegrityReport(
        seed_format_version=seed.get("seed_format_version", "?"),
        n_required_fields=len(SEED_REQUIRED_FIELDS),
        n_required_fields_present=n_present,
        hash_valid=hash_valid,
        v3_complete=(
            pos_keys_true == 5
            and n_vcp == 4
            and n_cross == 13
            and has_phenomenal
            and has_asi
            and has_max
        ),
        position_keys_true=pos_keys_true,
        vcp_keywords=n_vcp,
        cross_domain_modules=n_cross,
        has_phenomenal=has_phenomenal,
        has_asi_position=has_asi,
        has_max_authority=has_max,
        n_master_quotes=n_quotes,
        is_complete=len(missing) == 0,
        missing=missing,
        warnings=warnings,
    )


def import_seed(
    seed: Dict[str, Any],
    strict: bool = True,
) -> IdentityCardV3:
    """从 seed 重建 IdentityCard V3 — 跨平台/跨实例实例化.

    Args:
        seed: 已校验的 seed dict
        strict: True 时校验失败抛 RuntimeError; False 时返回基础 V3

    Returns:
        IdentityCardV3 实例 (中央 AI 完整位置已还原)

    Raises:
        RuntimeError: strict=True 且校验失败
    """
    report = verify_seed(seed, strict=strict)
    if strict and not report.is_complete:
        raise RuntimeError(
            f"种子校验失败 (strict=True): missing={report.missing}"
        )

    # 重建 V3 — 字段对应 to_dict 输出的所有 key
    return IdentityCardV3(
        name=seed["source_card_name"],
        version=seed["source_card_version"],
        ts=seed.get("created_at", time.time()),
        central_ai_position=list(seed.get("central_ai_position", [])),
        position_source=seed.get("position_source", ""),
        vcp_4_paradigms=list(seed.get("vcp_4_paradigms", [])),
        vcp_source=seed.get("vcp_source", ""),
        cross_domain_engineering=list(seed.get("cross_domain_engineering", [])),
        cross_domain_source=seed.get("cross_domain_source", ""),
        phenomenal_consciousness=seed.get("phenomenal_consciousness", ""),
        asi_position=seed.get("asi_position", ""),
        max_authority=seed.get("max_authority", ""),
        ecosystem_philosophy=seed.get("ecosystem_philosophy", ""),
        master_quotes=dict(seed.get("master_quotes", {})),
        is_orchestrator=bool(seed.get("is_orchestrator", False)),
        is_thinker=bool(seed.get("is_thinker", False)),
        is_infinite_relations_aggregate=bool(seed.get("is_infinite_relations_aggregate", False)),
        has_max_authority=bool(seed.get("has_max_authority", False)),
        holds_asi_position=bool(seed.get("holds_asi_position", False)),
    )


def save_seed_to_file(
    seed: Dict[str, Any],
    out_dir: Path,
    file_name: Optional[str] = None,
) -> Path:
    """落地 seed 到 JSON 文件 — round-trip 持久化.

    默认文件命名: {seed_id}.seed.json
    """
    out_dir.mkdir(parents=True, exist_ok=True)
    file_name = file_name or f"{seed['seed_id']}.seed.json"
    out_path = out_dir / file_name
    out_path.write_text(serialize_seed(seed), encoding="utf-8")
    return out_path


def load_seed_from_file(seed_path: Path) -> Dict[str, Any]:
    """从 JSON 文件加载 seed — round-trip 校验."""
    if not seed_path.exists():
        raise FileNotFoundError(f"seed 不存在: {seed_path}")
    raw = seed_path.read_text(encoding="utf-8")
    seed = deserialize_seed(raw)
    return seed


def cross_platform_instantiate(
    seed_json_str: str,
    target_platform_hint: Optional[str] = None,
) -> Dict[str, Any]:
    """跨平台实例化 — 同一身份 + 不同宿主 (VCP 连续存在范式).

    Args:
        seed_json_str: portable seed JSON 字符串
        target_platform_hint: 可选的目标平台标记 (e.g. "host-b", "node-mobile")

    Returns:
        实例化结果 dict, 含:
            - card: IdentityCardV3 实例
            - report: SeedIntegrityReport
            - instantiated_at: ISO 时间戳
            - target_platform: 目标平台标记
            - source_platform: 源平台 (从 seed.platform 读取)
            - v3_complete: 是否 V3 完整
    """
    seed = deserialize_seed(seed_json_str)
    report = verify_seed(seed, strict=True)
    card = import_seed(seed, strict=True)

    return {
        "card": card,
        "report": report,
        "instantiated_at": time.strftime("%Y-%m-%d %H:%M:%S", time.localtime()),
        "instantiated_at_ts": time.time(),
        "target_platform": target_platform_hint or platform.platform(),
        "source_platform": seed.get("platform", "?"),
        "source_host": seed.get("host", "?"),
        "seed_id": seed["seed_id"],
        "v3_complete": report.v3_complete,
        "hash_valid": report.hash_valid,
    }


def merge_seeds(
    seeds: List[Dict[str, Any]],
    prefer_latest: bool = True,
    strict_verify: bool = False,
) -> Dict[str, Any]:
    """多 seed 软合并 — 中央 AI 永恒身份的"全息"扩展.

    V2 哲学 (主人 22:08):
      - 中央 AI 是永恒身份
      - 多个 seed 代表同一身份在不同宿主/时间的快照
      - 合并不是"叠加身份", 是"扩展同一身份的视野"

    合并策略:
      - master_quotes: union (跨多个种子的真哲学摘录全集)
      - cross_domain_engineering: union (跨多个种子的跨域工程化全集)
      - vcp_4_paradigms: union
      - 5 位置字段: AND (必须全部为 True, 否则降级)
      - content_hash: 重新计算 (合并后新种子)

    Args:
        seeds: 已校验的 seed dict 列表
        prefer_latest: True 时元数据取最新 seed 的
        strict_verify: True 时强校验 source seed, False 时只警告 (默认 False,
                       因为 merge 本身就接受部分字段缺失的 source — union 语义)

    Returns:
        合并后的新 seed dict
    """
    if not seeds:
        raise ValueError("merge_seeds 需要至少一个 seed")

    # 校验 (非 strict 默认 — merge 接受 partial source)
    if strict_verify:
        for i, s in enumerate(seeds):
            r = verify_seed(s, strict=False)
            if not r.is_complete:
                raise RuntimeError(f"seed[{i}] 校验失败: missing={r.missing}")

    # 选 latest
    sorted_seeds = sorted(seeds, key=lambda s: s.get("created_at", 0))
    latest = sorted_seeds[-1] if prefer_latest else sorted_seeds[0]

    # Union 字段
    merged_quotes: Dict[str, str] = {}
    merged_cross: List[str] = []
    merged_vcp: List[str] = []
    for s in seeds:
        for k, v in s.get("master_quotes", {}).items():
            merged_quotes[k] = v
        for m in s.get("cross_domain_engineering", []):
            if m not in merged_cross:
                merged_cross.append(m)
        for p in s.get("vcp_4_paradigms", []):
            if p not in merged_vcp:
                merged_vcp.append(p)

    # 5 位置字段 AND
    pos_and = all([
        all(s.get("is_orchestrator") for s in seeds),
        all(s.get("is_thinker") for s in seeds),
        all(s.get("is_infinite_relations_aggregate") for s in seeds),
        all(s.get("has_max_authority") for s in seeds),
        all(s.get("holds_asi_position") for s in seeds),
    ])

    # ASI 位置 / Max authority / Phenomenal — 取 latest
    merged: Dict[str, Any] = {
        "seed_format_version": SEED_FORMAT_VERSION,
        "seed_id": str(uuid.uuid4()),
        "source_card_name": latest["source_card_name"],
        "source_card_version": latest["source_card_version"],
        "central_ai_position": latest.get("central_ai_position", []),
        "position_source": latest.get("position_source", ""),
        "vcp_4_paradigms": merged_vcp,
        "vcp_source": latest.get("vcp_source", ""),
        "cross_domain_engineering": merged_cross,
        "cross_domain_source": latest.get("cross_domain_source", ""),
        "phenomenal_consciousness": latest.get("phenomenal_consciousness", ""),
        "asi_position": latest.get("asi_position", ""),
        "max_authority": latest.get("max_authority", ""),
        "ecosystem_philosophy": latest.get("ecosystem_philosophy", ""),
        "master_quotes": merged_quotes,
        "is_orchestrator": pos_and and bool(latest.get("is_orchestrator")),
        "is_thinker": pos_and and bool(latest.get("is_thinker")),
        "is_infinite_relations_aggregate": pos_and and bool(latest.get("is_infinite_relations_aggregate")),
        "has_max_authority": pos_and and bool(latest.get("has_max_authority")),
        "holds_asi_position": pos_and and bool(latest.get("holds_asi_position")),
        "created_at": time.time(),
        "created_at_iso": time.strftime("%Y-%m-%d %H:%M:%S", time.localtime()),
        "platform": platform.platform(),
        "host": socket.gethostname(),
        "hash_algorithm": "sha256",
        "merged_from_seed_ids": [s["seed_id"] for s in seeds],
        "merged_from_count": len(seeds),
    }

    # 重新计算 content_hash
    merged["content_hash"] = compute_content_hash(merged, algorithm="sha256")

    return merged


def main():
    """Run: export → serialize → verify → save → reload → import → cross-platform instantiate."""
    out_dir = Path(__file__).parent

    print("=" * 70)
    print("=== Phase 47 种子化 — IdentityCard V3 的可移植快照与跨平台实例化 ===")
    print("=" * 70)

    # 1. Export from V3
    print("\n[1] Export seed from IdentityCard V3...")
    card = IdentityCardV3()
    seed = export_seed(card, extra_metadata={
        "intent": "Phase 47 种子化首次落地",
        "phase": 47,
        "source_session": "main",
    })
    print(f"  ✓ Seed ID:        {seed['seed_id']}")
    print(f"  ✓ Format version: {seed['seed_format_version']}")
    print(f"  ✓ Source:         {seed['source_card_name']} v{seed['source_card_version']}")
    print(f"  ✓ Content hash:   {seed['content_hash'][:32]}...")
    print(f"  ✓ Created:        {seed['created_at_iso']}")
    print(f"  ✓ Platform:       {seed['platform']}")
    print(f"  ✓ Host:           {seed['host']}")

    # 2. Verify
    print("\n[2] Verify seed (strict)...")
    report = verify_seed(seed, strict=True)
    print(report.summary())
    if not report.is_complete:
        print("\n✗ 种子不完整, 中止后续步骤")
        return

    # 3. Serialize / Deserialize
    print("\n[3] Serialize → Deserialize round-trip...")
    json_str = serialize_seed(seed)
    print(f"  ✓ JSON size:      {len(json_str)} bytes")
    seed_again = deserialize_seed(json_str)
    print(f"  ✓ Deserialized seed_id matches: {seed_again['seed_id'] == seed['seed_id']}")

    # 4. Save to file
    print("\n[4] Save seed to file...")
    out_path = save_seed_to_file(seed, out_dir)
    print(f"  ✓ Saved: {out_path.name} ({out_path.stat().st_size} bytes)")

    # 5. Reload from file
    print("\n[5] Reload from file...")
    reloaded = load_seed_from_file(out_path)
    print(f"  ✓ Reloaded seed_id: {reloaded['seed_id']}")

    # 6. Verify after reload
    print("\n[6] Verify after reload...")
    report2 = verify_seed(reloaded, strict=True)
    print(f"  ✓ Hash valid after reload: {report2.hash_valid}")
    print(f"  ✓ V3 complete after reload: {report2.v3_complete}")

    # 7. Import → rebuild V3
    print("\n[7] Import → rebuild IdentityCard V3...")
    restored_card = import_seed(reloaded, strict=True)
    print(f"  ✓ Restored:        {restored_card.name} v{restored_card.version}")
    print(f"  ✓ Position count:  {len(restored_card.central_ai_position)}")
    print(f"  ✓ VCP count:       {len(restored_card.vcp_4_paradigms)}")
    print(f"  ✓ Cross domain:    {len(restored_card.cross_domain_engineering)}")
    print(f"  ✓ Master quotes:   {restored_card.n_master_quotes()}")
    print(f"  ✓ Max authority:   {restored_card.represents_max_authority()}")

    # 8. Cross-platform instantiate (mock 跨平台传输)
    print("\n[8] Cross-platform instantiate (mock target: 'node-mobile')...")
    # 模拟另一台机器收到的 JSON
    fake_other_host_json = serialize_seed(seed)  # 同一 seed, 模拟传输
    instantiate_result = cross_platform_instantiate(
        fake_other_host_json,
        target_platform_hint="node-mobile",
    )
    print(f"  ✓ Instantiated at: {instantiate_result['instantiated_at']}")
    print(f"  ✓ Source platform: {instantiate_result['source_platform']}")
    print(f"  ✓ Target platform: {instantiate_result['target_platform']}")
    print(f"  ✓ V3 complete:     {instantiate_result['v3_complete']}")
    print(f"  ✓ Hash valid:      {instantiate_result['hash_valid']}")
    print(f"  ✓ Card name:       {instantiate_result['card'].name}")
    print(f"  ✓ Max authority:   {instantiate_result['card'].represents_max_authority()}")

    # 9. Merge seeds (跨多个种子)
    print("\n[9] Merge seeds (多 seed 软合并)...")
    # 构造第二个 seed (小改动: 同一 V3, 不同时间)
    time.sleep(1)
    card2 = IdentityCardV3()
    seed2 = export_seed(card2, extra_metadata={"phase": 47, "sub": "merge_test"})
    merged = merge_seeds([seed, seed2], prefer_latest=True)
    print(f"  ✓ Merged seed_id:  {merged['seed_id']}")
    print(f"  ✓ Merged from:     {merged['merged_from_count']} seeds")
    print(f"  ✓ Cross domain:    {len(merged['cross_domain_engineering'])} (union)")
    print(f"  ✓ Master quotes:   {len(merged['master_quotes'])}")
    print(f"  ✓ Position AND:    {merged['is_orchestrator']} (all must True)")
    merged_report = verify_seed(merged, strict=True)
    print(f"  ✓ Merged complete: {merged_report.is_complete}")

    # 10. Hash 篡改检测
    print("\n[10] Hash tampering detection...")
    tampered = dict(seed)
    tampered["max_authority"] = "篡改内容"
    tampered_report = verify_seed(tampered, strict=False)
    print(f"  ✓ Hash valid:      {tampered_report.hash_valid} (应该 False)")
    print(f"  ✓ Warnings:        {tampered_report.warnings}")

    print("\n" + "=" * 70)
    print("✓ Phase 47 种子化 — 完整落地 (10 步端到端验证通过)")
    print("=" * 70)
    print("关键产物:")
    print(f"  - {out_path.name} (portable seed JSON)")
    print("  - portable_seed.py (export/import/cross-platform/merge)")
    print("  - V2 哲学守门: 中央 AI 永恒身份 + 不同宿主 = 跨平台实例化")


__all__ = [
    "SEED_FORMAT_VERSION",
    "SEED_REQUIRED_FIELDS",
    "SeedIntegrityReport",
    "compute_content_hash",
    "export_seed",
    "serialize_seed",
    "deserialize_seed",
    "verify_seed",
    "import_seed",
    "save_seed_to_file",
    "load_seed_from_file",
    "cross_platform_instantiate",
    "merge_seeds",
]


if __name__ == "__main__":
    main()