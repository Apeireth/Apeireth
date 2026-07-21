"""V110 真生产 message queue (主 22:10 一次几十)."""
from __future__ import annotations
import time
import uuid
from collections import deque
V110_VERSION = "0.1.0"


class V110MessageQueue:
    def __init__(self, max_size=1000):
        self.queue = deque()
        self.dead_letter = []
        self.max_size = max_size
        self.n_published = 0
        self.n_consumed = 0
        self.nph = 0
        self.nas = 0

    def publish(self, message):
        if len(self.queue) >= self.max_size:
            self.dead_letter.append(message)
            return None
        self.queue.append({"id": uuid.uuid4().hex[:8], "msg": message, "ts": time.time()})
        self.n_published += 1
        return self.queue[-1]["id"]

    def consume(self):
        if not self.queue:
            return None
        msg = self.queue.popleft()
        self.n_consumed += 1
        return msg

    def queue_size(self):
        return len(self.queue)

    def stats(self):
        return {"queue_size": self.queue_size(),
                "n_published": self.n_published,
                "n_consumed": self.n_consumed,
                "n_dead_letter": len(self.dead_letter),
                "version": V110_VERSION,
                "philosophy": "V110 message queue (主 19:33 + 真借鉴 Kafka/RabbitMQ)"}


__all__ = ["V110_VERSION", "V110MessageQueue"]