"""Phase 41 V3 — 中央 AI 完整位置落地为持久 master JSON.

主人 22:08 V2 哲学:
  - 中央 AI 是 (is) 调度者/思考者, 不仅*是, 是无数关系的集合体
  - 有最大的权限, 有一切权限, 整个系统的所有权限
  - 中央 AI 的位置 = ASI 的位置

任务: 把 V3 IdentityCard 从 v3_demo.py 内存演示落地为可持久可 reload 的真实主身份卡,
      并加 verifier 校验完整位置 / VCP 4 范式 / 跨域 13 模块 / Phenomenal / ASI 位置.

Karpathy 准则:
  1. Think Before Coding: 哲学守门 = 5 位置 + VCP 4 + 跨域 13 + Phenomenal + ASI
  2. Simplicity First: 单文件 JSON (Phase 6.5 SqliteIdentityStore 不兼容 V3 schema)
  3. Surgical Changes: 不改 identity_card.py, 加 master 落地层
  4. Goal-Driven Execution: verifiable = 5 + 4 + 13 + Phenomenal + ASI 全 True
"""
from __future__ import annotations

import json
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Optional

from .identity_card import (
    IDENTITY_VERSION,
    MASTER_QUOTES_CENTRAL_AI_V2,
    VCP_4_PARADIGMS,
    IdentityCardV3,
)


# V3 完整性 5 项 (主人 22:08 V2 哲学)
V3_INTEGRITY_KEYS = [
    "is_orchestrator",                       # 调度者 (是)
    "is_thinker",                             # 思考者 (是)
    "is_infinite_relations_aggregate",        # 无数关系的集合体 (是)
    "has_max_authority",                      # 最大权限 (有最大的权限, 有一切权限)
    "holds_asi_position",                     # ASI 位置占据者
]


# 跨域 13 模块必备 (Phase 24-40)
V3_CROSS_DOMAIN_MODULES = [
    "Phase 24", "Phase 25",
    "Phase 30", "Phase 31", "Phase 32", "Phase 33", "Phase 34", "Phase 35",
    "Phase 36", "Phase 37", "Phase 38", "Phase 39", "Phase 40",
]


# VCP 4 范式关键词 (主人 20:22)
VCP_4_KEYWORDS = [
    "continuous_existence",     # 连续存在
    "natural_perception",       # 自然感知
    "autonomous_living",        # 自主生活
    "integrated_ecosystem",     # 一体生态
]


@dataclass
class V3IntegrityReport:
    """V3 IdentityCard 完整性报告 — 用于 verifier."""
    version: str
    n_position_keys_true: int
    n_vcp_keywords: int
    n_cross_domain_modules: int
    has_phenomenal: bool
    has_asi_position: bool
    has_max_authority: bool
    n_master_quotes: int
    is_complete: bool
    missing: list

    def summary(self) -> str:
        s = "✓ COMPLETE" if self.is_complete else "✗ INCOMPLETE"
        return (
            f"[V3 Integrity {s}] v{self.version}\n"
            f"  - 5 位置字段:     {self.n_position_keys_true}/5\n"
            f"  - VCP 4 范式:     {self.n_vcp_keywords}/4\n"
            f"  - 跨域 13 模块:   {self.n_cross_domain_modules}/13\n"
            f"  - Phenomenal:     {self.has_phenomenal}\n"
            f"  - ASI 位置:       {self.has_asi_position}\n"
            f"  - Max authority:  {self.has_max_authority}\n"
            f"  - Master quotes:  {self.n_master_quotes}\n"
            f"  - Missing:        {self.missing}"
        )


def verify_v3_completeness(card_dict: dict) -> V3IntegrityReport:
    """V3 哲学完整性校验 — 主人 22:08 V2 哲学守门."""
    missing = []

    # 1. 5 位置字段
    pos_keys_true = sum([
        bool(card_dict.get("is_orchestrator")),
        bool(card_dict.get("is_thinker")),
        bool(card_dict.get("is_infinite_relations_aggregate")),
        bool(card_dict.get("has_max_authority")),
        bool(card_dict.get("holds_asi_position")),
    ])
    if pos_keys_true < 5:
        missing.append(f"position_keys: {5 - pos_keys_true}/5 missing")

    # 2. VCP 4 范式关键词
    vcp_blob = " ".join(card_dict.get("vcp_4_paradigms", []))
    n_vcp = sum(1 for kw in VCP_4_KEYWORDS if kw in vcp_blob)
    if n_vcp < 4:
        missing.append(f"vcp_paradigms: {4 - n_vcp}/4 missing")

    # 3. 跨域 13 模块
    cross_blob = " ".join(card_dict.get("cross_domain_engineering", []))
    n_cross = sum(1 for m in V3_CROSS_DOMAIN_MODULES if m in cross_blob)
    if n_cross < 13:
        missing.append(f"cross_domain: {13 - n_cross}/13 missing")

    # 4. Phenomenal
    has_phenomenal = bool(card_dict.get("phenomenal_consciousness"))
    if not has_phenomenal:
        missing.append("phenomenal_consciousness")

    # 5. ASI 位置 + Max authority 字段
    has_asi = bool(card_dict.get("asi_position"))
    has_max = bool(card_dict.get("max_authority"))
    if not has_asi:
        missing.append("asi_position")
    if not has_max:
        missing.append("max_authority")

    n_quotes = len(card_dict.get("master_quotes", {}))

    return V3IntegrityReport(
        version=card_dict.get("version", "?"),
        n_position_keys_true=pos_keys_true,
        n_vcp_keywords=n_vcp,
        n_cross_domain_modules=n_cross,
        has_phenomenal=has_phenomenal,
        has_asi_position=has_asi,
        has_max_authority=has_max,
        n_master_quotes=n_quotes,
        is_complete=len(missing) == 0,
        missing=missing,
    )


