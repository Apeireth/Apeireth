"""V1026 真生产 tests (主 23:44 干到底)."""
from __future__ import annotations
import sys; sys.path.insert(0, '.')
import pytest
from apeireth.v1026_state_machine import (
    V1026_VERSION, Transition, V1026StateMachine,
)


class TestV1026:
    def test_init(self):
        sm = V1026StateMachine("order", "pending")
        assert sm.state == "pending"
        assert sm.n_transitions() == 0

    def test_add_transition(self):
        """V1026 真测 Spring State Machine 真借鉴 (主 19:33)."""
        sm = V1026StateMachine("order", "pending")
        sm.add_transition("pending", "paid", "pay")
        assert sm.n_transitions() == 1

    def test_trigger_success(self):
        sm = V1026StateMachine("order", "pending")
        sm.add_transition("pending", "paid", "pay")
        assert sm.trigger("pay") is True
        assert sm.state == "paid"

    def test_trigger_no_transition(self):
        sm = V1026StateMachine("order", "pending")
        sm.add_transition("pending", "paid", "pay")
        assert sm.trigger("ship") is False  # 没有这个 transition
        assert sm.state == "pending"

    def test_trigger_wrong_state(self):
        sm = V1026StateMachine("order", "pending")
        sm.add_transition("paid", "shipped", "ship")
        assert sm.trigger("ship") is False  # 当前是 pending, 不是 paid

    def test_trigger_with_guard_pass(self):
        """V1026 真测 guard 真借鉴 (主 19:33)."""
        sm = V1026StateMachine("order", "pending")
        sm.add_transition("pending", "paid", "pay",
                          guard=lambda ctx: ctx.get("amount", 0) > 0)
        assert sm.trigger("pay", {"amount": 100}) is True

    def test_trigger_with_guard_fail(self):
        sm = V1026StateMachine("order", "pending")
        sm.add_transition("pending", "paid", "pay",
                          guard=lambda ctx: ctx.get("amount", 0) > 0)
        assert sm.trigger("pay", {"amount": 0}) is False
        assert sm.state == "pending"

    def test_trigger_with_action(self):
        """V1026 真测 action 真借鉴 (主 19:33)."""
        sm = V1026StateMachine("order", "pending")
        called = []
        sm.add_transition("pending", "paid", "pay", action=lambda ctx: called.append(ctx))
        sm.trigger("pay", {"amount": 100})
        assert called == [{"amount": 100}]

    def test_can_trigger(self):
        sm = V1026StateMachine("order", "pending")
        sm.add_transition("pending", "paid", "pay")
        assert sm.can_trigger("pay") is True
        assert sm.can_trigger("ship") is False

    def test_can_trigger_with_guard(self):
        sm = V1026StateMachine("order", "pending")
        sm.add_transition("pending", "paid", "pay",
                          guard=lambda ctx: ctx.get("ok", False))
        assert sm.can_trigger("pay") is False  # no ctx, guard fails
        assert sm.can_trigger("pay", {"ok": True}) is True

    def test_reset(self):
        sm = V1026StateMachine("order", "pending")
        sm.add_transition("pending", "paid", "pay")
        sm.trigger("pay")
        assert sm.state == "paid"
        sm.reset()
        assert sm.state == "pending"

    def test_history(self):
        sm = V1026StateMachine("order", "pending")
        sm.add_transition("pending", "paid", "pay")
        sm.add_transition("paid", "shipped", "ship")
        sm.trigger("pay")
        sm.trigger("ship")
        assert sm.n_history() == 2

    def test_get_available_events(self):
        sm = V1026StateMachine("order", "pending")
        sm.add_transition("pending", "paid", "pay")
        sm.add_transition("pending", "cancelled", "cancel")
        sm.add_transition("paid", "shipped", "ship")
        events = sm.get_available_events()
        assert "pay" in events
        assert "cancel" in events
        assert "ship" not in events

    def test_stats(self):
        sm = V1026StateMachine("order", "pending")
        sm.add_transition("pending", "paid", "pay")
        s = sm.stats()
        assert s["state"] == "pending"
        assert s["n_transitions"] == 1

    def test_v22_33_asi_integration(self):
        """V1026 真测主 22:33 ASI 北极星."""
        sm = V1026StateMachine("asi", "v1001")
        s = sm.stats()
        assert "ASI" in s["philosophy"]

    def test_v19_33_spring_state(self):
        """V1026 真测主 19:33 Spring State Machine 真借鉴."""
        sm = V1026StateMachine("order", "pending")
        sm.add_transition("pending", "paid", "pay")
        sm.add_transition("paid", "shipped", "ship")
        sm.add_transition("shipped", "delivered", "deliver")
        sm.trigger("pay")
        sm.trigger("ship")
        sm.trigger("deliver")
        assert sm.state == "delivered"
        assert sm.n_history() == 3

    def test_v17_43_truth(self):
        """V1026 真测主 17:43 实事求是 — 真 transition, 不假装."""
        sm = V1026StateMachine("order", "pending")
        sm.add_transition("pending", "paid", "pay")
        # 真实测: state 必须真改
        assert sm.trigger("pay") is True
        assert sm.state == "paid"
        # 真实测: history 真记录
        assert sm.n_history() == 1

    def test_complete_integration(self):
        """V1026 真测完整 state machine (主 23:44 + 主 22:33 + 主 19:33 + 主 17:33)."""
        sm = V1026StateMachine("asi_lifecycle", "v1001")
        sm.add_transition("v1001", "v1002", "evolve")
        sm.add_transition("v1002", "v1003", "evolve")
        sm.add_transition("v1003", "v1004", "evolve")
        sm.add_transition("v1004", "v1005", "evolve")
        sm.trigger("evolve")
        sm.trigger("evolve")
        sm.trigger("evolve")
        sm.trigger("evolve")
        assert sm.state == "v1005"
        assert sm.n_history() == 4