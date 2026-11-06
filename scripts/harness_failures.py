#!/usr/bin/env python3
"""失败分类器 —— 把一坨日志 blob 结构化成「类别 + 证据 + 下一步动作」。

用法
----
    python3 scripts/harness_failures.py <log-file> [--json] [--add-knowledge] [--quiet]
    python3 scripts/harness_failures.py --stdin [--json] [--add-knowledge] [--quiet]

为什么要有它
------------
仓库里守卫/测试一红，留下的只有一段日志。日志本身不回答两个问题：

1. **下一步做什么**（是改代码、修环境，还是守卫自己坏了）；
2. **这个失败是不是见过**（同一类失败反复发生时，人/AI 不该从零开始想）。

本脚本把日志归到 **9 个真实类别 + unknown** 之一，给出**命中证据**和**一句可执行的下一步**；
`--add-knowledge` 再把命中沉淀进 `.harness/knowledge/failure-patterns.yaml`，
让重复失败**累积**成可检索的知识库。

类别与优先级
------------
规范顺序（根因优先，先命中先归类）::

    compile-error → test-regression → flaky-test → environment-failure →
    contract-drift → stale-base → dependency-drift → harness-bug →
    false-positive → unknown

在此之上有 **4 条「细化覆盖」规则**（否则下面这些类别永远不可能成为结果，
因为它们的日志必然同时命中更通用的类别）：

* ``flaky-test`` 覆盖 ``test-regression``（flaky 日志必然含 ``test result: FAILED``）
* ``false-positive`` 覆盖 ``contract-drift``（误报日志必然含 ``STALE``/``不一致``）
* ``harness-bug`` 覆盖 ``contract-drift`` / ``test-regression`` / ``false-positive``
  （守卫自己崩了 ⇒ 它给出的任何裁决都不可信）
* ``environment-failure``（仅**硬**环境信号）覆盖 ``test-regression``
  （测试红是因为环境坏 ⇒ 下一步是修环境，不是改代码）

每条覆盖规则都带**触发门槛**（winner's 信号强度须 ≥ ``medium``）——
否则弱信号会把一个明确的失败裁决冲掉。典型例子：nextest ``profile.ci``
里 ``retries = 2`` ⇒ **任何**失败都会打 ``TRY n FAIL`` ⇒ 若把它当强信号，
稳定失败会被误判成 flaky（本仓 ``nextest.toml`` 注释里警告的同一个坑）。

**证据不丢**：所有类别命中的信号都会记下来（正则 / 命中原文 / 行号），
即使最终归到了别的类别 —— 方便人复核判断对不对。

零依赖
------
YAML 读写**优先**用 PyYAML；**没有 PyYAML 也能跑**（自带最小 YAML 子集解析/序列化），
因为仓库不保证脚本环境里装了 PyYAML。
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from datetime import date
from pathlib import Path
from typing import Any

SCHEMA = "apeireth.harness-failure.v1"
PATTERNS_SCHEMA = "apeireth.failure-patterns.v1"

ROOT = Path(__file__).resolve().parent.parent
KNOWLEDGE_DIR = ROOT / ".harness" / "knowledge"
PATTERNS_FILE = KNOWLEDGE_DIR / "failure-patterns.yaml"

# 规范顺序（见模块 docstring）
CATEGORY_ORDER = (
    "compile-error",
    "test-regression",
    "flaky-test",
    "environment-failure",
    "contract-drift",
    "stale-base",
    "dependency-drift",
    "harness-bug",
    "false-positive",
    "unknown",
)

# 细化覆盖：winner 覆盖 loser。
# 每条带**触发门槛**：winner 的信号强度必须 ≥ 该级别才允许覆盖 —— 否则
# 「弱信号」（如 nextest 的 `TRY n`）会把一个明确的失败裁决冲掉。
REFINEMENTS = (
    ("harness-bug", ("contract-drift", "test-regression", "false-positive"), "medium"),
    ("flaky-test", ("test-regression",), "medium"),
    ("false-positive", ("contract-drift",), "medium"),
)

# 硬环境信号才覆盖 test-regression（弱信号如 "network" 不覆盖）
HARD_ENV_OVERRIDES = (("environment-failure", ("test-regression",)),)

LEVELS = {"weak": 1, "medium": 2, "strong": 3}

# ---------------------------------------------------------------------------
# 模式表
# ---------------------------------------------------------------------------
# 每条：{"re": 正则, "level": weak|medium|strong, "note": 说明,
#        "requires": 可选，companion 正则；日志里必须**同时**出现才算命中}
#
# `requires` 用来表达「必须与另一个信号共现」的语义，例如：
#   * harness-bug 的 Traceback 必须来自 `scripts/` 下的文件（否则是应用的 traceback）；
#   * false-positive 必须与一个「漂移/违规」信号共现（否则只是普通散文里的「已修」）。
# ---------------------------------------------------------------------------

DRIFT_MARKER = r"STALE|DRIFT|\bdrift\b|out of sync|不一致|超基线|越界|VIOLATION"

CATEGORY_PATTERNS: dict[str, list[dict[str, Any]]] = {
    "compile-error": [
        {"re": r"\berror\[E\d{4}\]", "level": "strong", "note": "rustc 错误码"},
        {"re": r"\berror: could not compile\b", "level": "strong", "note": "rustc 编译失败汇总"},
        {"re": r"\berror: aborting due to \d+ previous error", "level": "strong", "note": "rustc 错误汇总行"},
        # ⚠️ 只认 **rustc** 的错误形态（`error[E0xxx]` / `could not compile`）。
        #    泛化的 `^error:` 不能收 —— 否则 `error: the lock file ...`（依赖漂移）、
        #    `error: unknown revision`（基线失效）会被误判成编译错误。
    ],
    "test-regression": [
        {"re": r"\btest result: FAILED\b", "level": "strong", "note": "cargo test 汇总失败"},
        {"re": r"\bpanicked at\b", "level": "strong", "note": "Rust panic"},
        {"re": r"\bassertion failed\b", "level": "strong", "note": "Rust 断言失败"},
        {"re": r"\bassertion `left == right` failed\b", "level": "strong", "note": "Rust 断言失败"},
        {"re": r"\btests? run:.*\bfailed\b", "level": "medium", "note": "nextest 汇总含 failed"},
        {"re": r"\bSummary \[.*\b\d+ failed\b", "level": "medium", "note": "nextest 失败汇总"},
        {"re": r"^\s*FAIL\s*\[", "level": "strong", "note": "nextest 终判 FAIL 行"},
        {
            "re": r"\bFAILED\b",
            "level": "weak",
            "note": "FAILED 字面量（大写）",
            "flags": re.MULTILINE,  # ⚠️ 必须区分大小写：否则 "failed to select a version" 会被误判
        },
    ],
    "flaky-test": [
        {"re": r"\bflaky\b", "level": "medium", "note": "日志自称 flaky"},
        {"re": r"\bretry(?:ing|ied)?\b", "level": "weak", "note": "出现 retry"},
        {
            "re": r"\bTRY \d+\b",
            "level": "weak",
            "note": "nextest 重试标记",
            # ⚠️ 必须弱：nextest `profile.ci` 里 retries = 2 ⇒ **任何**失败都会打
            #    `TRY n FAIL` ⇒ 强信号会把「稳定失败」误判成 flaky（正是本仓
            #    nextest.toml 注释里警告的「重试把偶发失败洗成绿」）。
        },
    ],
    "environment-failure": [
        {"re": r"\bPermission denied\b", "level": "strong", "note": "权限不足"},
        {"re": r"\bENOSPC\b|No space left on device", "level": "strong", "note": "磁盘写满"},
        {"re": r"\bConnection refused\b", "level": "strong", "note": "连不上服务"},
        {"re": r"could not resolve (?:host|name)", "level": "strong", "note": "DNS 解析失败"},
        {
            "re": r"\b(?:cargo|rustc|rustup|python3?|node|pnpm|npm|git|nextest)\b[^\n]{0,60}"
                  r"(?:command not found|No such file or directory)",
            "level": "strong",
            "note": "工具缺失",
        },
        {"re": r": command not found", "level": "strong", "note": "工具缺失"},
        {"re": r"\bNo such file or directory\b", "level": "medium", "note": "文件/工具不存在"},
        {"re": r"\bnetwork\b", "level": "medium", "note": "网络"},
        {"re": r"\btimed out\b|\btimeout\b|\bETIMEDOUT\b", "level": "medium", "note": "超时"},
        {"re": r"failed to download", "level": "medium", "note": "下载失败"},
        {"re": r"\bECONNRESET\b|\bEAI_AGAIN\b", "level": "medium", "note": "网络错误码"},
        {"re": r"\boffline\b", "level": "weak", "note": "离线"},
    ],
    "contract-drift": [
        {"re": r"\bSTALE\b", "level": "strong", "note": "守卫判 STALE"},
        {"re": r"(?<![\w-])\bdrift\b", "level": "strong", "note": "守卫报漂移"},
        {"re": r"\bout of sync\b", "level": "strong", "note": "真源/副本不同步"},
        {"re": r"不一致", "level": "strong", "note": "不一致"},
        {"re": r"超基线", "level": "strong", "note": "超出基线"},
        {"re": r"越界", "level": "strong", "note": "越界"},
        {"re": r"\bVIOLATIONS?\b", "level": "medium", "note": "守卫报违规"},
        {"re": r"\bBLOCKED\b", "level": "medium", "note": "裁决 BLOCKED"},
    ],
    "stale-base": [
        {"re": r"\bbad revision\b", "level": "strong", "note": "git 认不出基线"},
        {"re": r"\bunknown revision\b", "level": "strong", "note": "git 认不出基线"},
        {"re": r"\bnot a valid object\b", "level": "strong", "note": "基线对象不存在"},
        {"re": r"\bambiguous argument\b", "level": "strong", "note": "git 参数歧义"},
    ],
    "dependency-drift": [
        {"re": r"failed to select a version", "level": "strong", "note": "依赖版本选不出来"},
        {"re": r"package\s+\S+\s+not found", "level": "strong", "note": "包找不到"},
        {"re": r"the lock file .* needs to be updated", "level": "strong", "note": "Cargo.lock 需要更新"},
        {"re": r"lock file needs to be updated", "level": "strong", "note": "Cargo.lock 需要更新"},
        {"re": r"\bout of date\b", "level": "strong", "note": "依赖过期"},
        {"re": r"Cargo\.lock", "level": "medium", "note": "提到 Cargo.lock"},
    ],
    "harness-bug": [
        {
            "re": r"Traceback \(most recent call last\)",
            "level": "strong",
            "note": "Python traceback（守卫脚本）",
            "requires": r"scripts/[\w./-]+\.py",
        },
        {
            "re": r"\bNameError\b",
            "level": "strong",
            "note": "守卫脚本 NameError",
            "requires": r"scripts/[\w./-]+\.py",
        },
        {
            "re": r"\bKeyError\b",
            "level": "strong",
            "note": "守卫脚本 KeyError",
            "requires": r"scripts/[\w./-]+\.py",
        },
        {
            "re": r"\bAssertionError\b",
            "level": "strong",
            "note": "守卫脚本 AssertionError",
            "requires": r"scripts/[\w./-]+\.py",
        },
        {"re": r"Traceback \(most recent call last\)", "level": "medium", "note": "Python traceback"},
    ],
    "false-positive": [
        {"re": r"历史口径", "level": "strong", "note": "历史口径（已不适用）", "requires": DRIFT_MARKER},
        {"re": r"\bhistorical\b", "level": "strong", "note": "历史口径（英文）", "requires": DRIFT_MARKER},
        {"re": r"已修", "level": "strong", "note": "已修（守卫引用的是已修事实）", "requires": DRIFT_MARKER},
        {"re": r"已过时", "level": "strong", "note": "已过时", "requires": DRIFT_MARKER},
        {"re": r"\bobsolete\b", "level": "medium", "note": "obsolete", "requires": DRIFT_MARKER},
    ],
}

# 「同一测试既失败又通过」—— 需要跨行比对，单独实现
_TEST_RESULT_RES = (
    re.compile(r"^\s*test\s+(?P<name>\S+)\s+\.\.\.\s+(?P<verdict>ok|FAILED|ignored)\b"),
    re.compile(r"^\s*(?P<verdict>PASS|FAIL|TRY \d+ FAIL|TRY \d+ PASS)\s+\[\s*[\d.]+s\]\s+(?P<name>\S+)"),
    re.compile(r"^\s*(?P<verdict>PASS|FAIL)\s+\[\s*[\d.]+s\]\s+\S+\s+(?P<name>\S+)\s*$"),
)

# 下一动作（中文，一句话，可执行）
NEXT_ACTIONS = {
    "compile-error": (
        "先跑 `cargo build -p <crate> --locked` 复现编译错误，"
        "按 rustc 的 `error[E0xxx]` 逐条修到编译通过，再谈其它门禁。"
    ),
    "test-regression": (
        "先定位失败用例名并单独复跑"
        "（`cargo nextest run -p <crate> --profile ci --locked <test_name>`），"
        "确认是稳定失败（而非偶发）后再改实现。"
    ),
    "flaky-test": (
        "先把该用例连跑 3 次确认是否可稳定复现；"
        "若确属偶发，按「隔离/标 flaky」处理，不要按稳定回归去改生产代码。"
    ),
    "environment-failure": (
        "先修环境本身（工具链/权限/网络/磁盘），"
        "再用 `python3 scripts/harness.py check` 复跑门禁；环境恢复前不要改业务代码。"
    ),
    "contract-drift": (
        "先跑 `python3 scripts/docs_truth.py --verify --block-stale` 列出漂移的声明/真源，"
        "再按真源重新生成或改正声明，**不要**手改人读副本。"
    ),
    "stale-base": (
        "先用 `git cat-file -t <sha>` 确认基线提交真实存在且已合入，"
        "再把 `verified_at`/基线 SHA 换成仓库里真实存在的提交。"
    ),
    "dependency-drift": (
        "先跑 `cargo metadata --locked` 与 `cargo update --dry-run` 看锁文件差异，"
        "同步 `Cargo.toml`/`Cargo.lock` 后再重跑门禁。"
    ),
    "harness-bug": (
        "先修守卫脚本自身（用 `python3 scripts/<guard>.py --help` 复现 traceback）；"
        "守卫没修好之前，**不要**采信它给出的任何裁决。"
    ),
    "false-positive": (
        "先回读被举报的那条证据，确认它确属历史口径/已修事实，"
        "再把这条误报从守卫基线里剔除或登记豁免，**不要**改实现去迎合它。"
    ),
    "unknown": (
        "先人工分诊：跑 `python3 scripts/harness.py plan` 看当前 diff 命中哪些门禁，"
        "再单独复跑对应门禁以拿到更明确的失败信号。"
    ),
}

# `--add-knowledge` 写入的 failure / prevention 模板
KB_FAILURE = {
    "compile-error": "编译不过 ⇒ 后续门禁全部无意义（信号被噪音淹没）",
    "test-regression": "用例失败 ⇒ 可能是真回归，也可能只是口径错",
    "flaky-test": "同一用例既失败又通过 ⇒ 偶发失败冒充回归",
    "environment-failure": "环境/工具链/网络/磁盘问题被误当成代码问题",
    "contract-drift": "守卫报漂移（真源与副本不一致）",
    "stale-base": "基线提交不存在/认不出 ⇒ 判定永远 STALE",
    "dependency-drift": "依赖或 Cargo.lock 与真源不一致",
    "harness-bug": "守卫脚本自身崩溃 ⇒ 它的裁决不可信",
    "false-positive": "守卫报了违规，但日志显示该违规已过时/已修 ⇒ 误报",
    "unknown": "无任何已知信号命中",
}

KB_PREVENTION = {
    "compile-error": "改动前先 `cargo build -p <crate> --locked`；跨 crate 接口改动先跑 `cargo check --workspace`",
    "test-regression": "改实现前先复跑该用例；口径变化时同步更新基线并说明理由",
    "flaky-test": "用例显式标注 flaky 并隔离；nextest profile 区分重试与真失败",
    "environment-failure": "把工具链/权限/网络前置写成可机检的断言，缺前置就明确报「环境不具备」",
    "contract-drift": "真源生成人读副本；守卫查真源而不是查副本",
    "stale-base": "`verified_at` 只写仓库里真实存在的提交；写入前用 `git cat-file -t` 校验",
    "dependency-drift": "依赖改动与 Cargo.lock 同批提交；CI 用 `--locked` 跑",
    "harness-bug": "守卫改动必须自测（`--help` + 自检子命令）；traceback 视为门禁红",
    "false-positive": "守卫命中历史事实时先核对证据时间；为已修项登记豁免而不是改实现",
    "unknown": "把新出现的失败形态登记成模式，别让日志停在「看不懂」",
}


# ---------------------------------------------------------------------------
# 最小 YAML 子集（无 PyYAML 时的兜底）
# ---------------------------------------------------------------------------

def _yaml_scalar(value: Any) -> str:
    """把标量序列化成 YAML 片段（必要时加双引号）。"""
    if value is None:
        return "null"
    if isinstance(value, bool):
        return "true" if value else "false"
    if isinstance(value, int):
        return str(value)
    if isinstance(value, float):
        return repr(value)
    text = str(value)
    if _needs_quotes(text):
        escaped = (
            text.replace("\\", "\\\\")
            .replace('"', '\\"')
            .replace("\n", "\\n")
            .replace("\t", "\\t")
        )
        return f'"{escaped}"'
    return text


# 日期/时间样式的字符串：必须加引号，否则 YAML 会读成 date/timestamp
# （`first_seen: 2026-10-28` ⇒ 读出来是 datetime.date，不是 str）。
_DATE_LIKE_RE = re.compile(r"^\d{4}-\d{1,2}-\d{1,2}([Tt ].*)?$")


def _needs_quotes(text: str) -> bool:
    if text == "" or text != text.strip():
        return True
    if text[0] in "-?:,[]{}#&*!|>'\"%@`":
        return True
    if ": " in text or text.endswith(":"):
        return True
    if " #" in text:
        return True
    if "\n" in text or "\t" in text:
        return True
    if text.lower() in ("true", "false", "null", "yes", "no", "on", "off", "~"):
        return True
    if _DATE_LIKE_RE.match(text):
        return True
    try:
        float(text)
    except ValueError:
        return False
    return True


def _dump_yaml_block(value: Any, indent: int, out: list[str]) -> None:
    """递归把 dict/list 写成 YAML 块（不含顶层键名）。"""
    pad = " " * indent
    if isinstance(value, dict):
        for key, item in value.items():
            if isinstance(item, (dict, list)):
                if not item:
                    out.append(f"{pad}{key}: {{}}" if isinstance(item, dict) else f"{pad}{key}: []")
                    continue
                out.append(f"{pad}{key}:")
                _dump_yaml_block(item, indent + 2, out)
            else:
                out.append(f"{pad}{key}: {_yaml_scalar(item)}")
    elif isinstance(value, list):
        for item in value:
            if isinstance(item, dict) and item:
                items = list(item.items())
                first_key, first_value = items[0]
                if isinstance(first_value, (dict, list)):
                    out.append(f"{pad}- {first_key}:")
                    _dump_yaml_block(first_value, indent + 4, out)
                else:
                    out.append(f"{pad}- {first_key}: {_yaml_scalar(first_value)}")
                if len(items) > 1:
                    _dump_yaml_block(dict(items[1:]), indent + 2, out)
            elif isinstance(item, list):
                out.append(f"{pad}-")
                _dump_yaml_block(item, indent + 2, out)
            else:
                out.append(f"{pad}- {_yaml_scalar(item)}")


def dump_yaml(data: dict[str, Any]) -> str:
    """序列化顶层 mapping（自带最小实现 ⇒ 输出不依赖 PyYAML 是否安装）。"""
    out: list[str] = []
    _dump_yaml_block(data, 0, out)
    return "\n".join(out) + "\n"


def _strip_comment(line: str) -> str:
    """去掉行尾注释（引号内的 # 不算）。"""
    out: list[str] = []
    quote: str | None = None
    index = 0
    while index < len(line):
        char = line[index]
        if quote:
            out.append(char)
            if char == "\\" and quote == '"' and index + 1 < len(line):
                out.append(line[index + 1])
                index += 2
                continue
            if char == quote:
                quote = None
        elif char in "\"'":
            quote = char
            out.append(char)
        elif char == "#" and (not out or out[-1] in " \t"):
            break
        else:
            out.append(char)
        index += 1
    return "".join(out).rstrip()


