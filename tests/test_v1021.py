"""V1021 真生产 tests (主 23:44 干到底)."""
from __future__ import annotations
import sys; sys.path.insert(0, '.')
import pytest
from apeireth.v1021_message_queue import (
    V1021_VERSION, Message, Topic, V1021MessageQueue,
)


class TestV1021:
    def test_init(self):
        mq = V1021MessageQueue()
        assert mq.n_topics() == 0

    def test_create_topic(self):
        mq = V1021MessageQueue()
        t = mq.create_topic("events")
        assert t.name == "events"
        assert mq.n_topics() == 1

    def test_create_topic_existing(self):
        mq = V1021MessageQueue()
        mq.create_topic("events", partitions=2)
        mq.create_topic("events", partitions=5)  # 已存在, 返回原
        assert mq.topics["events"].partitions == 2

    def test_produce(self):
        mq = V1021MessageQueue()
        msg = mq.produce("events", "hello")
        assert msg.topic == "events"
        assert msg.payload == "hello"

    def test_produce_partition(self):
        """V1021 真测 Kafka partition 真借鉴 (主 19:33)."""
        mq = V1021MessageQueue()
        mq.create_topic("events", partitions=3)
        msg = mq.produce("events", "x", partition=2)
        assert msg.partition == 2

    def test_produce_invalid_partition(self):
        mq = V1021MessageQueue()
        mq.create_topic("events", partitions=2)
        with pytest.raises(ValueError):
            mq.produce("events", "x", partition=5)

    def test_produce_auto_create_topic(self):
        mq = V1021MessageQueue()
        mq.produce("new", "x")  # 自动创建
        assert "new" in mq.topics

    def test_consume(self):
        mq = V1021MessageQueue()
        mq.produce("events", "a")
        mq.produce("events", "b")
        consumed = mq.consume("events", partition=0, max_n=10)
        assert len(consumed) == 2
        assert consumed[0].payload == "a"

    def test_consume_max_n(self):
        mq = V1021MessageQueue()
        for i in range(5):
            mq.produce("events", str(i))
        consumed = mq.consume("events", max_n=3)
        assert len(consumed) == 3

    def test_consume_empty(self):
        mq = V1021MessageQueue()
        assert mq.consume("missing", partition=0) == []

    def test_consume_fifo(self):
        mq = V1021MessageQueue()
        mq.produce("events", "first")
        mq.produce("events", "second")
        consumed = mq.consume("events")
        assert consumed[0].payload == "first"
        assert consumed[1].payload == "second"

    def test_peek(self):
        """V1021 真测 RabbitMQ peek 真借鉴 (主 19:33)."""
        mq = V1021MessageQueue()
        mq.produce("events", "a")
        mq.produce("events", "b")
        peeked = mq.peek("events", n=2)
        assert len(peeked) == 2
        # peek 不消费
        assert mq.n_messages("events") == 2

    def test_peek_does_not_consume(self):
        mq = V1021MessageQueue()
        mq.produce("events", "x")
        mq.peek("events")
        assert mq.n_messages("events") == 1

    def test_n_messages_per_partition(self):
        mq = V1021MessageQueue()
        mq.create_topic("events", partitions=3)
        mq.produce("events", "a", partition=0)
        mq.produce("events", "b", partition=1)
        mq.produce("events", "c", partition=1)
        assert mq.n_messages("events") == 3

    def test_stats(self):
        mq = V1021MessageQueue()
        mq.create_topic("t1")
        mq.create_topic("t2")
        mq.produce("t1", "x")
        s = mq.stats()
        assert s["n_topics"] == 2
        assert s["topics"]["t1"] == 1

    def test_v22_33_asi_integration(self):
        """V1021 真测主 22:33 ASI 北极星."""
        mq = V1021MessageQueue()
        s = mq.stats()
        assert "ASI" in s["philosophy"]

    def test_v19_33_kafka_rabbitmq(self):
        """V1021 真测主 19:33 Kafka + RabbitMQ 真借鉴."""
        mq = V1021MessageQueue()
        # Kafka topic + partitions
        mq.create_topic("events", partitions=3)
        # produce to specific partition
        mq.produce("events", "x", partition=2)
        # RabbitMQ peek
        mq.produce("events", "y")
        peeked = mq.peek("events", n=10)
        assert len(peeked) >= 1

    def test_v17_33_real_mq(self):
        """V1021 真测主 17:33 放手干到底 — 真 MQ."""
        mq = V1021MessageQueue()
        mq.create_topic("asi_events", partitions=4)
        for i in range(10):
            mq.produce("asi_events", f"event_{i}", partition=i % 4)
        assert mq.n_messages("asi_events") == 10
        consumed = mq.consume("asi_events", partition=0, max_n=100)
        assert len(consumed) >= 1

    def test_complete_integration(self):
        """V1021 真测完整 MQ (主 23:44 + 主 22:33 + 主 19:33 + 主 17:33)."""
        mq = V1021MessageQueue()
        mq.create_topic("asi", partitions=2)
        mq.produce("asi", "v1001")
        mq.produce("asi", "v1002", partition=1)
        # peek default partition=0
        peeked = mq.peek("asi", partition=0, n=2)
        assert len(peeked) == 1
        assert peeked[0].payload == "v1001"
        # peek partition=1
        peeked_1 = mq.peek("asi", partition=1, n=2)
        assert len(peeked_1) == 1
        assert peeked_1[0].payload == "v1002"
        # consume partition=1
        consumed = mq.consume("asi", partition=1, max_n=10)
        assert len(consumed) == 1
        assert consumed[0].payload == "v1002"