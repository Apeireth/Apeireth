#!/usr/bin/env python3
"""**严格 YAML 重复键守卫**（2026-11-03 加，防 `cargo-audit.yml` 那类病复发）。

# 为什么需要这个门禁（**三次复发**的教训）
2026-11-03：`cargo-audit.yml` 的同一个 step 有**两处** `if: always()`
⇒ **YAML 同层重复键** ⇒ GitHub **拒绝解析整个文件**
⇒ ⇒ `Invalid workflow file: ... (Line: 162, Col: 9): 'if' is already defined`
⇒ ⇒ **一个 job 都创建不出来** ⇒ run **秒红**（看起来像「CI 挂了」，真因是**YAML 坏**）。

⚠️ **为什么本地常规检查验不出来**：
* `yaml.safe_load`（PyYAML）与 Ruby 的 YAML **都静默取最后一个键** ⇒ **不报错**；
* `bash -n` 只查 shell 脚本，查不出 YAML 结构问题；
⇒ ⇒ 所以这个病能**一路活过 CI 自检**，直到 GitHub 自己拒解析。

⇒ ⇒ 本守卫用**自定义 loader**：构造 mapping 时**记录所有重复键** ⇒ 命中即失败。

# 为什么值得单独做一个门禁
* 这个病**已经复发三轮**（`7146f4ad` → 两次各修一轮 → `62f33288` 合并又生成新的）；
* 合并引入的重复键 **git 看不见冲突**（两行位置不同 ⇒ 自动合并成功）；
* 后果是**整个 workflow 静默失效**（0 job）⇒ 「CI 绿」是假的。

# 用法
```bash
python3 scripts/check_yaml_duplicate_keys.py            # 检查全部 workflow
python3 scripts/check_yaml_duplicate_keys.py --check     # 同上（门禁语义）
```
退出码：0 = 无重复键；1 = 有重复键（逐条打印文件:行:列 与键名）。
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

# ⭐ **控制台可能不是 UTF-8**（Windows：`cp1252` / `cp936`）⇒ ⇒ `print` 会抛
# `UnicodeEncodeError` ⇒ ⇒ 门禁 exit 1 ⇒ ⇒ **把本来通过的 CI job 判成红**
# ⇒ ⇒ 这里在**任何打印之前**把 stdout/stderr 重配成 UTF-8（守卫见 `scripts/utf8_stdout.py`）。
sys.path.insert(0, str(Path(__file__).resolve().parent))
from utf8_stdout import force_utf8_stdout  # noqa: E402

force_utf8_stdout()

try:
    import yaml
except ImportError:  # pragma: no cover - 环境缺依赖时给出可执行的提示
    print("需要 PyYAML：python3 -m pip install pyyaml", file=sys.stderr)
    raise SystemExit(2)

ROOT = Path(__file__).resolve().parent.parent
WORKFLOW_DIR = ROOT / ".github" / "workflows"


class DuplicateKeyError(Exception):
    """mapping 里出现了重复键。"""


class StrictLoader(yaml.SafeLoader):
    """静默丢键的 loader 换成「重复即报错」的 loader。"""


def _construct_mapping(loader: StrictLoader, node: yaml.MappingNode, deep: bool = False):
    mapping: dict = {}
    for key_node, value_node in node.value:
        key = loader.construct_object(key_node, deep=deep)
        if key in mapping:
            raise DuplicateKeyError(
                f"重复键 {key!r}（行 {key_node.start_mark.line + 1}，"
                f"列 {key_node.start_mark.column + 1}）"
            )
        mapping[key] = loader.construct_object(value_node, deep=deep)
    return mapping


StrictLoader.add_constructor(
    yaml.resolver.BaseResolver.DEFAULT_MAPPING_TAG, _construct_mapping
)


def scan(path: Path) -> list[str]:
    """返回该文件的重复键问题列表（空 = 干净）。"""
    try:
        text = path.read_text(encoding="utf-8")
    except OSError as error:
        return [f"{path}: 读取失败：{error}"]
    try:
        yaml.load(text, Loader=StrictLoader)
    except DuplicateKeyError as error:
        return [f"{path}: {error}"]
    except yaml.YAMLError as error:
        # 语法错误也归本门禁管（GitHub 同样会拒解析）⇒ 但要区分措辞。
        return [f"{path}: YAML 语法错误 ⇒ GitHub 会拒解析本文件：{error}"]
    return []


def main() -> int:
    parser = argparse.ArgumentParser(description="严格 YAML 重复键守卫")
    parser.add_argument("--check", action="store_true", help="门禁语义（与无参同）")
    parser.add_argument(
        "--path",
        action="append",
        default=[],
        help="要扫的目录或文件（可重复）。缺省扫 .github/workflows。",
    )
    args = parser.parse_args()

    targets = [Path(item) for item in args.path] or [WORKFLOW_DIR]
    files: list[Path] = []
    for target in targets:
        if target.is_dir():
            files.extend(sorted(target.glob("*.yml")))
            files.extend(sorted(target.glob("*.yaml")))
        elif target.is_file():
            files.append(target)
        else:
            print(f"路径不存在：{target}", file=sys.stderr)
            return 1

    if not files:
        print("没有 workflow 文件", file=sys.stderr)
        return 1

    problems: list[str] = []
    for path in files:
        problems.extend(scan(path))

    if problems:
        print(f"FAIL: YAML 重复键 / 语法问题（{len(problems)} 处）")
        for item in problems:
            print(f"  · {item}")
        print(
            "\n⚠️ GitHub 遇到重复键会**拒绝解析整个文件** ⇒ 该 workflow 静默失效"
            "（0 job ⇒ run 秒红）。"
        )
        print("⚠️ `yaml.safe_load` 与 Ruby 的 YAML 都**静默取最后一个键** ⇒ 本门禁必须用严格 loader。")
        return 1

    print(f"OK: {len(files)} 个 workflow 无重复键 / 语法错误")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
