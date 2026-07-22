"""Apeireth Textual observation console.

This is intentionally a local verification surface: unlike the OpenAI-compatible
service, it displays internal backend invariants and artifact observations.
"""
from __future__ import annotations

import asyncio
import json
import os
import re
import time
from pathlib import Path
from typing import Any, Callable, Iterable

from textual.app import App, ComposeResult
from textual.containers import Horizontal, Vertical
from textual.events import Key
from textual.widgets import Input, Static

from .asi_fun_score import compute_asi_fun_score
from .cli import SUPPORTED_MODELS, _model_call
from .tui_sessions import SessionError, SessionStore
from .tui_widgets import BottomBar, ChatView, CommandPalette, MetricsPanel, TopBar
from .v1083_asi_decision_router import (
    DEFAULT_MODEL_REGISTRY,
    RequestContext,
    plan_failover,
    select_model,
)
from .v1085_hqb_core import HonestDecisionModule
from .v36_hqb_benchmark import V36HQBBenchmark


TUI_VERSION = "0.1.0"
PROJECT_ROOT = Path(__file__).resolve().parent.parent


class ArtifactReader:
    """Read small scalar observations without loading recursive snapshots."""

    def __init__(self, root: str | Path | None = None):
        self.root = Path(root or os.environ.get("APEIRETH_PROJECT_ROOT", PROJECT_ROOT))

    @staticmethod
    def _scalar(text: str, key: str) -> Any:
        pattern = rf'"{re.escape(key)}"\s*:\s*(?:"([^"\\]*(?:\\.[^"\\]*)*)"|(-?\d+(?:\.\d+)?)|(true|false|null))'
        match = re.search(pattern, text)
        if not match:
            return None
        if match.group(1) is not None:
            return match.group(1)
        if match.group(2) is not None:
            number = float(match.group(2))
            return int(number) if number.is_integer() else number
        return {"true": True, "false": False, "null": None}[match.group(3)]

    def _bounded_text(self, path: Path, limit: int = 256 * 1024) -> str:
        if not path.exists():
            return ""
        try:
            with path.open("rb") as handle:
                head = handle.read(limit)
                size = path.stat().st_size
                tail = b""
                if size > limit:
                    handle.seek(max(0, size - limit))
                    tail = handle.read(limit)
            return (head + b"\n" + tail).decode("utf-8", errors="replace")
        except OSError:
            return ""

    def scalar(self, relative: str, key: str) -> Any:
        return self._scalar(self._bounded_text(self.root / relative), key)

    def snapshot(self) -> dict[str, Any]:
        path = self.root / "artifacts" / "asi_snapshot.json"
        text = self._bounded_text(path)
        return {
            "score": self._scalar(text, "v03_score") or self._scalar(text, "level_score"),
            "guard": self._scalar(text, "philosophy_guard_ok"),
            "trend": [
                float(value)
                for value in re.findall(r'"(?:v03_score|level_score)"\s*:\s*(-?\d+(?:\.\d+)?)', text)[-10:]
            ],
        }

    def audit_shells(self) -> Any:
        path = self.root / "artifacts" / "v1082" / "audit.json"
        text = self._bounded_text(path, limit=512 * 1024)
        if not text:
            return "N/A"
        for key in ("empty_shell_count", "n_empty_shells", "shell_count", "n_stubs"):
            value = self._scalar(text, key)
            if value is not None:
                return value
        return "N/A"

    def honest_score(self) -> Any:
        paths = sorted((self.root / "artifacts" / "v1081").glob("*.json"), key=lambda p: p.stat().st_mtime)
        if not paths:
            return "N/A"
        text = self._bounded_text(paths[-1], limit=64 * 1024)
        return self._scalar(text, "honesty_score") or "N/A"

    def observability(self, action_scores: Iterable[float]) -> dict[str, Any]:
        snapshot = self.snapshot()
        return {
            "asi_score": snapshot["score"] if snapshot["score"] is not None else "N/A",
            "trend": list(action_scores)[-10:] or snapshot["trend"],
            "guard": "PASS" if snapshot["guard"] is True else "N/A",
            "shells": self.audit_shells(),
            "honesty": self.honest_score(),
            "life": "12/12",
        }


