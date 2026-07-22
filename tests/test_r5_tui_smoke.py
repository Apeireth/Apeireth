"""R5 Textual TUI in-process smoke tests."""
from __future__ import annotations

import json
from pathlib import Path

import pytest
from textual.widgets import Input

from apeireth.tui import ApeirethTUI
from apeireth.tui_sessions import SessionStore
from apeireth.tui_widgets import CommandPalette


async def _wait_for_messages(pilot, app: ApeirethTUI, count: int) -> None:
    for _ in range(40):
        if len(app.sessions.current.get("messages", [])) >= count:
            return
        await pilot.pause(0.05)
    raise AssertionError(f"expected {count} messages")


async def _submit(pilot, app: ApeirethTUI, text: str) -> None:
    field = app.query_one("#task-input", Input)
    field.focus()
    field.value = text
    await pilot.press("enter")


@pytest.mark.asyncio
async def test_tui_starts_and_quits(tmp_path: Path) -> None:
    app = ApeirethTUI(sessions_dir=tmp_path, call_llm=lambda prompt: prompt)
    async with app.run_test(size=(120, 40)) as pilot:
        await pilot.pause()
        for selector in ("#topbar", "#left-panel", "#chat", "#right-panel", "#bottom-bar"):
            assert app.query_one(selector)
        await pilot.press("ctrl+q")


@pytest.mark.asyncio
async def test_tui_send_message_no_crash(tmp_path: Path, monkeypatch) -> None:
    monkeypatch.delenv("NEWAPI_API_KEY", raising=False)
    monkeypatch.delenv("MINIMAX_API_KEY", raising=False)
    app = ApeirethTUI(sessions_dir=tmp_path)  # true llm_kernel path; no-key is deterministic
    async with app.run_test(size=(120, 40)) as pilot:
        await _submit(pilot, app, "hello")
        await _wait_for_messages(pilot, app, 2)
        messages = app.sessions.current["messages"]
        assert [message["role"] for message in messages] == ["user", "assistant"]
        assert messages[-1]["content"]


@pytest.mark.asyncio
async def test_tui_command_palette(tmp_path: Path) -> None:
    app = ApeirethTUI(sessions_dir=tmp_path, call_llm=lambda prompt: prompt)
    async with app.run_test(size=(120, 40)) as pilot:
        palette = app.query_one("#command-palette", CommandPalette)
        assert palette.display is False
        await pilot.press("ctrl+p")
        await pilot.pause()
        assert palette.display is True
        rendered = str(palette.render())
        assert "/model" in rendered and "/switch" in rendered and "/quit" in rendered


@pytest.mark.asyncio
async def test_tui_sessions_persist(tmp_path: Path) -> None:
    first = ApeirethTUI(sessions_dir=tmp_path, call_llm=lambda prompt: f"answer:{prompt}")
    async with first.run_test(size=(120, 40)) as pilot:
        await _submit(pilot, first, "remember me")
        await _wait_for_messages(pilot, first, 2)
        session_id = first.sessions.current["id"]
        first.exit()
    path = tmp_path / f"{session_id}.json"
    assert path.exists()
    assert len(json.loads(path.read_text(encoding="utf-8"))["messages"]) == 2

    second = ApeirethTUI(sessions_dir=tmp_path, call_llm=lambda prompt: prompt)
    async with second.run_test(size=(120, 40)):
        assert second.sessions.current["id"] == session_id
        assert len(second.sessions.current["messages"]) == 2
        second.exit()


@pytest.mark.asyncio
async def test_tui_switch_session(tmp_path: Path) -> None:
    store = SessionStore(tmp_path)
    old = store.create("template")
    old_id = old["id"]
    store.append_message("user", "old history", metadata={"model": "template"})

    app = ApeirethTUI(sessions_dir=tmp_path, call_llm=lambda prompt: f"answer:{prompt}")
    async with app.run_test(size=(120, 40)) as pilot:
        assert app.sessions.current["id"] == old_id
        await app._command("/new")
        new_id = app.sessions.current["id"]
        assert new_id != old_id
        await app._process_input("new history")
        assert len(app.sessions.current["messages"]) == 2
        await app._command(f"/switch {old_id}")
        await pilot.pause()
        assert app.sessions.current["id"] == old_id
        assert app.sessions.current["messages"][0]["content"] == "old history"


@pytest.mark.asyncio
async def test_tui_no_asi_leak_in_input(tmp_path: Path) -> None:
    app = ApeirethTUI(sessions_dir=tmp_path, call_llm=lambda prompt: prompt)
    async with app.run_test(size=(120, 40)) as pilot:
        field = app.query_one("#task-input", Input)
        assert field.value == ""
        assert "asi" not in field.placeholder.lower()
        await _submit(pilot, app, "hello")
        await _wait_for_messages(pilot, app, 2)
        assert field.value == ""


@pytest.mark.asyncio
async def test_tui_model_command(tmp_path: Path) -> None:
    app = ApeirethTUI(sessions_dir=tmp_path, call_llm=lambda prompt: prompt)
    async with app.run_test(size=(120, 40)):
        await app._command("/model template")
        assert app.route["model"] == "template"
        assert app.sessions.current["model"] == "template"
        app.exit()
