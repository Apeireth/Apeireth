"""V106 真生产 event bus (主 22:10 一次几十 + 主 19:33 + 主 22:33)."""
from __future__ import annotations
import uuid, time
from dataclasses import dataclass, field
V106_VERSION = "0.1.0"

@dataclass
class Event:
    event_id: str
    topic: str
    payload: any = None
    timestamp: float = field(default_factory=time.time)


class EventHandler:
    def __init__(self):
        self.handlers = {}
        self.events = []
        self.n = 0
        self.nph = 0
        self.nas = 0

    def subscribe(self, topic, handler_fn):
        if topic not in self.handlers:
            self.handlers[topic] = []
        self.handlers[topic].append(handler_fn)

    def publish(self, topic, payload=None):
        eid = f"evt_{uuid.uuid4().hex[:12]}"
        event = Event(event_id=eid, topic=topic, payload=payload)
        self.events.append(event)
        self.n += 1
        if topic in self.handlers:
            for fn in self.handlers[topic]:
                fn(event)
        return eid

    def stats(self):
        return {"n_events": self.n, "n_topics": len(self.handlers),
                "version": V106_VERSION,
                "philosophy": "V106 event bus (主 19:33 + 真借鉴 pub/sub)"}


__all__ = ["V106_VERSION", "EventHandler"]