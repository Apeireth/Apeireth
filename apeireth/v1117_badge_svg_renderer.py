"""V1117 CI badge SVG renderer + cross-model diff viz + HF cache timeout + env config.

R9-DEV-003 / R9-DevOps W4 收尾.

主 22:33 ASI 北极星 (终极梦想: 让任何 LLM 接入即获 AGI/ASI 能力 — W4 把"量化"做成可视化)
主 17:43 实事求是 (SVG/HTML 数据全从 HarnessResult 真测, 不 hardcode)
主 13:31 大胆激进 (跨模型差异 SVG/HTML 图表 + badge SVG 自动渲染)
主 00:56 任何人都能接手 (`render_badge_svg` / `load_env_file` 一行可调)
主 19:33 走在前人经验上:
  - shields.io 2014 endpoint badge schema
  - d3-compare 2017 cross-table (HTML)
  - HF transformers Cache 2018 (此处封装 + 超时, 避免 CI hang 在大模型)
  - python-dotenv 2014 (env file 加载惯例)
主 23:44 干到底 (≥5 个真功能, 真测真产)
主 17:58+20:46 不假装 (HF 加载超时 → 显式 TimeoutError, 不假装 PASS)

Public API:
    render_badge_svg(label, message, color, style="flat") -> str
    render_status_badge(status, message) -> str  # GREEN/YELLOW/RED/UNKNOWN 显式
    render_diff_svg(diff_data, width=720, height=320) -> str  # 跨模型 diff 柱状图
    render_diff_html(diff_data, title="Cross-Model Diff") -> str  # 跨模型 diff HTML
    HFModelCache(load_fn, timeout_sec=30) -> HFModelCache  # 加载 + 超时
    load_env_file(path="apeireth.env") -> Dict[str, str]
    write_env_file(values, path="apeireth.env") -> Path
    REAL_MODEL_ENV: Dict[str, str]  # 5 env var → path 约定
    COLOR_MAP: Dict[str, str]  # status → shield color
"""
from __future__ import annotations

import html
import json
import os
import re
import signal
import threading
from contextlib import contextmanager
from pathlib import Path
from typing import Any, Callable, Dict, List, Optional, Sequence, Tuple, TypeVar


T = TypeVar("T")


# ---------------------------------------------------------------------------
# 1. Badge SVG 渲染 (主 13:31 大胆激进: shields.io SVG 显式输出, 不依赖外网)
# ---------------------------------------------------------------------------
# 借鉴 shields.io 2014 endpoint schema; 这里直接生成 SVG (避免外网依赖)
COLOR_MAP: Dict[str, str] = {
    "GREEN": "#4c1",       # shields brightgreen ≈ #4c1
    "green": "#4c1",
    "YELLOW": "#dfb317",
    "yellow": "#dfb317",
    "RED": "#e05d44",
    "red": "#e05d44",
    "UNKNOWN": "#9f9f9f",
    "unknown": "#9f9f9f",
    "lightgrey": "#9f9f9f",
    "pass": "#4c1",
    "fail": "#e05d44",
    "mixed": "#dfb317",
}

# shields.io style colors 与 status 显式映射
STATUS_TO_COLOR: Dict[str, str] = {
    "pass": COLOR_MAP["GREEN"],
    "fail": COLOR_MAP["RED"],
    "mixed": COLOR_MAP["YELLOW"],
    "unknown": COLOR_MAP["UNKNOWN"],
}


def _svg_text_width(text: str, char_w: float = 6.5) -> float:
    """估算文本宽度 (主 00:56: 不引外 font 库, 用粗略像素估)."""
    # 简单估算: 每个 ASCII 字符 ~6.5px, CJK ~13px
    w = 0.0
    for ch in text:
        if ord(ch) > 0x2E80:  # CJK 范围
            w += 13.0
        else:
            w += 6.5
    return w