def _split_key_value(text: str) -> tuple[str, str] | None:
    """在引号外找第一个 ``: `` 或行尾 ``:``。"""
    quote: str | None = None
    index = 0
    while index < len(text):
        char = text[index]
        if quote:
            if char == "\\" and quote == '"':
                index += 2
                continue
            if char == quote:
                quote = None
        elif char in "\"'":
            quote = char
        elif char == ":" and (index + 1 == len(text) or text[index + 1] in " \t"):
            return text[:index].strip(), text[index + 1:].strip()
        index += 1
    return None


def _parse_scalar(text: str) -> Any:
    text = text.strip()
    if text == "":
        return ""
    if len(text) >= 2 and text[0] == '"' and text[-1] == '"':
        body = text[1:-1]
        out: list[str] = []
        index = 0
        while index < len(body):
            char = body[index]
            if char == "\\" and index + 1 < len(body):
                nxt = body[index + 1]
                out.append({"n": "\n", "t": "\t", "\\": "\\", '"': '"'}.get(nxt, nxt))
                index += 2
                continue
            out.append(char)
            index += 1
        return "".join(out)
    if len(text) >= 2 and text[0] == "'" and text[-1] == "'":
        return text[1:-1].replace("''", "'")
    if text in ("null", "Null", "NULL", "~"):
        return None
    if text.lower() in ("true", "yes", "on"):
        return True
    if text.lower() in ("false", "no", "off"):
        return False
    try:
        return int(text)
    except ValueError:
        pass
    try:
        return float(text)
    except ValueError:
        pass
    return text


