"""Persistent single-user sessions for the Apeireth TUI."""
from __future__ import annotations

import json
import os
import re
import time
import uuid
from pathlib import Path
from typing import Any


SESSION_ID_RE = re.compile(r"^sess_[0-9a-f]{12}$")
DEFAULT_USER_ID = "apeireth-owner"


class SessionError(ValueError):
    """A session is missing or malformed."""


class SessionStore:
    """Atomic JSON session storage under ``memory/sessions``."""

    def __init__(self, root: str | Path | None = None, user_id: str = DEFAULT_USER_ID):
        configured = os.environ.get("APEIRETH_SESSIONS_DIR")
        project_root = Path(__file__).resolve().parent.parent
        self.root = Path(root or configured or project_root / "memory" / "sessions")
        self.root.mkdir(parents=True, exist_ok=True)
        self.user_id = user_id
        self.current: dict[str, Any] | None = None

    def _path(self, session_id: str) -> Path:
        if not SESSION_ID_RE.fullmatch(session_id):
            raise SessionError(f"invalid session id: {session_id}")
        return self.root / f"{session_id}.json"

    def create(self, model: str = "qwen-coder") -> dict[str, Any]:
        now = time.time()
        self.current = {
            "id": f"sess_{uuid.uuid4().hex[:12]}",
            "user_id": self.user_id,
            "created_at": now,
            "updated_at": now,
            "model": model,
            "messages": [],
        }
        self.flush()
        return self.current

    def load(self, session_id: str) -> dict[str, Any]:
        path = self._path(session_id)
        if not path.exists():
            raise SessionError(f"session not found: {session_id}")
        try:
            data = json.loads(path.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError) as exc:
            raise SessionError(f"cannot load {session_id}: {exc}") from exc
        if data.get("id") != session_id or not isinstance(data.get("messages"), list):
            raise SessionError(f"malformed session: {session_id}")
        data.setdefault("user_id", self.user_id)
        data.setdefault("model", "qwen-coder")
        self.current = data
        return data

    def list_recent(self, limit: int = 10) -> list[dict[str, Any]]:
        sessions: list[dict[str, Any]] = []
        for path in self.root.glob("sess_*.json"):
            try:
                data = json.loads(path.read_text(encoding="utf-8"))
                if data.get("id") and isinstance(data.get("messages"), list):
                    sessions.append(data)
            except (OSError, json.JSONDecodeError):
                continue
        sessions.sort(key=lambda item: float(item.get("updated_at", 0)), reverse=True)
        return sessions[:limit]

    def open_latest_or_create(self, model: str = "qwen-coder") -> dict[str, Any]:
        recent = self.list_recent(1)
        return self.load(recent[0]["id"]) if recent else self.create(model)

    def append_message(
        self,
        role: str,
        content: str,
        *,
        tokens: int = 0,
        latency_ms: float = 0.0,
        metadata: dict[str, Any] | None = None,
    ) -> dict[str, Any]:
        if self.current is None:
            raise SessionError("no active session")
        message = {
            "role": role,
            "content": str(content),
            "ts": time.time(),
            "tokens": max(0, int(tokens)),
            "latency_ms": round(max(0.0, float(latency_ms)), 2),
            "metadata": dict(metadata or {}),
        }
        self.current["messages"].append(message)
        self.current["updated_at"] = message["ts"]
        self.flush()
        return message

    def set_model(self, model: str) -> None:
        if self.current is None:
            raise SessionError("no active session")
        self.current["model"] = model
        self.current["updated_at"] = time.time()
        self.flush()

    def flush(self) -> None:
        if self.current is None:
            return
        path = self._path(self.current["id"])
        payload = json.dumps(self.current, ensure_ascii=False, indent=2)
        temporary = path.with_suffix(".json.tmp")
        temporary.write_text(payload, encoding="utf-8")
        temporary.replace(path)


__all__ = ["DEFAULT_USER_ID", "SessionError", "SessionStore"]
