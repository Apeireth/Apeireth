"""Small Textual widgets used by the Apeireth observation TUI."""
from __future__ import annotations

import asyncio
import time
from typing import Any, Iterable

from rich.text import Text
from textual.app import ComposeResult
from textual.containers import Vertical, VerticalScroll
from textual.widgets import Input, Static


COMMANDS = (
    "/model <name>",
    "/clear",
    "/sessions",
    "/switch <session_id>",
    "/new",
    "/help",
    "/quit",
)


def _clock(ts: float | int | None) -> str:
    return time.strftime("%H:%M:%S", time.localtime(float(ts or time.time())))


class TopBar(Static):
    """Current V1083 routing decision."""

    def set_route(self, route: dict[str, Any]) -> None:
        chain = " → ".join(str(item) for item in route.get("failover", []) if item)
        self.update(
            f"MODEL {route.get('model', 'N/A')}  |  {chain or 'no failover'}  |  "
            f"affinity {route.get('affinity', 'N/A')}  cost ${route.get('cost', 'N/A')}  "
            f"p50 {route.get('latency_ms', 'N/A')}ms"
        )


class MetricsPanel(Static):
    """Label/value observation panel."""

    def set_metrics(self, title: str, metrics: Iterable[tuple[str, Any]]) -> None:
        lines = [f"[b]{title}[/b]"]
        lines.extend(f"{name}: {value}" for name, value in metrics)
        self.update("\n".join(lines))


class ChatMessage(Static):
    """One persisted chat message with request metadata."""

    def __init__(self, message: dict[str, Any]):
        super().__init__(classes=f"message {message.get('role', 'system')}")
        self.message = message
        self.render_message()

    def render_message(self) -> None:
        m = self.message
        metadata = m.get("metadata") or {}
        heading = (
            f"{str(m.get('role', 'system')).upper()} {_clock(m.get('ts'))}  "
            f"{metadata.get('model', '-')}  {m.get('tokens', 0)} tok  "
            f"{float(m.get('latency_ms', 0)):.1f}ms\n"
        )
        self.update(Text(heading + str(m.get("content", ""))))


class ChatView(VerticalScroll):
    """Central chat area with chunked response rendering."""

    async def add_message(self, message: dict[str, Any]) -> ChatMessage:
        widget = ChatMessage(message)
        await self.mount(widget)
        self.scroll_end(animate=False)
        return widget

    async def stream_message(self, message: dict[str, Any], chunk_size: int = 48) -> ChatMessage:
        content = str(message.get("content", ""))
        streamed = dict(message)
        streamed["content"] = ""
        widget = await self.add_message(streamed)
        for offset in range(0, len(content), chunk_size):
            streamed["content"] += content[offset: offset + chunk_size]
            widget.message = streamed
            widget.render_message()
            await asyncio.sleep(0)
        return widget

    async def load_messages(self, messages: Iterable[dict[str, Any]]) -> None:
        await self.remove_children()
        for message in messages:
            await self.add_message(message)

    async def clear_messages(self) -> None:
        await self.remove_children()


class CommandPalette(Static):
    """Ctrl+P command reference; commands are entered in the task input."""

    def on_mount(self) -> None:
        self.display = False
        self.update("[b]COMMANDS[/b]\n" + "\n".join(COMMANDS))

    def toggle(self) -> None:
        self.display = not self.display


class BottomBar(Vertical):
    """Session status, input prompt and shortcut hint."""

    def compose(self) -> ComposeResult:
        yield Static("", id="session-status")
        yield Input(placeholder="› Type a task or /help", id="task-input")
        yield Static("Ctrl+P commands  Ctrl+Q quit  Ctrl+L clear", id="shortcut-hint")

    def set_session(self, session_id: str, memory_hits: int) -> None:
        self.query_one("#session-status", Static).update(
            f"session {session_id}  |  memory hits {memory_hits}"
        )


__all__ = [
    "COMMANDS",
    "BottomBar",
    "ChatMessage",
    "ChatView",
    "CommandPalette",
    "MetricsPanel",
    "TopBar",
]