def _parse_yaml_block(lines: list[tuple[int, str]], index: int, indent: int) -> tuple[Any, int]:
    if index >= len(lines):
        return None, index
    if lines[index][1].startswith("- "):
        return _parse_yaml_sequence(lines, index, indent)
    return _parse_yaml_mapping(lines, index, indent)


def _parse_yaml_sequence(lines: list[tuple[int, str]], index: int, indent: int) -> tuple[list, int]:
    items: list[Any] = []
    while index < len(lines):
        line_indent, content = lines[index]
        if line_indent != indent or not content.startswith("- "):
            break
        rest = content[2:].strip()
        index += 1
        if rest == "":
            child, index = _parse_yaml_block(lines, index, indent + 2)
            items.append(child)
            continue
        pair = _split_key_value(rest)
        if pair is None:
            items.append(_parse_scalar(rest))
            continue
        key, raw = pair
        item: dict[str, Any] = {}
        if raw == "":
            if index < len(lines) and lines[index][0] > line_indent:
                child, index = _parse_yaml_block(lines, index, lines[index][0])
            else:
                child = None
            item[key] = child
        else:
            item[key] = _parse_scalar(raw)
        # 吸收同一 item 的后续键（缩进 = "- " 之后的位置）
        while index < len(lines):
            sub_indent, sub_content = lines[index]
            if sub_indent != indent + 2 or sub_content.startswith("- "):
                break
            sub_pair = _split_key_value(sub_content)
            if sub_pair is None:
                break
            sub_key, sub_raw = sub_pair
            index += 1
            if sub_raw == "":
                if index < len(lines) and lines[index][0] > sub_indent:
                    child, index = _parse_yaml_block(lines, index, lines[index][0])
                else:
                    child = None
                item[sub_key] = child
            else:
                item[sub_key] = _parse_scalar(sub_raw)
        items.append(item)
    return items, index