def render_badge_svg(label: str = "ci",
                     message: str = "n/a",
                     color: str = "#4c1",
                     style: str = "flat",
                     font_family: str = "Verdana,Geneva,DejaVu Sans,sans-serif") -> str:
    """CI badge SVG 字符串 (主 13:31 大胆激进 + 主 00:56 任何人都能接手).

    借鉴 shields.io 2014 endpoint: <svg> 内含 <rect> <text> + <linearGradient> + shadow.
    颜色支持 #hex / 命名色 (green/yellow/red 等), 显式映射表 COLOR_MAP.

    Args:
        label: 左侧文本 (e.g. "ci")
        message: 右侧文本 (e.g. "12/13 pass")
        color: 十六进制色 / 命名色 (green/yellow/red/unknown)
        style: flat (默认) / flat-square / plastic
    """
    if color in COLOR_MAP:
        hex_color = COLOR_MAP[color]
    elif re.match(r"^#[0-9a-fA-F]{3,6}$", color):
        hex_color = color
    else:
        hex_color = COLOR_MAP["UNKNOWN"]

    label_w = _svg_text_width(label) + 10
    msg_w = _svg_text_width(message) + 10
    total_w = label_w + msg_w
    h = 28 if style == "flat-square" else 20

    label_x = label_w / 2
    msg_x = label_w + msg_w / 2
    gradient_id = f"v1117_{abs(hash((label, message, color, style))) % 10**8}"
    text_anchor_label = label_x
    text_anchor_msg = msg_x
    rect_label_y = h
    rect_msg_y = h
    rect_msg_x = label_w

    # shields.io style with subtle gradient
    extra_defs = ""
    label_fill = "#555"
    msg_fill = hex_color
    if style == "plastic":
        label_fill = "#555"
        msg_fill = hex_color
    elif style == "flat-square":
        label_fill = "#444"
        msg_fill = hex_color
    else:
        # flat (default) — 加 gradient
        extra_defs = (
            f'<linearGradient id="{gradient_id}_l" x2="0" y2="100%">'
            f'<stop offset="0" stop-color="#fff" stop-opacity=".7"/>'
            f'<stop offset=".1" stop-color="#fff" stop-opacity=".15"/>'
            f'<stop offset=".9" stop-color="#000" stop-opacity=".15"/>'
            f'<stop offset="1" stop-color="#000" stop-opacity=".35"/>'
            f'</linearGradient>'
            f'<linearGradient id="{gradient_id}_r" x2="0" y2="100%">'
            f'<stop offset="0" stop-color="#fff" stop-opacity=".7"/>'
            f'<stop offset=".1" stop-color="#fff" stop-opacity=".15"/>'
            f'<stop offset=".9" stop-color="#000" stop-opacity=".15"/>'
            f'<stop offset="1" stop-color="#000" stop-opacity=".35"/>'
            f'</linearGradient>'
        )

    return (
        f'<svg xmlns="http://www.w3.org/2000/svg" '
        f'xmlns:xlink="http://www.w3.org/1999/xlink" '
        f'width="{total_w:.0f}" height="{h}" role="img" '
        f'aria-label="{html.escape(label)}: {html.escape(message)}">'
        f'<title>{html.escape(label)}: {html.escape(message)}</title>'
        f'<defs>{extra_defs}</defs>'
        f'<clipPath id="r">'
        f'<rect width="{total_w:.0f}" height="{h}" rx="{3 if style != "flat-square" else 0}" '
        f'fill="#fff"/>'
        f'</clipPath>'
        f'<g clip-path="url(#r)">'
        f'<rect width="{label_w:.0f}" height="{h}" fill="{label_fill}"/>'
        f'<rect x="{rect_msg_x:.0f}" width="{msg_w:.0f}" height="{h}" fill="{msg_fill}"/>'
        f'<rect width="{label_w:.0f}" height="{h}" fill="url(#{gradient_id}_l)"/>'
        f'<rect x="{rect_msg_x:.0f}" width="{msg_w:.0f}" height="{h}" '
        f'fill="url(#{gradient_id}_r)"/>'
        f'</g>'
        f'<g fill="#fff" text-anchor="middle" '
        f'font-family=\'{font_family}\' font-size="11" '
        f'transform="translate(0,{h / 2 + 4})">'
        f'<text x="{text_anchor_label:.1f}" y="0">{html.escape(label)}</text>'
        f'<text x="{text_anchor_msg:.1f}" y="0">{html.escape(message)}</text>'
        f'</g>'
        f'</svg>'
    )


def render_status_badge(status: str, message: str = "",
                        label: str = "cross-small-model-ci") -> str:
    """Status badge SVG: pass/mixed/fail/unknown → GREEN/YELLOW/RED/UNKNOWN 显式映射.

    主 13:31 大胆激进: 4 个状态显式映射, 不会混淆颜色含义.
    主 17:43 实事求是: status 来自真测 (render_badge 或外部传入), 不 hardcode.
    """
    s = status.lower().strip()
    color = STATUS_TO_COLOR.get(s, STATUS_TO_COLOR["unknown"])
    if not message:
        message = s.upper()
    return render_badge_svg(label=label, message=message, color=color, style="flat")


