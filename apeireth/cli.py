"""User-facing command line interface for Apeireth.

The CLI deliberately keeps platform invariants private.  ``run`` exposes only
an answer by default; ``--score`` and ``--debug`` are explicit opt-ins.
"""
from __future__ import annotations

import argparse
import contextlib
import io
import sys
from dataclasses import dataclass
from typing import Callable, Sequence

from . import __version__
from .asi_demo_v8 import ASIDemoV8
from .deliberation import DeliberationEngine, DeliberationResult
from .llm_kernel import LLMConfig, call_llm_minimax, make_call_llm
from .self_evolving import Harness, phase1_eval


CLI_VERSION = __version__
DEFAULT_MODEL = "minimax"

# These names are intentionally a small, explicit public surface.  The router
# knows more models, but accepting arbitrary strings here would silently turn a
# typo into an unconfigured provider (the old kernel factory's behaviour).
_EXTERNAL_MODELS = frozenset(
    {
        "deepseek-v3",
        "claude-opus-4",
        "claude-sonnet-4",
        "gpt-4o",
        "gpt-4o-mini",
        "qwen-coder",
    }
)
_TEMPLATE_MODELS = frozenset({"template", "apeireth-default"})
_MINIMAX_MODELS = frozenset({"minimax", "MiniMax-M3"})
SUPPORTED_MODELS = _TEMPLATE_MODELS | _MINIMAX_MODELS | _EXTERNAL_MODELS


class UnknownModelError(ValueError):
    """Raised when a user asks for a model not registered with the CLI."""


@dataclass(frozen=True)
class _InternalInvariant:
    """Private run metadata; never printed unless ``--debug`` is requested."""

    model: str
    cognitive_layer: str
    deliberation_steps: int
    life_features: int
    harness_ok: bool


def _model_call(model: str) -> Callable[[str], str]:
    """Build the kernel callback for a registered model name.

    ``llm_kernel`` exposes provider-oriented callbacks.  For registered model
    IDs we use the same OpenAI-compatible kernel with the requested model name,
    while preserving its no-key fallback and error handling.
    """
    if model not in SUPPORTED_MODELS:
        choices = ", ".join(sorted(SUPPORTED_MODELS))
        raise UnknownModelError(f"unknown model {model!r}; choose one of: {choices}")

    if model in _TEMPLATE_MODELS:
        return make_call_llm("template")

    if model in _MINIMAX_MODELS:
        # Keep the kernel's configured MiniMax endpoint and environment lookup.
        return make_call_llm("minimax")

    # External IDs use the existing OpenAI-compatible request implementation.
    # No new HTTP client or dependency is introduced for the CLI.
    config = LLMConfig.minimax_default()
    config.provider = model
    config.model = model

    def call(prompt: str) -> str:
        return call_llm_minimax(prompt, config=config).content

    return call


def _run_task(task: str, model: str) -> tuple[str, DeliberationResult, _InternalInvariant]:
    """Run a task through the LLM kernel and L4-L5 deliberation layer."""
    call_llm = _model_call(model)
    engine = DeliberationEngine(call_llm=call_llm)
    result = engine.deliberate(task, mode="reflexion")

    # Reflexion's final step is the user-facing answer.  Do not print the
    # private chain, branches, or prompts.  A provider is allowed to return an
    # empty body, so retain a useful non-empty response in that edge case.
    answer = result.final_plan[-1].strip() if result.final_plan else ""
    if not answer:
        answer = call_llm(task).strip()
    if not answer:
        answer = "No response was returned for this task."

    # V5 harness integrity is deliberately represented as a private invariant
    # rather than exposing its implementation/report in the user response.
    harness = Harness(
        archetypes={"task_runner": {"description": "task execution", "weight": 1.0}},
        sct_weights={"task_runner": {"cognitive": 1.0}},
        funnel_priors={"task": 0.5},
    )
    harness_report = phase1_eval(harness, recent_events=[{"task": task}])
    invariant = _InternalInvariant(
        model=model,
        cognitive_layer="L4-L5",
        deliberation_steps=result.total_steps,
        life_features=12,
        harness_ok=0.0 <= harness_report.score <= 1.0,
    )
    return answer, result, invariant


