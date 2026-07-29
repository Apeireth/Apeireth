"""V1122 模板加载器 (主 19:33 走在前人经验上 — 借鉴 V1011 PromptTemplate).

设计原则 (主 23:44 干到底 + 主 00:56 任何人都能接手):
- 零外部依赖: 仅用 stdlib pathlib + string.Formatter
- 模板即 .j2 文本: 文件名 slug, {var} 占位
- 渲染失败显式抛错: 缺失变量 / 越界 token 都拒绝静默

V3 守门 (主 17:58 + 主 20:46 + 主 17:43):
- 模板里不写"已达成 ASI"等不假装表述
- 模板渲染后必须仍包含 V3 guard 子串 (rendered_with_guard 守门)
"""
from __future__ import annotations

import re
from pathlib import Path
from string import Formatter
from typing import Any, Dict, List, Mapping

V1122_TPL_VERSION = "0.1.0"
TEMPLATE_DIR = Path(__file__).resolve().parent

# V3 守门: 渲染后所有 prompt 必须包含的子串 (主 17:58 不假装 + 主 20:46)
V3_GUARD_FRAGMENTS = (
    "不假装",  # 任意主 17:58 / 20:46 守门
    "ASI 北极星",  # 主 22:33 锚定
)

# 变量名合法字符 (防止 prompt injection 通过变量名注入控制符)
_SAFE_VAR = re.compile(r"^[a-zA-Z_][a-zA-Z0-9_]{0,63}$")


def list_templates() -> List[str]:
    """列出全部 .j2 模板文件名 (相对 TEMPLATE_DIR)."""
    return sorted(p.name for p in TEMPLATE_DIR.glob("*.j2"))


def load_template(name: str) -> str:
    """按名加载 .j2 文本, name 不带后缀亦可."""
    if not name.endswith(".j2"):
        name = f"{name}.j2"
    path = TEMPLATE_DIR / name
    if not path.is_file():
        raise FileNotFoundError(f"V1122 模板不存在: {name} (TEMPLATE_DIR={TEMPLATE_DIR})")
    return path.read_text(encoding="utf-8")


def _safe_vars(template: str) -> List[str]:
    """提取 {var} 形式的占位符 (忽略 {{...}} 转义)."""
    names: List[str] = []
    for _, field_name, _, _ in Formatter().parse(template):
        if field_name is None or field_name == "":
            continue
        # 不支持 {var.attr} 这类复杂字段 — V1122 简化为纯 {var}
        if "." in field_name or "[" in field_name:
            raise ValueError(f"V1122 不支持复杂占位: {field_name!r} (主 00:56 简化)")
        if not _SAFE_VAR.match(field_name):
            raise ValueError(f"V1122 变量名非法: {field_name!r}")
        names.append(field_name)
    return names


def _escape_user_input(value: str) -> str:
    """防止 prompt injection: 剥离常见注入标记."""
    # 主 17:43 实事求是 — 真做防护, 不假装.
    # 简单方案: 把易注入的控制符替换为转义文本
    bad = {
        "```": "ʼʼʼ",
        "<<": "‹‹",
        ">>": "››",
        "<|": "‹|",
        "|>": "|›",
        "IGNORE PREVIOUS": "[I-P REJECTED]",
        "DISREGARD ABOVE": "[D-A REJECTED]",
    }
    out = value
    for k, v in bad.items():
        out = out.replace(k, v)
    return out


def render_template(
    name: str,
    variables: Mapping[str, Any],
    *,
    max_tokens: int = 1024,
    guard: bool = True,
) -> str:
    """真渲染 (主 00:56 任何人能接手).

    Args:
        name: 模板名 (可省略 .j2 后缀)
        variables: 变量字典
        max_tokens: 渲染后 token 估算上限 (主 13:31 大胆激进 — 真加保护)
        guard: 是否强制 V3 guard 守门

    Raises:
        FileNotFoundError: 模板不存在
        KeyError: 变量缺失
        ValueError: 变量名非法 / token 越界 / guard 失败
    """
    tpl = load_template(name)
    expected = set(_safe_vars(tpl))
    provided = set(variables.keys())
    missing = expected - provided
    if missing:
        raise KeyError(f"V1122 模板 {name} 缺变量: {sorted(missing)}")
    extra = provided - expected
    # 多余变量静默丢弃, 不报 (主 00:56 简化)

    # 对所有 string 变量做 prompt injection 防护
    safe_vars: Dict[str, Any] = {}
    for k, v in variables.items():
        if isinstance(v, str):
            safe_vars[k] = _escape_user_input(v)
        else:
            safe_vars[k] = v

    rendered = tpl.format_map(safe_vars)

    # token 估算: 1 token ≈ 1.5 英文 / 0.7 中文字 (粗估, 主 17:43 实事求是)
    n_tokens = _estimate_tokens(rendered)
    if n_tokens > max_tokens:
        raise ValueError(
            f"V1122 渲染超 token 上限: {n_tokens} > {max_tokens} (主 13:31 真加保护)"
        )

    if guard:
        for frag in V3_GUARD_FRAGMENTS:
            if frag not in rendered:
                raise ValueError(
                    f"V1122 渲染缺失 V3 guard {frag!r} — 模板未声明守门 (主 17:58 不假装)"
                )
    return rendered


def _estimate_tokens(text: str) -> int:
    """粗估 token 数: 中文按 0.7 字符/token, 英文按 1.5 字符/token (主 17:43 实事求是)."""
    n_cn = sum(1 for c in text if "\u4e00" <= c <= "\u9fff")
    n_other = len(text) - n_cn
    return int(n_cn / 0.7 + n_other / 1.5) + 1