# ---------------------------------------------------------------------------
# 2. 跨模型差异 SVG/HTML 可视化 (主 13:31 大胆激进: 三向对比图表)
# ---------------------------------------------------------------------------
def render_diff_svg(diff_data: Dict[str, Any],
                    width: int = 720,
                    height: int = 320,
                    metric: str = "delta_subscore") -> str:
    """跨模型差异 SVG 柱状图 (主 13:31 + 主 00:56).

    Args:
        diff_data: compute_diff() 输出的 dict (含 rows / lift_summary)
        width/height: SVG 尺寸
        metric: 取哪一维 (delta_sc/delta_nr/delta_ev/delta_cdt/delta_subscore)
    """
    rows = diff_data.get("rows", []) or []
    lift = diff_data.get("lift_summary", {}) or {}
    if not rows:
        return (
            f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" '
            f'role="img"><text x="20" y="40" fill="#c00">no rows in diff</text></svg>'
        )

    # 过滤有效 delta
    plot_rows = []
    for r in rows:
        v = r.get(metric)
        if isinstance(v, (int, float)):
            plot_rows.append((str(r.get("target", "?")), float(v), bool(r.get("available"))))
    if not plot_rows:
        return (
            f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" '
            f'role="img"><text x="20" y="40" fill="#c00">'
            f'no valid {metric} values</text></svg>'
        )

    # 计算 0 轴位置 + 缩放
    max_abs = max(abs(v) for _, v, _ in plot_rows) or 1.0
    pad_l, pad_r = 110, 30
    pad_t, pad_b = 30, 50
    plot_w = width - pad_l - pad_r
    plot_h = height - pad_t - pad_b
    y0 = pad_t + plot_h / 2  # 零轴

    # 柱宽 (按 row 数分配)
    n = len(plot_rows)
    bar_h = max(10.0, plot_h / n - 4.0)
    bars_svg: List[str] = []
    for i, (name, val, avail) in enumerate(plot_rows):
        bar_w = (val / max_abs) * (plot_w * 0.45) if max_abs > 0 else 0
        # 主 17:43: 正值 green, 负值 red, 0 grey
        if val > 0:
            fill = COLOR_MAP["GREEN"]
        elif val < 0:
            fill = COLOR_MAP["RED"]
        else:
            fill = COLOR_MAP["UNKNOWN"]
        y = pad_t + (i + 0.5) * (plot_h / n) - bar_h / 2
        x = pad_l + (plot_w * 0.5) if val >= 0 else pad_l + (plot_w * 0.5) + bar_w
        # unreachable negative rebase
        if val < 0:
            x = pad_l + (plot_w * 0.5)
            rect_x = x + bar_w
            rect_w = -bar_w
        else:
            rect_x = pad_l + (plot_w * 0.5)
            rect_w = bar_w
        avail_mark = "✓" if avail else "✗"
        bars_svg.append(
            f'<rect x="{rect_x:.1f}" y="{y:.1f}" width="{rect_w:.1f}" height="{bar_h:.1f}" '
            f'fill="{fill}" opacity="0.85"><title>{html.escape(name)}: {val:+.4f} '
            f'available={avail}</title></rect>'
        )
        # 标签 + 值
        ty = pad_t + (i + 0.5) * (plot_h / n) + 4
        bars_svg.append(
            f'<text x="{pad_l - 8:.0f}" y="{ty:.1f}" text-anchor="end" '
            f'font-size="11" fill="#222">{html.escape(name)} {avail_mark}</text>'
        )
        bars_svg.append(
            f'<text x="{rect_x + rect_w + (4 if rect_w > 0 else -4):.1f}" y="{ty:.1f}" '
            f'text-anchor="{"start" if rect_w > 0 else "end"}" font-size="11" '
            f'fill="#333">{val:+.4f}</text>'
        )
    # 零轴
    zero_axis = (
        f'<line x1="{pad_l + plot_w * 0.5:.1f}" y1="{pad_t}" '
        f'x2="{pad_l + plot_w * 0.5:.1f}" y2="{pad_t + plot_h:.1f}" '
        f'stroke="#222" stroke-width="1"/>'
    )
    # 标题 + lift_summary
    title_y = 18
    title = (
        f'Cross-Model Diff · {metric} '
        f'(baseline={lift.get("baseline_name", "?")}, '
        f'n_loaded={lift.get("n_loaded", 0)}, '
        f'n_failed={lift.get("n_failed", 0)})'
    )
    title_svg = (
        f'<text x="{pad_l}" y="{title_y}" font-size="13" font-weight="bold" '
        f'fill="#222">{html.escape(title)}</text>'
    )

    return (
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" '
        f'role="img" aria-label="cross-model-diff">'
        f'{title_svg}{zero_axis}{"".join(bars_svg)}'
        f'</svg>'
    )


