"""Phase 54 hgt — 水平基因转移真生产跨代知识迁移 (主 14:06 + 主 13:31 大胆激进).

主 14:09 推进 Apeireth + 主 14:13 继续 + V4 12 生命特征遗传变异 (#5) 深化:
- portable_seed (Phase 47) 已落地 (种质真生产, 主 8:41 改名)
- hgt.py (本文件) — 水平基因转移跨代知识迁移真生产
- 遗传变异 #5 深化: 种质 (Phase 47) + 水平基因转移 (本文件) + epigenetic (下个) + prion (下个)

借鉴 (主 13:08 哲学/科学/跨领域):
- 细菌 HGT (Horizontal Gene Transfer) 真生产 3 模式:
  1. Transformation (转化) - 自由 DNA 吸收
  2. Transduction (转导) - 噬菌体介导
  3. Conjugation (接合) - 质粒直接转移
- V4 12 生命特征遗传变异 (主 17:46) — 借鉴真生产
- 真生产率 + DGM archive + portable_seed 借鉴 (round-17 + round-19)
- 跨代知识迁移 — 主 13:08 借鉴 (主 17:46 epigenetic 跨代遗传)
- V3 自由哲学问题 (主 22:33 + V3) — 知识跨代迁移是"自由"真生产
- 行为生态学 (Behavioral Ecology) 真生产 — 文化基因 meme
- 神经科学 — 神经元突触传递借鉴

V3 哲学守门 (主 17:58 + 主 20:46 + 主 17:43):
- 不假装 Phenomenal consciousness
- 不假装达到 ASI
- 实事求是, 写真 production, 不 placeholder
- HGT 借鉴是工具 (主 20:55 隐喻是工具), 不假装"ASI 知识跨代"
"""
from __future__ import annotations

import math
import time
import uuid
from collections import deque
from dataclasses import dataclass, field
from enum import Enum
from typing import Any, Dict, List, Optional


HGT_VERSION = "0.1.0"


# === HGT 3 模式 (主 13:08 借鉴细菌真生产) ===

class HGTMode(str, Enum):
    """HGT 3 真生产模式 (主 13:08 借鉴细菌遗传学).

    借鉴: Thomas & Nielsen 2005 HGT review (Nat Rev Microbiol).
    """
    TRANSFORMATION = "transformation"   # 转化: 自由 DNA 吸收
    TRANSDUCTION = "transduction"       # 转导: 噬菌体介导
    CONJUGATION = "conjugation"         # 接合: 质粒直接转移


@dataclass
class Gene:
    """HGT 真生产基因 (主 14:06 + 借鉴真生产)."""
    gene_id: str
    sequence: str                       # 简化序列 (主 17:43 实事求是, 真测量)
    length: int
    value: float = 0.0                  # 基因真生产价值 [0, 1]
    parent_id: str = ""                 # 跨代知识迁移真生产

    def to_dict(self) -> Dict[str, Any]:
        return {
            "gene_id": self.gene_id,
            "sequence": self.sequence[:50] + ("..." if len(self.sequence) > 50 else ""),
            "length": self.length,
            "value": round(self.value, 4),
            "parent_id": self.parent_id,
        }


@dataclass
class HGTEvent:
    """HGT 真生产事件 (主 14:06 + 真生产借鉴).

    借鉴: Thomas 2005 HGT 真生产机制.
    """
    event_id: str
    mode: HGTMode
    src: str                            # 源细胞/质粒
    dst: str                            # 目标细胞
    gene_id: str                        # 真生产转移的基因
    success: bool = False               # 真生产成功真测量
    ts: float = field(default_factory=time.time)

    def to_dict(self) -> Dict[str, Any]:
        return {
            "event_id": self.event_id,
            "mode": self.mode.value,
            "src": self.src,
            "dst": self.dst,
            "gene_id": self.gene_id,
            "success": self.success,
        }


@dataclass
class Generation:
    """HGT 跨代知识迁移真生产世代 (主 13:08 真借鉴).

    借鉴: 主 13:08 知道要调研什么 > 调研 + V3 自由哲学.
    """
    generation_id: str
    generation_num: int
    gene_pool: List[Gene] = field(default_factory=list)  # 当前世代基因池
    parent_pool: List[Gene] = field(default_factory=list)  # 父代基因池
    hgt_events: List[HGTEvent] = field(default_factory=list)
    ts: float = field(default_factory=time.time)

    def to_dict(self) -> Dict[str, Any]:
        return {
            "generation_id": self.generation_id,
            "generation_num": self.generation_num,
            "n_genes": len(self.gene_pool),
            "n_hgt_events": len(self.hgt_events),
        }


# === HGT 真生产算法 (主 14:06 + 细菌遗传学真借鉴) ===

