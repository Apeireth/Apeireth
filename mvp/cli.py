"""R13 MVP — CLI entry (--new-session / --resume-session / --chat / --recall).

Ponytail ceiling: click for argparse, rich for color. Phase 2 will swap
echo() for LLM-generated response; keep CLI surface stable.
"""
from __future__ import annotations

import sys
from pathlib import Path
from typing import Optional

import click

from mvp.memory.store import Store
from mvp.memory.retrieve import retrieve, retrieve_notes
from mvp.identity import card as idcard

DEFAULT_DB = Path("./data/mvp.db")


def _make_store(db: Path) -> Store:
    return Store(db_path=db)


def _echo_episode(store: Store, session_id: str, role: str, content: str) -> None:
    """Phase 1: echo + store. Phase 2: LLM-generated reply."""
    ep = store.append_episode(role=role, content=content, session_id=session_id)
    click.echo(f"[{ep.role}] {ep.content}")


@click.group()
@click.option("--db", default=DEFAULT_DB, type=click.Path(path_type=Path),
              help="SQLite database path (default: ./data/mvp.db)")
@click.pass_context
def cli(ctx: click.Context, db: Path) -> None:
    """R13 MVP CLI — 跨 session 记忆 agent."""
    ctx.ensure_object(dict)
    ctx.obj["db"] = db


@cli.command()
@click.option("--session-id", default=None, help="Explicit session id (default: random)")
@click.pass_context
def new_session(ctx: click.Context, session_id: Optional[str]) -> None:
    """开新 session."""
    store = _make_store(ctx.obj["db"])
    sid = store.start_session(session_id)
    click.echo(f"new session: {sid}")
    store.close()


@cli.command()
@click.pass_context
def resume_session(ctx: click.Context) -> None:
    """恢复上次 session (auto-append episode 'resume')."""
    store = _make_store(ctx.obj["db"])
    sid = store.last_session()
    if sid is None:
        click.echo("no previous session; use --new-session first", err=True)
        sys.exit(1)
    click.echo(f"resumed session: {sid}")
    recent = store.list_episodes(session_id=sid, limit=10)
    click.echo(f"  recent episodes ({len(recent)}):")
    for ep in reversed(recent):
        click.echo(f"    [{ep.role}] {ep.content[:80]}")
    store.close()


@cli.command()
@click.option("--session-id", default=None)
@click.pass_context
def chat(ctx: click.Context, session_id: Optional[str]) -> None:
    """Interactive REPL: type 'add <text>' / 'recall <query>' / 'note <text>' / 'bye'."""
    store = _make_store(ctx.obj["db"])
    sid = session_id or store.last_session()
    if sid is None:
        sid = store.start_session()
        click.echo(f"auto-started session: {sid}")
    else:
        # ensure exists
        sid = store.start_session(sid)
    click.echo(f"chat session: {sid} (db={ctx.obj['db']})")
    click.echo("commands: add <text> | recall <query> | note <text> | whoami | bye")
    while True:
        try:
            line = click.prompt("", default="", show_default=False)
        except (click.Abort, EOFError):
            click.echo("")
            break
        line = line.strip()
        if not line:
            continue
        if line in ("bye", "exit", "quit"):
            break
        if line.startswith("add "):
            content = line[4:].strip()
            if content:
                _echo_episode(store, sid, "user", content)
                # Phase 1: simple echo; Phase 2: LLM-generated reply
                _echo_episode(store, sid, "agent", f"(echo) 收到: {content[:60]}")
        elif line.startswith("recall "):
            query = line[7:].strip()
            if query:
                hits = retrieve(store, query, top_k=5, session_id=sid)
                if not hits:
                    click.echo(f"  (no match for: {query})")
                else:
                    click.echo(f"  top {len(hits)} for '{query}':")
                    for h in hits:
                        click.echo(f"    [bm25={h.bm25_score:.2f} sal={h.salience:.2f}]"
                                   f" [{h.episode.role}] {h.episode.content[:80]}")
                # also notes
                notes = retrieve_notes(store, query, top_k=3)
                if notes:
                    click.echo(f"  notes:")
                    for n in notes:
                        click.echo(f"    [conf={n.confidence:.2f}] {n.content[:80]}")
        elif line.startswith("note "):
            content = line[5:].strip()
            if content:
                note = store.add_note(content=content, confidence=0.7)
                click.echo(f"  note added: {note.id}")
        elif line == "whoami":
            c = idcard.load()
            click.echo(idcard.render(c))
        elif line == "stats":
            s = store.stats()
            click.echo(f"  episodes={s['episodes']} notes={s['notes']}"
                       f" sessions={s['sessions']}")
        else:
            click.echo(f"  unknown command: {line.split()[0]}")
    click.echo(f"bye (session {sid})")
    store.close()