def render_diff_html(diff_data: Dict[str, Any],
                     title: str = "Cross-Model Diff (W4)",
                     embed_svg: bool = True) -> str:
    """跨模型差异 HTML 页面 (主 00:56 任何人都能接手: 浏览器打开即看).

    embed_svg=True → SVG 嵌 HTML (单文件可读); 否则只放表格.
    """
    rows = diff_data.get("rows", []) or []
    lift = diff_data.get("lift_summary", {}) or []
    if isinstance(lift, list):
        lift = {}

    safe_title = html.escape(title)
    if embed_svg:
        svg_block = render_diff_svg(diff_data)
    else:
        svg_block = ""

    # 表格行
    body_rows: List[str] = []
    for r in rows:
        target = html.escape(str(r.get("target", "?")))
        fam = html.escape(str(r.get("family", "?")))
        avail = "✅" if r.get("available") else "❌"
        color = COLOR_MAP["GREEN"] if r.get("available") else COLOR_MAP["UNKNOWN"]

        def _fmt(v: Any) -> str:
            if isinstance(v, (int, float)):
                return f"{v:+.4f}"
            return "—"

        note = html.escape(str(r.get("error") or ""))
        body_rows.append(
            f'<tr><td>{target}</td><td>{fam}</td>'
            f'<td style="text-align:center;color:{color};font-weight:bold">{avail}</td>'
            f'<td>{_fmt(r.get("delta_sc"))}</td>'
            f'<td>{_fmt(r.get("delta_nr"))}</td>'
            f'<td>{_fmt(r.get("delta_ev"))}</td>'
            f'<td>{_fmt(r.get("delta_cdt"))}</td>'
            f'<td><b>{_fmt(r.get("delta_subscore"))}</b></td>'
            f'<td style="color:#666;font-size:12px">{note}</td></tr>'
        )
    table = (
        '<table border="1" cellpadding="6" cellspacing="0" '
        'style="border-collapse:collapse;font-family:sans-serif;font-size:13px">'
        '<thead style="background:#f4f4f4">'
        '<tr><th>target</th><th>family</th><th>available</th>'
        '<th>ΔSC</th><th>ΔNR</th><th>ΔEV</th><th>ΔCDT</th>'
        '<th>Δsubscore</th><th>note</th></tr>'
        '</thead>'
        f'<tbody>{"".join(body_rows) or "<tr><td colspan=9>(no rows)</td></tr>"}</tbody>'
        '</table>'
    )

    mean = lift.get("mean_delta")
    mx = lift.get("max_delta")
    mn = lift.get("min_delta")
    summary = (
        f"<p><b>lift_summary</b>: "
        f"n_targets={lift.get('n_targets', 0)} "
        f"n_loaded={lift.get('n_loaded', 0)} "
        f"n_failed={lift.get('n_failed', 0)} "
        f"mean={f'{mean:+.4f}' if isinstance(mean, (int, float)) else '—'} "
        f"max={f'{mx:+.4f}' if isinstance(mx, (int, float)) else '—'} "
        f"min={f'{mn:+.4f}' if isinstance(mn, (int, float)) else '—'} "
        f"baseline={lift.get('baseline_name', '?')}</p>"
    )

    return (
        f'<!DOCTYPE html><html><head><meta charset="utf-8">'
        f'<title>{safe_title}</title></head>'
        f'<body style="margin:24px;font-family:Verdana,sans-serif;color:#222">'
        f'<h1>{safe_title}</h1>'
        f'{summary}'
        f'{("<div>" + svg_block + "</div>") if svg_block else ""}'
        f'<h2>Table</h2>{table}'
        f'<hr><p style="color:#888;font-size:11px">'
        f'Generated by apeireth.v1117_badge_svg_renderer '
        f'(主 13:31 大胆激进 · 主 17:43 实事求是)</p>'
        f'</body></html>'
    )


# ---------------------------------------------------------------------------
# 3. HF Model Cache 超时控制 (主 17:58 不假装: 大模型 CI 不 hang)
# ---------------------------------------------------------------------------
class HFModelTimeoutError(TimeoutError):
    """HF model 加载超时显式异常 (主 17:58+20:46 不假装 + 主 23:44 干到底)."""


