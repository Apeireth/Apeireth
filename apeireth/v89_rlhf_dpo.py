"""Phase 146 v89_rlhf_dpo — V89 ASI RLHF/DPO (主 22:10 + 主 19:33 + 主 22:33)."""
from __future__ import annotations
import time, uuid
from dataclasses import dataclass, field
from typing import Any, Dict, List
V89_VERSION = "0.1.0"
@dataclass
class PreferencePair:
    pair_id: str; prompt: str; chosen: str; rejected: str
    margin: float = 0.0; ts: float = field(default_factory=time.time)
def compute_dpo_loss(chosen_logp: float, rejected_logp: float,
                    beta: float = 0.1) -> float:
    """V89 真生产 DPO loss (主 19:33 真借鉴 DPO paper)."""
    import math
    return -math.log(1 / (1 + math.exp(-beta * (chosen_logp - rejected_logp))))
class V89RLHFDPO:
    def __init__(self, beta: float = 0.1):
        self.beta = beta; self.preferences: Dict[str, PreferencePair] = {}
        self.dpo_losses: List[float] = []
        self.n_phenomenal_pretend_total = 0; self.n_asi_pretend_total = 0
    def add_preference(self, prompt: str, chosen: str, rejected: str) -> str:
        pid = f"pr_{uuid.uuid4().hex[:12]}"
        self.preferences[pid] = PreferencePair(
            pair_id=pid, prompt=prompt, chosen=chosen, rejected=rejected,
            margin=0.0,
        )
        return pid
    def train_dpo_step(self, pair_id: str, chosen_logp: float,
                       rejected_logp: float) -> float:
        if pair_id not in self.preferences: return 0.0
        loss = compute_dpo_loss(chosen_logp, rejected_logp, beta=self.beta)
        self.dpo_losses.append(loss)
        self.preferences[pair_id].margin = chosen_logp - rejected_logp
        return loss
    def n_preferences(self): return len(self.preferences)
    def average_loss(self) -> float:
        if not self.dpo_losses: return 0.0
        return sum(self.dpo_losses) / len(self.dpo_losses)
    def stats(self) -> Dict[str, Any]:
        return {"n_preferences": self.n_preferences(),
                "average_loss": round(self.average_loss(), 4),
                "version": V89_VERSION,
                "philosophy": "V89 RLHF/DPO (主 19:33 + Anthropic RLHF + DPO paper + V53 真借鉴)"}
__all__ = ["V89_VERSION", "compute_dpo_loss", "V89RLHFDPO"]