def _parse_yaml_mapping(lines: list[tuple[int, str]], index: int, indent: int) -> tuple[dict, int]:
    mapping: dict[str, Any] = {}
    while index < len(lines):
        line_indent, content = lines[index]
        if line_indent != indent or content.startswith("- "):
            break
        pair = _split_key_value(content)
        if pair is None:
            break
        key, raw = pair
        index += 1
        if raw == "":
            if index < len(lines) and lines[index][0] > line_indent:
                child, index = _parse_yaml_block(lines, index, lines[index][0])
            else:
                child = None
            mapping[key] = child
        else:
            mapping[key] = _parse_scalar(raw)
    return mapping, index


def parse_yaml(text: str) -> Any:
    """解析最小 YAML 子集（顶层 mapping / 嵌套 mapping / 列表 / 标量）。"""
    prepared: list[tuple[int, str]] = []
    for raw_line in text.splitlines():
        if not raw_line.strip() or raw_line.lstrip().startswith("#"):
            continue
        stripped = _strip_comment(raw_line)
        if not stripped.strip():
            continue
        prepared.append((len(stripped) - len(stripped.lstrip(" ")), stripped.strip()))
    if not prepared:
        return {}
    value, _ = _parse_yaml_block(prepared, 0, prepared[0][0])
    return value if value is not None else {}