class HFModelCache:
    """HF model 缓存 + 加载超时控制 (主 13:31 大胆激进 + 主 17:58 不假装).

    借鉴 HF transformers Cache 2018 概念 + threading.Timer 控制 wall-clock 上限.

    设计目标:
      - CI 跑真模型时, 单次加载不超过 timeout_sec.
      - 加载线程 hang → 主线程抛 HFModelTimeoutError, 不会 spawn 巨内存无限等.
      - 缓存加载结果 (cache: True), 后续 instantiate 直接走缓存, 0 加载耗时.

    Usage:
        cache = HFModelCache(timeout_sec=20)
        model = cache.get_or_load(lambda: AutoModel.from_pretrained(path))
        # model is now live; subsequent calls (cache=True, default) return same instance.

    Threading model: uses daemon Thread + Event.wait(timeout); Platform: POSIX/Windows both
    supported (using threading.Event.wait, not signal-based kill — 跨平台).
    """

    def __init__(self, timeout_sec: float = 30.0, cache: bool = True):
        self.timeout_sec = float(timeout_sec)
        self.cache = bool(cache)
        self._value: Any = None
        self._error: Optional[BaseException] = None
        self._loaded: bool = False
        self._lock = threading.Lock()
        self.elapsed_ms: float = 0.0

    def _runner(self, load_fn: Callable[[], T]) -> None:
        try:
            self._value = load_fn()
        except BaseException as e:  # noqa: BLE001 - record any error verbatim
            self._error = e

    def get_or_load(self, load_fn: Callable[[], T]) -> T:
        """同步加载 (带超时). 异常 / 超时 → 抛 HFModelTimeoutError / 原始异常."""
        if self.cache and self._loaded:
            if self._error is not None:
                raise self._error
            return self._value  # type: ignore[return-value]

        with self._lock:
            if self.cache and self._loaded:
                if self._error is not None:
                    raise self._error
                return self._value  # type: ignore[return-value]

            import time
            t0 = time.time()
            self._value = None
            self._error = None
            th = threading.Thread(target=self._runner, args=(load_fn,), daemon=True)
            th.start()
            th.join(timeout=self.timeout_sec)
            self.elapsed_ms = (time.time() - t0) * 1000.0
            still_alive = th.is_alive()
            if still_alive:
                # 超时 — daemon thread 自然 GC, 主线程不再等
                self._loaded = True
                self._error = HFModelTimeoutError(
                    f"HFModelCache: load_fn did not return within {self.timeout_sec}s "
                    f"(主 17:58 不假装: 大模型加载超时 → 显式 TimeoutError, "
                    f"不会 hang CI). thread still alive."
                )
                raise self._error
            self._loaded = True
            if self._error is not None:
                raise self._error
            return self._value  # type: ignore[return-value]

    def reset(self) -> None:
        """清缓存 (重置后下次 get_or_load 重跑 load_fn)."""
        with self._lock:
            self._value = None
            self._error = None
            self._loaded = False
            self.elapsed_ms = 0.0


# ---------------------------------------------------------------------------
# 4. REAL_MODEL_ENV 配置 (主 00:56 任何人都能接手 + 主 00:44 质量工程化)
# ---------------------------------------------------------------------------
# 5 个 env var 约定 (主 00:56: 通过 env 注入路径, 不 hardcode)
REAL_MODEL_ENV: Dict[str, str] = {
    "qwen": "APEIRETH_QWEN35_PATH",
    "llama": "APEIRETH_LLAMA31_PATH",
    "hermes": "APEIRETH_HERMES_PATH",
    "gemma": "APEIRETH_GEMMA4_PATH",
    "embedding": "APEIRETH_EMBEDDING_PATH",
}

DEFAULT_ENV_FILE = "apeireth.env"


def _parse_env_text(text: str) -> Dict[str, str]:
    """解析 env 文件格式 (KEY=VALUE, # 注释, 空行忽略).

    借鉴 python-dotenv 2014 + .env 惯例.
    """
    out: Dict[str, str] = {}
    for line in text.splitlines():
        s = line.strip()
        if not s or s.startswith("#"):
            continue
        if "=" not in s:
            continue
        k, v = s.split("=", 1)
        k = k.strip()
        v = v.strip()
        # 去引号
        if len(v) >= 2 and v[0] == v[-1] and v[0] in ("'", '"'):
            v = v[1:-1]
        if k:
            out[k] = v
    return out