def _print_run(answer: str, result: DeliberationResult, invariant: _InternalInvariant,
               *, show_score: bool, debug: bool) -> None:
    """Print only explicitly requested metadata in addition to the answer."""
    print(answer)
    if show_score:
        print(f"\nASI score (for fun): {result.self_score:.4f}")
    if debug:
        print("\n[debug]")
        print(f"model: {invariant.model}")
        print(f"cognitive_layer: {invariant.cognitive_layer}")
        print(f"deliberation_steps: {invariant.deliberation_steps}")
        print(f"life_features: {invariant.life_features} (background)")
        print(f"invariants: {'PASS' if invariant.harness_ok else 'FAIL'}")


def _run_demo() -> int:
    """Run and summarize asi_demo_v8's user-facing Phase 1-5 path."""
    demo = ASIDemoV8(verbose=False)
    phases = (
        ("Phase 1", demo.phase1_v31_init),
        ("Phase 2", demo.phase2_v32_init),
        ("Phase 3", demo.phase3_v33_init),
        ("Phase 4", demo.phase4_v34_dialog),
        ("Phase 5", demo.phase5_v35_evolve),
    )

    # A dependency may print diagnostics while initializing.  The CLI contract
    # is a concise user view, so keep those internal diagnostics out of stdout.
    with contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
        steps = [(label, method()) for label, method in phases]

    print("Apeireth demo")
    for label, step in steps:
        status = "ok" if "error" not in step.artifacts else "error"
        print(f"{label}: {status}")
    return 0


def _build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(prog="apeireth", description="Run tasks on Apeireth.")
    parser.add_argument("--version", action="version", version=f"apeireth {CLI_VERSION}")
    commands = parser.add_subparsers(dest="command")

    run_parser = commands.add_parser("run", help="run a task")
    run_parser.add_argument("--model", default=DEFAULT_MODEL, help="registered model name")
    run_parser.add_argument("--score", action="store_true", help="show the optional fun score")
    run_parser.add_argument("--debug", action="store_true", help="show internal invariants")
    run_parser.add_argument("task", nargs="+", help="task to run")

    commands.add_parser("demo", help="run the user-facing demo phases 1-5")
    commands.add_parser("tui", help="open the terminal observation console")
    return parser


def main(argv: Sequence[str] | None = None) -> int:
    """CLI entry point returning a process status code."""
    parser = _build_parser()
    args = parser.parse_args(argv)

    if args.command == "run":
        task = " ".join(args.task).strip()
        if not task:
            parser.error("run requires a non-empty task")
        try:
            answer, result, invariant = _run_task(task, args.model)
        except UnknownModelError as exc:
            print(f"apeireth: error: {exc}", file=sys.stderr)
            return 2
        except Exception as exc:  # keep provider failures from producing a traceback
            print(f"apeireth: run failed: {exc}", file=sys.stderr)
            return 1
        _print_run(answer, result, invariant, show_score=args.score, debug=args.debug)
        return 0

    if args.command == "demo":
        try:
            return _run_demo()
        except Exception as exc:
            print(f"apeireth: demo failed: {exc}", file=sys.stderr)
            return 1

    if args.command == "tui":
        try:
            from .tui import main as tui_main
            return tui_main()
        except Exception as exc:
            print(f"apeireth: tui failed: {exc}", file=sys.stderr)
            return 1

    parser.print_help()
    return 0


__all__ = [
    "CLI_VERSION",
    "DEFAULT_MODEL",
    "SUPPORTED_MODELS",
    "UnknownModelError",
    "main",
]


if __name__ == "__main__":
    raise SystemExit(main())