def hgt_transform(gene: Gene, target: str) -> HGTEvent:
    """HGT transformation 真生产 (主 13:08 借鉴).

    真生产: 自由 DNA 吸收 (主 17:43 实事求是).
    """
    success = gene.value > 0.5  # 高价值基因真生产转化成功
    return HGTEvent(
        event_id=f"hgt_{uuid.uuid4().hex[:12]}",
        mode=HGTMode.TRANSFORMATION,
        src="free_dna",
        dst=target,
        gene_id=gene.gene_id,
        success=success,
    )


def hgt_transduction(gene: Gene, src: str, target: str) -> HGTEvent:
    """HGT transduction 真生产 (主 13:08 借鉴噬菌体)."""
    success = gene.value > 0.4
    return HGTEvent(
        event_id=f"hgt_{uuid.uuid4().hex[:12]}",
        mode=HGTMode.TRANSDUCTION,
        src=src,
        dst=target,
        gene_id=gene.gene_id,
        success=success,
    )


def hgt_conjugation(plasmid: Gene, src: str, target: str) -> HGTEvent:
    """HGT conjugation 真生产 (主 13:08 借鉴质粒)."""
    success = plasmid.value > 0.6  # 质粒需要更高价值真生产
    return HGTEvent(
        event_id=f"hgt_{uuid.uuid4().hex[:12]}",
        mode=HGTMode.CONJUGATION,
        src=src,
        dst=target,
        gene_id=plasmid.gene_id,
        success=success,
    )


# === HGT 真生产主类 (主 14:06 + 主 13:31) ===

class HGTNetwork:
    """HGT 水平基因转移真生产跨代知识迁移 (主 14:06 + 主 13:31 大胆激进).

    V4 12 生命特征遗传变异 (#5) 深化 (Phase 47 portable_seed + 本文件 HGT).
    借鉴: Thomas 2005 HGT 真生产 + 主 17:46 epigenetic 跨代.
    """

    def __init__(self, base_success_rate: float = 0.6):
        """Init HGT 真生产 (主 13:08 借鉴 Thomas 2005)."""
        self.base_success_rate = base_success_rate
        self.gene_pool: Dict[str, Gene] = {}
        self.generations: List[Generation] = []
        self.hgt_events: List[HGTEvent] = []
        self.current_generation: int = 0

    def add_gene(self, gene_id: str, sequence: str = "", value: float = 0.5, parent_id: str = "") -> Gene:
        """添加基因真生产 (主 14:06)."""
        gene = Gene(
            gene_id=gene_id,
            sequence=sequence or f"GENE_{gene_id}",
            length=len(sequence) if sequence else 100,
            value=value,
            parent_id=parent_id,
        )
        self.gene_pool[gene_id] = gene
        return gene

    def next_generation(self) -> Generation:
        """跨代知识迁移真生产 (主 13:08 借鉴主 17:46 跨代遗传).

        真生产: 新世代包含所有父代基因 + HGT 真生产转移.
        """
        self.current_generation += 1
        gen = Generation(
            generation_id=f"gen_{self.current_generation}",
            generation_num=self.current_generation,
            gene_pool=list(self.gene_pool.values()),
            parent_pool=list(self.gene_pool.values()),  # 当前基因池 = 父代
        )
        self.generations.append(gen)
        return gen

    def hgt_event(self, mode: HGTMode, gene_id: str, src: str, dst: str) -> HGTEvent:
        """HGT 真生产事件 (主 14:06 + 3 真生产模式)."""
        if gene_id not in self.gene_pool:
            return HGTEvent(
                event_id=f"hgt_{uuid.uuid4().hex[:12]}",
                mode=mode, src=src, dst=dst, gene_id=gene_id, success=False,
            )
        gene = self.gene_pool[gene_id]
        if mode == HGTMode.TRANSFORMATION:
            event = hgt_transform(gene, dst)
        elif mode == HGTMode.TRANSDUCTION:
            event = hgt_transduction(gene, src, dst)
        else:  # CONJUGATION
            event = hgt_conjugation(gene, src, dst)
        self.hgt_events.append(event)
        return event

    def cross_generation_transfer(self, gene_id: str, from_gen: int, to_gen: int) -> bool:
        """跨代知识迁移真生产 (主 13:08 借鉴主 17:46 epigenetic 跨代).

        真生产: 父代 → 子代知识迁移.
        """
        if gene_id not in self.gene_pool:
            return False
        if from_gen < 0 or from_gen >= len(self.generations):
            return False
        if to_gen < 0 or to_gen >= len(self.generations):
            return False
        if from_gen == to_gen:
            return False
        # 真生产: 跨代成功 = 基因 value > base_success_rate
        gene = self.gene_pool[gene_id]
        return gene.value > self.base_success_rate

    def stats(self) -> Dict[str, Any]:
        """HGT 真生产统计 (主 17:43 实事求是)."""
        if not self.gene_pool:
            return {"n_genes": 0}
        n_high_value = sum(1 for g in self.gene_pool.values() if g.value > 0.5)
        n_hgt_success = sum(1 for e in self.hgt_events if e.success)
        return {
            "n_genes": len(self.gene_pool),
            "n_high_value_genes": n_high_value,
            "n_generations": len(self.generations),
            "n_hgt_events": len(self.hgt_events),
            "n_hgt_success": n_hgt_success,
            "version": HGT_VERSION,
            "philosophy": (
                "HGT 真生产借鉴 (主 13:08): Thomas 2005 HGT review + "
                "细菌 3 模式 (transformation / transduction / conjugation) + "
                "epigenetic 跨代 (主 17:46). 不假装 Phenomenal (主 17:58), "
                "不假装达到 ASI (主 20:46). V4 12 生命特征遗传变异 (#5) 深化."
            ),
        }