def load_yaml(text: str) -> Any:
    """有 PyYAML 就用 PyYAML（更稳），没有就用内置子集解析器。"""
    try:
        import yaml  # type: ignore
    except Exception:
        return parse_yaml(text)
    try:
        return yaml.safe_load(text)
    except Exception:
        return parse_yaml(text)


# ---------------------------------------------------------------------------
# 分类
# ---------------------------------------------------------------------------

def _find_flaky_tests(lines: list[str]) -> list[tuple[str, int]]:
    """找出「同一测试名既判失败又判通过」的用例。"""
    seen: dict[str, dict[str, int]] = {}
    for number, line in enumerate(lines, 1):
        for pattern in _TEST_RESULT_RES:
            match = pattern.search(line)
            if not match:
                continue
            name = match.group("name")
            verdict = match.group("verdict")
            failed = "FAIL" in verdict.upper()
            passed = verdict.upper() in ("OK", "PASS")
            bucket = seen.setdefault(name, {})
            if failed:
                bucket["failed"] = bucket.get("failed", number)
            if passed:
                bucket["passed"] = bucket.get("passed", number)
            break
    flakies: list[tuple[str, int]] = []
    for name, bucket in seen.items():
        if "failed" in bucket and "passed" in bucket:
            flakies.append((name, min(bucket["failed"], bucket["passed"])))
    flakies.sort(key=lambda item: item[1])
    return flakies


