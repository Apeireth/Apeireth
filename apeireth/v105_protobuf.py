# V105 真生产 protocol buffers / 序列化
from __future__ import annotations
import uuid, json
from dataclasses import dataclass, field
V105_VERSION = "0.1.0"
@dataclass
class SerializedMessage:
    msg_id: str; schema: str; payload: str; format: str = "json"
    size_bytes: int = 0; ts: float = field(default_factory=lambda: __import__('time').time())
class V105Protobuf:
    def __init__(self): self.messages = []; self.n = 0; self.nph = 0; self.nas = 0
    def serialize(self, schema, payload, format="json"):
        mid = f"msg_{uuid.uuid4().hex[:12]}"
        if format == "json":
            serialized = json.dumps(payload) if not isinstance(payload, str) else payload
        else:
            serialized = str(payload)
        self.messages.append(SerializedMessage(
            msg_id=mid, schema=schema, payload=serialized,
            format=format, size_bytes=len(serialized)))
        self.n += 1
        return mid
    def total_bytes(self): return sum(m.size_bytes for m in self.messages)
    def stats(self): return {"n": self.n, "total_bytes": self.total_bytes(),
                             "version": V105_VERSION,
                             "philosophy": "V105 protobuf/序列化 (主 19:33 + 真借鉴)"}
__all__ = ["V105_VERSION", "V105Protobuf"]