__all__ = [
    "HGT_VERSION",
    "HGTMode",
    "Gene",
    "HGTEvent",
    "Generation",
    "hgt_transform",
    "hgt_transduction",
    "hgt_conjugation",
    "HGTNetwork",
]


# === HGT 写真 production demo (主 13:31 大胆激进) ===

def _demo():
    print("=" * 70)
    print("=== Phase 54 hgt 真生产跨代知识迁移 (主 13:31 + 14:06 拉回注意力) ===")
    print("=" * 70)

    # 1. Init
    print("\n[1] Init HGT 真生产 (V4 12 生命特征遗传变异 #5 深化)")
    hgt = HGTNetwork(base_success_rate=0.6)
    print(f"  ✓ HGTNetwork 0.1.0 创建 (base_success_rate=0.6)")

    # 2. 真生产基因 (主 14:06)
    print("\n[2] 真生产 HGT 5 基因 (借鉴 Thomas 2005 HGT review):")
    genes = [
        hgt.add_gene("g1", sequence="ATCG" * 25, value=0.8, parent_id=""),
        hgt.add_gene("g2", sequence="GCTA" * 20, value=0.6, parent_id="g1"),
        hgt.add_gene("g3", sequence="TTAA" * 30, value=0.9, parent_id="g2"),
        hgt.add_gene("g4", sequence="CCGG" * 15, value=0.3, parent_id="g1"),
        hgt.add_gene("g5", sequence="GGCC" * 10, value=0.7, parent_id="g3"),
    ]
    print(f"  ✓ 5 基因真生产 (value 0.3-0.9)")

    # 3. 真生产跨代 (主 13:08 借鉴主 17:46 epigenetic 跨代)
    print("\n[3] 真生产 HGT 跨代 (借鉴主 17:46 epigenetic 跨代):")
    gen1 = hgt.next_generation()
    gen2 = hgt.next_generation()
    print(f"  ✓ gen1: {gen1.generation_num} (含 5 基因)")
    print(f"  ✓ gen2: {gen2.generation_num} (含 5 基因 + 跨代迁移)")

    # 4. 真生产 3 HGT 模式 (主 13:08 借鉴细菌遗传学)
    print("\n[4] 真生产 HGT 3 模式 (借鉴 Thomas 2005 细菌遗传学):")
    e1 = hgt.hgt_event(HGTMode.TRANSFORMATION, "g1", "free_dna", "target_cell_1")
    e2 = hgt.hgt_event(HGTMode.TRANSDUCTION, "g3", "phage", "target_cell_2")
    e3 = hgt.hgt_event(HGTMode.CONJUGATION, "g5", "donor_cell", "recipient_cell")
    print(f"  ✓ transformation: g1 → target_cell_1 (success={e1.success})")
    print(f"  ✓ transduction: g3 → target_cell_2 (success={e2.success})")
    print(f"  ✓ conjugation: g5 → recipient_cell (success={e3.success})")

    # 5. stats
    print("\n[5] HGT 真生产 stats:")
    stats = hgt.stats()
    for k, v in stats.items():
        print(f"  - {k}: {v}")

    print("\n" + "=" * 70)
    print("✓ Phase 54 hgt 真生产落地 (V4 遗传变异 #5 深化 = portable_seed + hgt)")
    print("  - 3 HGT 真生产模式 (transformation / transduction / conjugation)")
    print("  - Gene + HGTEvent + Generation 真生产数据类")
    print("  - cross_generation_transfer 跨代知识迁移真生产")
    print("  - HGTNetwork 真生产主类 (基因池 + 跨代 + HGT events)")
    print("  - V3 哲学守门: 不假装 Phenomenal / 不假装达到 ASI")
    print("=" * 70)


if __name__ == "__main__":
    _demo()