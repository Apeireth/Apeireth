#!/usr/bin/env python3
"""通用「**交互对照**」工具（键入 / 改值 ⇒ 两侧逐字比对）。

## 为什么需要它

2026-10-25 实测到一个**架构级缺陷**：预算面的状态对象**自建了一个独立的 `Runtime`**
⇒ ⇒ 草稿信号**不在**宿主挂 effect 的那个 runtime 上 ⇒ ⇒ **effect 根本不依赖它**
⇒ ⇒ 改草稿**不触发重渲染** ⇒ ⇒ 实测表现为「**幽灵输入**」：
用户敲进去的值**引擎不知道**（DOM 上显示的是浏览器自己画的，引擎草稿仍是 `""`）。

⇒ ⇒ **静态像素对照测不到这类缺陷**（画面对 ⇒ 因为浏览器自己画了值）
⇒ ⇒ 必须有**真实键入**的对照 ⇒ ⇒ 本模块就是那条通路。

## 为什么做成「通用模块」

每个面各写一遍键入逻辑 ⇒ ⇒ 会**漂**（一个面改了、另一个面没改）
⇒ ⇒ 本模块把「两侧同路径键入 ⇒ 读回 ⇒ 逐字比对」做成**一份实现**，各面只给
「页面路径 + 选择器 + 要键入的文本」。

## 口径

- **两侧同路径**：同一个 URL 结构（参考侧 `/reference/<面>/`、引擎侧 `/engine/<面>/`）、
  同一个选择器、同一个键入文本 ⇒ ⇒ 差异只反映**渲染与状态**。
- **键入用真实键盘事件**（`Input.dispatchKeyEvent`）⇒ ⇒ 走**真实**的事件路径
  ⇒ ⇒ 能触发「引擎接管 / 不接管」的差别。
- **失焦用 `blur()`** ⇒ ⇒ 触发生产的「整组失焦提交」。
- **读回三样**：① 控件当前 `value`；② 控件的 `aria-label`（定位用）；
  ③ 页面上与该字段相关的**回显文本**（若页面有）⇒ ⇒ 用来验证「引擎算出了同样的东西」。
"""

from __future__ import annotations

import json
import time
from typing import Any

#: 键盘事件里「字符」与「键」的映射（`dispatchKeyEvent` 需要逐个给）。
_KEY_EVENTS = {
    "7": ("Digit7", 55),
    "9": ("Digit9", 57),
    "0": ("Digit0", 48),
    "a": ("KeyA", 65),
    "b": ("KeyB", 66),
    "c": ("KeyC", 67),
    "t": ("KeyT", 84),
    "p": ("KeyP", 80),
    "-": ("Minus", 189),
}


def type_text(cdp, text: str) -> None:
    """用**真实键盘事件**键入一段文本。"""
    for char in text:
        code, key_code = _KEY_EVENTS.get(char.lower(), ("Key" + char.upper(), ord(char.upper())))
        cdp.call(
            "Input.dispatchKeyEvent",
            {
                "type": "keyDown",
                "text": char,
                "unmodifiedText": char,
                "key": char,
                "code": code,
                "windowsVirtualKeyCode": key_code,
                "nativeVirtualKeyCode": key_code,
            },
        )
        cdp.call(
            "Input.dispatchKeyEvent",
            {
                "type": "keyUp",
                "key": char,
                "code": code,
                "windowsVirtualKeyCode": key_code,
                "nativeVirtualKeyCode": key_code,
            },
        )
        time.sleep(0.05)


def read_control(cdp, selector: str) -> dict:
    """读控件的当前状态（`value` / `type` / `aria-label`）。"""
    return cdp.evaluate(
        "(() => { const el = document.querySelector(%s);"
        " if (!el) return null;"
        " return {value: el.value === undefined ? null : el.value,"
        "         type: el.getAttribute('type'),"
        "         label: el.getAttribute('aria-label')}; })()" % json.dumps(selector)
    )


def echo_text(cdp, selector: str) -> str:
    """读控件所在行（或其最近祖先）的**文本回显**（若页面有）⇒ 验证「引擎算出了同样的东西」。"""
    return cdp.evaluate(
        "(() => { const el = document.querySelector(%s);"
        " if (!el) return '';"
        " const row = el.closest('.cap-input-wrap, .row, .setting-row, tr, label') || el.parentElement;"
        " return row ? (row.innerText || '').trim() : ''; })()" % json.dumps(selector)
    )


def type_into_pair(
    cdp,
    navigate,
    sides: tuple[str, str],
    selector: str,
    text: str,
    settle: float = 0.5,
) -> dict:
    """两侧**同路径**键入 ⇒ 读回 ⇒ 返回可逐字比对的记录。

    参数：
    - `cdp`：CDP 客户端。
    - `navigate`：`navigate(cdp, side, url)` 形态的**导航回调**（各面自己实现，因为路径不同）。
    - `sides`：`(参考侧名, 引擎侧名)`。
    - `selector`：**两侧通用**的控件选择器（要能定位到同一个逻辑控件）。
    - `text`：要键入的文本。

    返回：``{"side": {参考: {...}, 引擎: {...}}, "equal": {...}}``。
    """
    records: dict[str, Any] = {}
    for side in sides:
        navigate(cdp, side)
        cdp.evaluate(
            "(() => { const el = document.querySelector(%s);"
            " if (el) el.focus(); return !!el; })()" % json.dumps(selector)
        )
        type_text(cdp, text)
        time.sleep(settle)
        typed = read_control(cdp, selector)
        typed_echo = echo_text(cdp, selector)
        # 失焦 ⇒ 触发提交（生产的「整组失焦提交」）。
        cdp.evaluate(
            "(() => { const el = document.querySelector(%s);"
            " if (el) el.blur(); return true; })()" % json.dumps(selector)
        )
        time.sleep(settle)
        committed = read_control(cdp, selector)
        committed_echo = echo_text(cdp, selector)
        records[side] = {
            "typed_value": (typed or {}).get("value"),
            "typed_echo": typed_echo,
            "committed_value": (committed or {}).get("value"),
            "committed_echo": committed_echo,
        }
    first, second = sides
    left, right = records[first], records[second]
    equal = {
        "typed_value": left["typed_value"] == right["typed_value"],
        "committed_value": left["committed_value"] == right["committed_value"],
        "typed_echo": left["typed_echo"] == right["typed_echo"],
        "committed_echo": left["committed_echo"] == right["committed_echo"],
    }
    return {
        "case": f"type-into:{selector}",
        "selector": selector,
        "typed_text": text,
        "reference": left,
        "engine": right,
        "equal": equal,
        "passed": all(equal.values()),
    }


def compare_records(case_records: list[dict]) -> dict:
    """把多条交互用例汇总成一个判据（供各面写进 `report["checks"]`）。"""
    passed = all(record.get("passed") for record in case_records) if case_records else False
    return {
        "passed": passed,
        "cases": [
            {
                "case": record.get("case"),
                "passed": record.get("passed"),
                "reference": record.get("reference"),
                "engine": record.get("engine"),
            }
            for record in case_records
        ],
    }
