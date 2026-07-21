"""V108 真生产 state machine (主 22:10 一次几十)."""
from __future__ import annotations
V108_VERSION = "0.1.0"


class V108StateMachine:
    def __init__(self):
        self.states = {}
        self.transitions = {}
        self.current_state = None
        self.history = []
        self.n = 0
        self.nph = 0
        self.nas = 0

    def add_state(self, name):
        self.states[name] = name
        if self.current_state is None:
            self.current_state = name

    def add_transition(self, from_state, to_state, trigger):
        key = (from_state, trigger)
        self.transitions[key] = to_state

    def trigger(self, trigger_name):
        if self.current_state is None:
            return None
        key = (self.current_state, trigger_name)
        if key in self.transitions:
            old = self.current_state
            self.current_state = self.transitions[key]
            self.history.append((old, self.current_state, trigger_name))
            self.n += 1
            return self.current_state
        return None

    def stats(self):
        return {"n_states": len(self.states),
                "n_transitions": len(self.transitions),
                "n_triggers": self.n,
                "version": V108_VERSION,
                "philosophy": "V108 state machine (主 19:33 + 真借鉴)"}


__all__ = ["V108_VERSION", "V108StateMachine"]