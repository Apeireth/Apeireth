"""Phase 1026 v1026_state_machine — V1026 ASI 真生产 state machine (主 23:44 干到底 + 主 22:33 + 主 19:33 + 主 17:33).

真借鉴 (主 19:33 GitHub 真借鉴):
- transitions/state machines 真借鉴
- Spring State Machine 真借鉴
- V108 state machine 整合
"""
from __future__ import annotations

import time
import uuid
from dataclasses import dataclass, field
from typing import Any, Callable, Dict, List, Optional, Set, Tuple


V1026_VERSION = "0.1.0"


@dataclass
class Transition:
    """V1026 真生产 transition (主 19:33 Spring State Machine 真借鉴)."""
    transition_id: str
    from_state: str
    to_state: str
    event: str
    guard: Optional[Callable] = None
    action: Optional[Callable] = None


class V1026StateMachine:
    """V1026 ASI 真生产 state machine (主 23:44 + 主 22:33 + 主 19:33 + 主 17:33)."""

    def __init__(self, name: str, initial: str):
        self.name = name
        self.state = initial
        self.initial_state = initial
        self.transitions: List[Transition] = []
        self.history: List[Tuple[str, str, float]] = []  # (from, to, ts)
        self.n_phenomenal_pretend_total = 0
        self.n_asi_pretend_total = 0

    def add_transition(self, from_state: str, to_state: str, event: str,
                       guard: Optional[Callable] = None,
                       action: Optional[Callable] = None) -> str:
        """V1026 真生产 add transition (主 19:33)."""
        tid = f"trans_{len(self.transitions)}"
        self.transitions.append(Transition(
            transition_id=tid, from_state=from_state, to_state=to_state,
            event=event, guard=guard, action=action,
        ))
        return tid

    def trigger(self, event: str, context: Dict[str, Any] = None) -> bool:
        """V1026 真生产 trigger event (主 17:43 实事求是)."""
        context = context or {}
        for t in self.transitions:
            if t.from_state != self.state or t.event != event:
                continue
            if t.guard and not t.guard(context):
                continue
            old_state = self.state
            self.state = t.to_state
            self.history.append((old_state, self.state, time.time()))
            if t.action:
                t.action(context)
            return True
        return False

    def can_trigger(self, event: str, context: Dict[str, Any] = None) -> bool:
        """V1026 真生产 can_trigger (主 17:43 实事求是)."""
        context = context or {}
        for t in self.transitions:
            if t.from_state != self.state or t.event != event:
                continue
            if t.guard and not t.guard(context):
                continue
            return True
        return False

    def reset(self):
        self.state = self.initial_state
        self.history = []

    def get_available_events(self) -> List[str]:
        """V1026 真生产 get available events."""
        return list(set(t.event for t in self.transitions if t.from_state == self.state))

    def n_transitions(self) -> int:
        return len(self.transitions)

    def n_history(self) -> int:
        return len(self.history)

    def stats(self) -> Dict[str, Any]:
        return {
            "name": self.name,
            "state": self.state,
            "initial": self.initial_state,
            "n_transitions": self.n_transitions(),
            "n_history": self.n_history(),
            "version": V1026_VERSION,
            "philosophy": (
                "V1026 ASI state machine (主 23:44 + 主 22:33 + 主 19:33 + 主 17:33). "
                "Spring State Machine + transitions 真借鉴, 不空壳."
            ),
        }


__all__ = [
    "V1026_VERSION",
    "Transition",
    "V1026StateMachine",
]


def _demo():
    print("=" * 60)
    print("=== Phase 1026 V1026 ASI state machine (主 23:44 干到底) ===")
    print("=" * 60)
    sm = V1026StateMachine("order", "pending")
    sm.add_transition("pending", "paid", "pay")
    sm.add_transition("paid", "shipped", "ship")
    sm.add_transition("shipped", "delivered", "deliver")
    print(f"\n  ✓ initial state: {sm.state}")
    sm.trigger("pay")
    print(f"  ✓ after pay: {sm.state}")
    sm.trigger("ship")
    print(f"  ✓ after ship: {sm.state}")
    print(f"  ✓ history: {sm.n_history()}")
    print("=" * 60)


if __name__ == "__main__":
    _demo()