def collect_signals(text: str, lines: list[str]) -> list[dict[str, Any]]:
    """扫描日志，返回所有命中的信号（pattern / matched_text / line / category）。"""
    signals: list[dict[str, Any]] = []
    for category, specs in CATEGORY_PATTERNS.items():
        for spec in specs:
            flags = spec.get("flags", re.IGNORECASE | re.MULTILINE)
            regex = re.compile(spec["re"], flags)
            requires = (
                re.compile(spec["requires"], re.IGNORECASE | re.MULTILINE)
                if spec.get("requires")
                else None
            )
            for number, line in enumerate(lines, 1):
                match = regex.search(line)
                if not match:
                    continue
                if requires is not None and not requires.search(text):
                    break
                signals.append(
                    {
                        "category": category,
                        "pattern": spec["re"],
                        "matched_text": match.group(0).strip(),
                        "line": number,
                        "level": spec["level"],
                        "note": spec["note"],
                    }
                )
                break
    # 「同一测试既失败又通过」—— 跨行信号
    for name, number in _find_flaky_tests(lines):
        signals.append(
            {
                "category": "flaky-test",
                "pattern": "同一测试名同时出现 FAILED 与 ok/PASS",
                "matched_text": name,
                "line": number,
                "level": "strong",
                "note": "同一用例既失败又通过",
            }
        )
    return signals