def _serialize_env_text(values: Dict[str, str]) -> str:
    """写 env 文件 (KEY=VALUE + 注释头)."""
    lines = [
        "# Apeireth (阿佩瑞斯) — AI 基座平台 env config (R9-DEV-003 / V1117 W4)",
        "# 借鉴 python-dotenv 2014 .env 惯例. 主 00:56 任何人都能接手.",
        "# 5 env var 注入真模型路径; 未注入 → CI 用 fixture (主 17:58 不假装).",
        "",
    ]
    for family, env_key in REAL_MODEL_ENV.items():
        v = values.get(env_key, "")
        lines.append(f"# {family}: {env_key} (本地路径)")
        lines.append(f"{env_key}={v}")
        lines.append("")
    return "\n".join(lines) + "\n"


def load_env_file(path: str | Path = DEFAULT_ENV_FILE) -> Dict[str, str]:
    """加载 env 文件; 不存在 → 返空 dict (主 17:58 不假装: 缺文件不假装有值).

    不会向 os.environ 写入, 只返 dict; 调用方自己 os.environ.update().
    """
    p = Path(path)
    if not p.exists():
        return {}
    return _parse_env_text(p.read_text(encoding="utf-8"))


def write_env_file(values: Dict[str, str],
                   path: str | Path = DEFAULT_ENV_FILE) -> Path:
    """写 env 文件 (主 00:44 质量工程化: 配置可固化 / gitignored 不强制).

    Args:
        values: partial dict (填几个 key 即可; 缺 key 留空)
        path: apeireth.env 路径
    """
    p = Path(path)
    p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text(_serialize_env_text(values), encoding="utf-8")
    return p


def apply_env_file(path: str | Path = DEFAULT_ENV_FILE,
                   override: bool = False) -> Dict[str, str]:
    """加载 env 文件并 (可选) 应用到 os.environ.

    override=False (默认) → 不覆盖已存在的 env var (主 17:58 不假装: 现实 env 优先).
    """
    data = load_env_file(path)
    if not override:
        for k, v in data.items():
            os.environ.setdefault(k, v)
    else:
        for k, v in data.items():
            os.environ[k] = v
    return data


# ---------------------------------------------------------------------------
# 5. 工具: dict 风格 lift summary 比较
# ---------------------------------------------------------------------------
def render_badge_history_svg(history: Sequence[Tuple[str, str]],
                              label: str = "ci-history") -> str:
    """历史 CI badge 串接 (主 13:31 大胆激进: 走势一目了然).

    Args:
        history: [(label_suffix, "pass"/"fail"/"mixed"/"unknown")] 列表
        label: 主标签
    """
    if not history:
        return render_badge_svg(label=label, message="0/0", color=COLOR_MAP["UNKNOWN"])
    parts: List[str] = []
    x_offset = 0.0
    for suffix, status in history:
        s_label = f"{label}-{suffix}"
        s_msg = status
        s_color = STATUS_TO_COLOR.get(status, STATUS_TO_COLOR["unknown"])
        # 直接调用 render_badge_svg, 但需要累积 x 偏移
        # 为简化, 把每个 badge 内嵌到一个外层 svg
        inner = render_badge_svg(label=s_label, message=s_msg, color=s_color)
        # 内嵌 svg: 计算 inner width via 文本估值
        lw = _svg_text_width(s_label) + 10
        mw = _svg_text_width(s_msg) + 10
        tw = lw + mw
        parts.append(
            f'<g transform="translate({x_offset:.0f},0)">{inner}</g>'
        )
        x_offset += tw + 4
    total_w = x_offset + 4
    return (
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{total_w:.0f}" height="20" '
        f'role="img"><g>{"".join(parts)}</g></svg>'
    )


# ---------------------------------------------------------------------------
# __all__
# ---------------------------------------------------------------------------
__all__ = [
    # 1. badge SVG
    "COLOR_MAP", "STATUS_TO_COLOR",
    "render_badge_svg", "render_status_badge", "render_badge_history_svg",
    # 2. diff viz
    "render_diff_svg", "render_diff_html",
    # 3. HF cache + timeout
    "HFModelCache", "HFModelTimeoutError",
    # 4. env config
    "REAL_MODEL_ENV", "DEFAULT_ENV_FILE",
    "load_env_file", "write_env_file", "apply_env_file",
    # module
    "__version__",
]

__version__ = "0.1.0"