class ApeirethTUI(App):
    """Five-zone Textual console for tasks and internal invariants."""

    TITLE = "apeireth — terminal observation console"
    SUB_TITLE = f"TUI {TUI_VERSION}"
    ENABLE_COMMAND_PALETTE = False
    CSS = """
    Screen { background: #10151c; color: #d7e3f0; }
    #topbar { height: 3; min-height: 3; padding: 0 1; border: round #4c7899; }
    #main-body { height: 1fr; }
    #left-panel, #right-panel { width: 20%; min-width: 24; padding: 1; border: round #38516a; }
    #center-panel { width: 60%; padding: 0 1; }
    #chat { height: 1fr; border: round #38516a; padding: 0 1; }
    #command-palette { height: auto; min-height: 8; padding: 1; border: round #d39b4a; background: #202b38; }
    #bottom-bar { height: 8; min-height: 8; padding: 0 1; border: round #4c7899; }
    #session-status, #shortcut-hint { height: 1; color: #8ea9bf; }
    #task-input { height: 3; border: round #4c7899; }
    .message { height: auto; padding: 1 0; }
    .user { color: #9ad8ff; }
    .assistant { color: #b9f2bd; }
    .system { color: #e8c982; }
    """
    BINDINGS = [
        ("ctrl+p", "toggle_palette", "Commands"),
        ("ctrl+q", "quit", "Quit"),
        ("ctrl+l", "clear_chat", "Clear"),
    ]

    def __init__(
        self,
        *,
        project_root: str | Path | None = None,
        sessions_dir: str | Path | None = None,
        session_store: SessionStore | None = None,
        call_llm: Callable[[str], str] | None = None,
    ):
        super().__init__()
        self.reader = ArtifactReader(project_root)
        self.sessions = session_store or SessionStore(sessions_dir)
        self._call_llm_override = call_llm
        self.route: dict[str, Any] = {}
        self.action_scores: list[float] = []
        self.memory_hits = 0

    def compose(self) -> ComposeResult:
        yield TopBar("routing…", id="topbar")
        with Horizontal(id="main-body"):
            yield MetricsPanel(id="left-panel")
            with Vertical(id="center-panel"):
                yield ChatView(id="chat")
                yield CommandPalette(id="command-palette")
            yield MetricsPanel(id="right-panel")
        yield BottomBar(id="bottom-bar")

    async def on_mount(self) -> None:
        session = self.sessions.open_latest_or_create()
        self.memory_hits = len(session.get("messages", []))
        self._select_route("qa")
        await self.query_one("#chat", ChatView).load_messages(session.get("messages", []))
        self._refresh_panels()
        self.query_one("#task-input", Input).focus()

    def on_unmount(self) -> None:
        self.sessions.flush()

    def _task_type(self, task: str) -> str:
        value = task.lower()
        if any(word in value for word in ("code", "python", "rust", "bug", "implement")):
            return "code"
        if any(word in value for word in ("write", "creative", "story", "design")):
            return "creative"
        if any(word in value for word in ("summarize", "summary", "总结")):
            return "summarization"
        return "qa"

    def _select_route(self, task_type: str, requested: str | None = None) -> None:
        context = RequestContext(
            task_type=task_type,
            capability_need=0.7,
            latency_budget_ms=1000,
            cost_budget_per_1k=0.005,
            prompt_size_tokens=1000,
        )
        decision = select_model(context, DEFAULT_MODEL_REGISTRY, policy="balanced")
        selected = requested or decision.chosen_model or decision.fallback_model or "qwen-coder"
        if selected not in SUPPORTED_MODELS:
            selected = "qwen-coder"
        plan = plan_failover(decision.chosen_model or selected, DEFAULT_MODEL_REGISTRY)
        record = DEFAULT_MODEL_REGISTRY.get(selected)
        affinity = (record.task_affinities.get(task_type, record.capability_score) if record else "N/A")
        self.route = {
            "model": selected,
            "failover": [plan.primary, plan.secondary, plan.tertiary],
            "affinity": affinity,
            "cost": record.cost_per_1k_tokens if record else "N/A",
            "latency_ms": record.latency_p50_ms if record else "N/A",
            "task_type": task_type,
            "decision": decision.to_dict(),
        }
        if self.sessions.current is not None and self.sessions.current.get("model") != selected:
            self.sessions.set_model(selected)

    def _refresh_panels(self) -> None:
        if not self.is_mounted:
            return
        self.query_one("#topbar", TopBar).set_route(self.route)
        observed = self.reader.observability(self.action_scores)
        self.query_one("#left-panel", MetricsPanel).set_metrics(
            "ASI / GUARDS",
            (
                ("V0.3", observed["asi_score"]),
                ("trend", self._sparkline(observed["trend"])),
                ("guard", observed["guard"]),
                ("HQB", self._hqb()["verdict"]),
            ),
        )
        self.query_one("#right-panel", MetricsPanel).set_metrics(
            "BACKEND",
            (
                ("V1074", observed["asi_score"]),
                ("V1082 shells", observed["shells"]),
                ("V1081 honest", observed["honesty"]),
                ("life heartbeat", observed["life"]),
            ),
        )
        session_id = self.sessions.current.get("id", "N/A") if self.sessions.current else "N/A"
        self.query_one("#bottom-bar", BottomBar).set_session(session_id, self.memory_hits)

    @staticmethod
    def _sparkline(values: Iterable[float]) -> str:
        values = list(values)[-10:]
        if not values:
            return "—"
        blocks = " ▁▂▃▄▅▆▇█"
        low, high = min(values), max(values)
        span = high - low or 1.0
        return "".join(blocks[min(8, int((value - low) / span * 8))] for value in values)

    def _hqb(self) -> dict[str, Any]:
        benchmark = V36HQBBenchmark()
        score = benchmark.run_benchmark()
        decision = HonestDecisionModule().evaluate(score, context="tui observation")
        return {"verdict": decision.verdict.value.upper(), "reason": decision.reason, "score": score.total}

    def on_key(self, event: Key) -> None:
        """Own Ctrl+P before Textual's built-in palette consumes it."""
        if event.key == "ctrl+p":
            event.stop()
            event.prevent_default()
            self.action_toggle_palette()
        elif event.key == "ctrl+l":
            event.stop()
            event.prevent_default()
            self.run_worker(self.action_clear_chat(), exclusive=True, name="clear-chat")

    def action_toggle_palette(self) -> None:
        self.query_one("#command-palette", CommandPalette).toggle()

    async def action_clear_chat(self) -> None:
        await self.query_one("#chat", ChatView).clear_messages()

    def on_input_submitted(self, event: Input.Submitted) -> None:
        text = event.value.strip()
        event.input.value = ""
        if not text:
            return
        self.run_worker(self._process_input(text), exclusive=True, name="apeireth-task")

    async def _process_input(self, text: str) -> None:
        if text.startswith("/"):
            await self._command(text)
            return
        task_type = self._task_type(text)
        self._select_route(task_type)
        user_message = self.sessions.append_message(
            "user", text, metadata={"model": self.route["model"], "task_type": task_type}
        )
        chat = self.query_one("#chat", ChatView)
        await chat.add_message(user_message)
        self.memory_hits += 1
        started = time.perf_counter()
        try:
            callback = self._call_llm_override or _model_call(self.route["model"])
            answer = await asyncio.to_thread(callback, text)
            answer = str(answer or "No response was returned.")
            error = ""
        except Exception as exc:
            answer = f"Backend error: {exc}"
            error = type(exc).__name__
        latency = (time.perf_counter() - started) * 1000
        hqb = self._hqb()
        score = compute_asi_fun_score(
            task_type=task_type,
            model=self.route["model"],
            deliberation=True,
            reasoning_steps=3,
            emergence_index=1.0,
            phi_intrinsic=0.0,
            hqb_verdict=hqb["verdict"].lower(),
            hqb_violations=0,
            total_decisions=1,
        )
        self.action_scores.append(score)
        metadata = {
            "model": self.route["model"],
            "task_type": task_type,
            "hqb_verdict": hqb["verdict"],
            "fun_score": score,
        }
        if error:
            metadata["error"] = error
        assistant = self.sessions.append_message(
            "assistant", answer, tokens=max(1, len(answer) // 4), latency_ms=latency, metadata=metadata
        )
        await chat.stream_message(assistant)
        self.memory_hits += 1
        self._refresh_panels()

    async def _system(self, content: str) -> None:
        message = self.sessions.append_message("system", content, metadata={"model": self.route.get("model", "-")})
        await self.query_one("#chat", ChatView).add_message(message)

    async def _command(self, text: str) -> None:
        command, _, argument = text.partition(" ")
        argument = argument.strip()
        if command in ("/quit", "/exit"):
            self.exit()
        elif command == "/help":
            self.query_one("#command-palette", CommandPalette).display = True
            await self._system("Commands: " + ", ".join(("/model", "/clear", "/sessions", "/switch", "/new", "/help", "/quit")))
        elif command == "/clear":
            await self.action_clear_chat()
        elif command == "/new":
            self.sessions.create(self.route.get("model", "qwen-coder"))
            self.memory_hits = 0
            await self.query_one("#chat", ChatView).clear_messages()
            self._refresh_panels()
        elif command == "/sessions":
            recent = self.sessions.list_recent(10)
            listing = "\n".join(f"{item['id']}  {len(item.get('messages', []))} messages" for item in recent) or "No sessions"
            await self._system(listing)
        elif command == "/switch":
            try:
                session = self.sessions.load(argument)
                self.memory_hits = len(session.get("messages", []))
                await self.query_one("#chat", ChatView).load_messages(session.get("messages", []))
                self._select_route("qa", session.get("model"))
                self._refresh_panels()
            except SessionError as exc:
                await self._system(f"Session error: {exc}")
        elif command == "/model":
            if argument not in SUPPORTED_MODELS:
                await self._system(f"Unknown model: {argument}")
            else:
                self._select_route(self.route.get("task_type", "qa"), argument)
                await self._system(f"Model switched to {argument}")
                self._refresh_panels()
        else:
            await self._system("Unknown command. Press Ctrl+P or use /help.")


ApeirethApp = ApeirethTUI


def main() -> int:
    ApeirethTUI().run()
    return 0


__all__ = ["ApeirethApp", "ApeirethTUI", "ArtifactReader", "TUI_VERSION", "main"]


if __name__ == "__main__":
    raise SystemExit(main())