def _levels(signals: list[dict[str, Any]]) -> dict[str, int]:
    levels = {category: 0 for category in CATEGORY_ORDER}
    for signal in signals:
        value = LEVELS.get(signal["level"], 0)
        if value > levels.get(signal["category"], 0):
            levels[signal["category"]] = value
    return levels


def choose_category(signals: list[dict[str, Any]]) -> str:
    """按规范顺序 + 细化覆盖规则选唯一类别。"""
    levels = _levels(signals)
    candidates = {category for category, level in levels.items() if level > 0}
    if not candidates:
        return "unknown"

    for winner, losers, min_level in REFINEMENTS:
        if winner in candidates and levels.get(winner, 0) >= LEVELS[min_level]:
            for loser in losers:
                candidates.discard(loser)

    for winner, losers in HARD_ENV_OVERRIDES:
        if winner in candidates and levels.get(winner, 0) >= LEVELS["strong"]:
            for loser in losers:
                candidates.discard(loser)

    for category in CATEGORY_ORDER:
        if category in candidates:
            return category
    return "unknown"


def classify(text: str, source: str) -> dict[str, Any]:
    """把日志分类，返回 JSON schema 的字典。"""
    lines = text.splitlines()
    signals = collect_signals(text, lines)
    category = choose_category(signals)
    levels = _levels(signals)

    if category == "unknown":
        confidence = "low"
    elif levels.get(category, 0) >= LEVELS["strong"]:
        confidence = "high"
    elif levels.get(category, 0) >= LEVELS["medium"]:
        confidence = "medium"
    else:
        confidence = "low"

    counts: dict[str, int] = {}
    for signal in signals:
        counts[signal["category"]] = counts.get(signal["category"], 0) + 1

    return {
        "schema": SCHEMA,
        "category": category,
        "confidence": confidence,
        "signals": [
            {
                "pattern": signal["pattern"],
                "matched_text": signal["matched_text"],
                "line": signal["line"],
            }
            for signal in signals
        ],
        "next_action": NEXT_ACTIONS.get(category, NEXT_ACTIONS["unknown"]),
        "source": source,
        "counts": counts,
    }


# ---------------------------------------------------------------------------
# 知识库写入
# ---------------------------------------------------------------------------

def _entry_id(category: str, matched_text: str) -> str:
    """id = 类别 + 命中原文的稳定哈希（跨次运行稳定）。"""
    digest = hashlib.sha256(matched_text.encode("utf-8")).hexdigest()[:8]
    return f"{category}-{digest}"


def _primary_signal(result: dict[str, Any]) -> dict[str, Any] | None:
    """取主信号：优先命中原文最长者（信息量最大），其次首个。"""
    signals = result["signals"]
    if not signals:
        return None
    ordered = sorted(
        signals,
        key=lambda item: (-len(item["matched_text"]), item["line"] or 0),
    )
    return ordered[0]


def add_knowledge(result: dict[str, Any]) -> dict[str, Any]:
    """把本次命中追加成候选条目（按 id 幂等：同 id 只加 occurrences）。"""
    category = result["category"]
    if category == "unknown":
        return {"action": "skipped", "reason": "category=unknown，不入库"}

    signal = _primary_signal(result)
    if signal is None:
        return {"action": "skipped", "reason": "没有命中信号，不入库"}

    matched_text = signal["matched_text"]
    entry_id = _entry_id(category, matched_text)

    KNOWLEDGE_DIR.mkdir(parents=True, exist_ok=True)
    if PATTERNS_FILE.exists():
        document = load_yaml(PATTERNS_FILE.read_text(encoding="utf-8")) or {}
    else:
        document = {}
    if not isinstance(document, dict):
        document = {}
    document.setdefault("schema", PATTERNS_SCHEMA)
    patterns = document.get("patterns")
    if not isinstance(patterns, list):
        patterns = []
    document["patterns"] = patterns

    for entry in patterns:
        if isinstance(entry, dict) and entry.get("id") == entry_id:
            occurrences = entry.get("occurrences")
            entry["occurrences"] = (occurrences if isinstance(occurrences, int) else 0) + 1
            PATTERNS_FILE.write_text(dump_yaml(document), encoding="utf-8")
            return {"action": "incremented", "id": entry_id, "occurrences": entry["occurrences"]}

    entry = {
        "id": entry_id,
        "trigger": signal["pattern"],
        "failure": KB_FAILURE.get(category, category),
        "prevention": KB_PREVENTION.get(category, ""),
        "guard": "",
        "first_seen": date.today().isoformat(),
        "occurrences": 1,
        "category": category,
        "source": result.get("source", "stdin"),
    }
    patterns.append(entry)
    PATTERNS_FILE.write_text(dump_yaml(document), encoding="utf-8")
    return {"action": "added", "id": entry_id, "occurrences": 1}