@cli.command()
@click.option("--session-id", default=None)
@click.argument("query")
@click.pass_context
def recall(ctx: click.Context, session_id: Optional[str], query: str) -> None:
    """One-shot recall: python -m mvp.cli recall '少数民族语' """
    store = _make_store(ctx.obj["db"])
    sid = session_id or store.last_session()
    hits = retrieve(store, query, top_k=5, session_id=sid)
    if not hits:
        click.echo("(no match)")
    else:
        for h in hits:
            click.echo(f"[bm25={h.bm25_score:.2f} sal={h.salience:.2f}]"
                       f" [{h.episode.role}] {h.episode.content}")
    store.close()


@cli.command(name="consolidate")
@click.option("--session-id", default=None,
              help="Target session id (default: last)")
@click.option("--note-threshold", default=0.2, type=float,
              help="Forget notes with confidence below this (default: 0.2)")
@click.option("--merge-threshold", default=0.85, type=float,
              help="Merge notes with cosine similarity >= this (default: 0.85)")
@click.pass_context
def consolidate_cmd(ctx: click.Context, session_id: Optional[str],
                     note_threshold: float, merge_threshold: float) -> None:
    """周期 consolidate: 从 Episode 提炼 Note + 合并相似 + 更新 IdentityCard.

    主 17:43 实事求是: 启发式, Phase 2 LLM 接入后换 LLM 提炼.
    """
    from mvp.memory import consolidate as cm
    from mvp.memory import forget as fm

    db_path = ctx.obj["db"]
    # IdentityCard JSON 路径 = db_path 同目录 + 不同后缀, 避免读 SQLite binary
    card_path = db_path.with_suffix(".card.json")
    store = _make_store(db_path)
    sid = session_id or store.last_session()
    if sid is None:
        click.echo("no previous session; use --new-session first", err=True)
        sys.exit(1)

    episodes = store.list_episodes(session_id=sid, limit=200)
    if not episodes:
        click.echo(f"no episodes in session {sid}; nothing to consolidate")
        store.close()
        return

    # 1. extract_notes (启发式)
    card = idcard.load(card_path)
    notes = cm.extract_notes(episodes, card)

    # 2. merge_similar_notes (cosine)
    notes = cm.merge_similar_notes(notes, threshold=merge_threshold)

    # 3. forget_low_confidence_notes
    notes = fm.forget_low_confidence_notes(notes, threshold=note_threshold)

    # 4. 写回 Store (先清空旧 notes for session, 再 add)
    # Ponytail: Phase 1.2 简化方案, 全删全加. Phase 1.4 可换 upsert by content.
    store._conn.execute("DELETE FROM notes")
    store._conn.commit()
    saved: list = []
    for n in notes:
        saved_note = store.add_note(
            content=n.content,
            source_episode_ids=n.source_episode_ids,
            confidence=n.confidence,
            tags=n.tags,
        )
        saved.append(saved_note)

    # 5. IdentityCard consolidate
    added = list(card.owner_background)
    card.consolidate(saved)
    new_added = [x for x in card.owner_background if x not in added]
    idcard.save(card, card_path)

    click.echo(f"consolidated: {len(episodes)} episodes → {len(saved)} notes"
               f" (forget < {note_threshold}, merge >= {merge_threshold})")
    if new_added:
        click.echo(f"identity evolved: +{new_added}")
    store.close()


cli.add_command(new_session)
cli.add_command(resume_session)
cli.add_command(chat)
cli.add_command(recall)
cli.add_command(consolidate_cmd)


def main() -> None:
    cli(obj={})


if __name__ == "__main__":
    main()