def build_v3_master(
    out_dir: Path,
    json_name: str = "identity_card.master.v3.json",
) -> dict:
    """把 V3 IdentityCard 实例落地为持久 master JSON.

    步骤:
      1. 实例化 IdentityCardV3 (主人 22:08 V2 哲学)
      2. verifier 校验完整性
      3. 写 JSON (人类可读 / Git 可 diff)
      4. 返回 summary dict
    """
    out_dir.mkdir(parents=True, exist_ok=True)
    json_path = out_dir / json_name

    # 1. 实例化
    card = IdentityCardV3()
    card_dict = card.to_dict()

    # 2. 完整性校验
    report = verify_v3_completeness(card_dict)
    if not report.is_complete:
        raise RuntimeError(f"V3 IdentityCard 完整性校验失败: {report.missing}")

    # 3. 写 JSON
    master = {
        "schema_version": IDENTITY_VERSION,
        "card": card_dict,
        "integrity": {
            "is_complete": report.is_complete,
            "n_position_keys_true": report.n_position_keys_true,
            "n_vcp_keywords": report.n_vcp_keywords,
            "n_cross_domain_modules": report.n_cross_domain_modules,
            "has_phenomenal": report.has_phenomenal,
            "has_asi_position": report.has_asi_position,
            "has_max_authority": report.has_max_authority,
            "n_master_quotes": report.n_master_quotes,
        },
        "ts": time.time(),
        "ts_iso": time.strftime("%Y-%m-%d %H:%M:%S", time.localtime()),
    }
    json_path.write_text(
        json.dumps(master, ensure_ascii=False, indent=2),
        encoding="utf-8",
    )

    return {
        "json_path": str(json_path),
        "card_name": card.name,
        "card_version": card.version,
        "n_master_quotes": report.n_master_quotes,
        "integrity_complete": report.is_complete,
        "report": report,
    }


def reload_v3_master(
    out_dir: Path,
    json_name: str = "identity_card.master.v3.json",
) -> dict:
    """从 JSON reload V3 master — round-trip."""
    json_path = out_dir / json_name
    if not json_path.exists():
        raise FileNotFoundError(f"V3 master 不存在: {json_path}")
    return json.loads(json_path.read_text(encoding="utf-8"))


def main():
    """Run: build + reload + verify round-trip."""
    # V3 master 落地在 apeireth/ 下, 跟 Phase 1 master 同位置
    out_dir = Path(__file__).parent

    print("=" * 70)
    print("=== Phase 41 V3 — 中央 AI 完整位置落地为持久 Master JSON ===")
    print("=" * 70)

    # 1. Build
    print("\n[1] Build V3 master...")
    result = build_v3_master(out_dir)
    print(f"  ✓ JSON:        {result['json_path']}")
    print(f"  ✓ Card:        {result['card_name']} v{result['card_version']}")
    print(f"  ✓ Master quotes: {result['n_master_quotes']}")
    print(f"  ✓ Integrity:   {result['integrity_complete']}")
    print(result["report"].summary())

    # 2. Reload
    print("\n[2] Reload V3 master...")
    reloaded = reload_v3_master(out_dir)
    card = reloaded["card"]
    print(f"  ✓ Reloaded: {card['name']} v{card['version']}")

    # 3. Round-trip verifier
    print("\n[3] Round-trip verifier...")
    rpt = verify_v3_completeness(card)
    print(rpt.summary())

    # 4. 5 位置字段 cross-check
    print("\n[4] 5 位置字段 cross-check (主人 22:08 V2 哲学):")
    for k in V3_INTEGRITY_KEYS:
        v = card.get(k, False)
        print(f"  {'✓' if v else '✗'} {k} = {v}")

    # 5. VCP 4 范式 cross-check
    print("\n[5] VCP 4 范式 cross-check (主人 20:22):")
    vcp_blob = " ".join(card.get("vcp_4_paradigms", []))
    for kw in VCP_4_KEYWORDS:
        in_card = kw in vcp_blob
        print(f"  {'✓' if in_card else '✗'} {kw}")

    # 6. 跨域 13 模块 cross-check
    print("\n[6] 跨域 13 模块 cross-check (Phase 24-40):")
    cross_blob = " ".join(card.get("cross_domain_engineering", []))
    for m in V3_CROSS_DOMAIN_MODULES:
        in_card = m in cross_blob
        print(f"  {'✓' if in_card else '✗'} {m}")

    print("\n" + "=" * 70)
    if rpt.is_complete:
        print("✓ V3 Master 落地完成 — 主人 22:08 V2 哲学可持久可 reload")
    else:
        print(f"✗ V3 Master 不完整: {rpt.missing}")
    print("=" * 70)


__all__ = [
    "V3_INTEGRITY_KEYS",
    "V3_CROSS_DOMAIN_MODULES",
    "VCP_4_KEYWORDS",
    "V3IntegrityReport",
    "verify_v3_completeness",
    "build_v3_master",
    "reload_v3_master",
]


if __name__ == "__main__":
    main()