# ---------------------------------------------------------------------------
# 输出
# ---------------------------------------------------------------------------

def render_human(result: dict[str, Any]) -> str:
    lines = [
        "─" * 60,
        f"失败类别 (category): {result['category']}",
        f"置信度 (confidence): {result['confidence']}",
        f"来源 (source): {result['source']}",
        f"命中信号 (signals): {len(result['signals'])}",
    ]
    if result["signals"]:
        for index, signal in enumerate(result["signals"], 1):
            line = signal.get("line")
            where = f"line {line}" if line else "line ?"
            lines.append(
                f"  {index:>2}. [{signal['pattern']}] "
                f"→ {signal['matched_text']!r} ({where})"
            )
    else:
        lines.append("  （无已知信号命中）")
    lines.append("下一步 (next action):")
    lines.append(f"  {result['next_action']}")
    return "\n".join(lines)


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------

def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        prog="harness_failures.py",
        description=(
            "把守卫/测试失败的日志 blob 结构化分类：类别 + 命中证据 + 下一步动作，"
            "并可把命中沉淀进 .harness/knowledge/failure-patterns.yaml。"
        ),
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=(
            "示例:\n"
            "  python3 scripts/harness_failures.py logs/ci.log\n"
            "  python3 scripts/harness_failures.py logs/ci.log --json\n"
            "  cat logs/ci.log | python3 scripts/harness_failures.py --stdin\n"
            "  python3 scripts/harness_failures.py logs/ci.log --add-knowledge\n"
        ),
    )
    parser.add_argument(
        "logfile",
        nargs="?",
        help="待分类的日志文件路径（与 --stdin 二选一）",
    )
    parser.add_argument(
        "--stdin",
        action="store_true",
        help="从标准输入读日志",
    )
    parser.add_argument(
        "--json",
        action="store_true",
        help="输出 JSON（schema: " + SCHEMA + "）",
    )
    parser.add_argument(
        "--add-knowledge",
        action="store_true",
        help="把本次命中追加为知识库候选条目（按 id 幂等）",
    )
    parser.add_argument(
        "--quiet",
        action="store_true",
        help="不打印人读报告（配合 --json 用）",
    )
    args = parser.parse_args(argv)

    if args.stdin and args.logfile:
        print("错误：--stdin 与日志文件路径不能同时给。", file=sys.stderr)
        return 1
    if not args.stdin and not args.logfile:
        print("错误：要给一个日志文件路径，或用 --stdin 从标准输入读。", file=sys.stderr)
        return 1

    try:
        if args.stdin:
            text = sys.stdin.read()
            source = "stdin"
        else:
            path = Path(args.logfile)
            if not path.is_file():
                print(f"错误：日志文件不存在：{path}", file=sys.stderr)
                return 1
            text = path.read_text(encoding="utf-8", errors="replace")
            source = str(path)
    except Exception as error:  # noqa: BLE001 - 顶层兜底 ⇒ exit 1
        print(f"错误：读取日志失败：{error}", file=sys.stderr)
        return 1

    try:
        result = classify(text, source)

        if args.add_knowledge:
            knowledge = add_knowledge(result)
            if not args.quiet and not args.json:
                print(f"知识库: {knowledge['action']}"
                      f"{' ' + knowledge.get('id', '') if knowledge.get('id') else ''}"
                      f"{'（' + knowledge.get('reason', '') + '）' if knowledge.get('reason') else ''}")
            if args.json:
                result["knowledge"] = knowledge

        if args.json:
            print(json.dumps(result, ensure_ascii=False, indent=2))
        elif not args.quiet:
            print(render_human(result))
    except Exception as error:  # noqa: BLE001 - 顶层兜底 ⇒ exit 1
        print(f"错误：分类失败：{error}", file=sys.stderr)
        return 1

    return 0


if __name__ == "__main__":
    sys.exit(main())
