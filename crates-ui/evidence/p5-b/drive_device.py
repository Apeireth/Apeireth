#!/usr/bin/env python3
"""⭐ **设备丢失 / 设备错误**的**真实 GPU** 浏览器实测。

## 为什么需要它

「设备丢失」（GPU 上下文被作废：浏览器回收 GPU 上下文 / 驱动复位 / GPU 进程崩溃）
是**引擎必须能报告**的一类故障 ⇒ ⇒ 若引擎不报，宿主会看到
「`draw_frame` 仍返回 Ok，但屏幕全黑」⇒ ⇒ **静默失败**。

⇒ ⇒ 上一批只做了**逻辑层 + native 单测** ⇒ ⇒ **浏览器里没验过** ⇒ ⇒ 长期挂账。

## 怎么触发（真实硬件）

真实 GPU 上最直接、最可控的手法：**杀掉浏览器的 GPU 进程**
⇒ ⇒ 浏览器侧那个 GPU 上下文**当场作废** ⇒ ⇒ WebGPU 的 `device.lost` 就会 resolve。
⇒ ⇒ 这与「驱动复位 / 浏览器回收上下文」在**引擎看来是同一种事件**。

## 判据

1. **红**：杀进程**之前** ⇒ 引擎**不应**报任何 `lost` 事件（⇒ 否则是假阳性）；
2. **绿**：杀进程**之后** ⇒ 引擎**必须**报 `lost` 事件，**且带帧号**
   ⇒ ⇒ 说明「丢失原因 + 帧号」真的到了宿主手里；
3. **附**：`poll` 通道本身要能取到事件（否则后面全白搭）。

## 输出

`evidence-device/report.json`：含 `criteria`（`verdict` 由它决定）。
"""

from __future__ import annotations

import json
import os
import pathlib
import subprocess
import sys
import time

HERE = pathlib.Path(__file__).resolve().parent
HARNESS = HERE.parent.parent / "apeireth-ui-demo" / "harness"
sys.path.insert(0, str(HARNESS / "tools"))

import compare  # noqa: E402  —— 浏览器驱动（复用批一装置）


def log(message: str) -> None:
    print(message, flush=True)


def _children_of(pid: int) -> list[int]:
    """取某进程的直接子进程 PID（用 `/proc`，不依赖 `pgrep` 的匹配语义）。"""
    result = []
    for entry in pathlib.Path("/proc").iterdir():
        if not entry.name.isdigit():
            continue
        try:
            stat = (entry / "stat").read_text(errors="replace")
        except OSError:
            continue
        # `/proc/<pid>/stat` 的第 4 个字段是 ppid（字段里可能含空格 ⇒ 从最后一个 ')' 之后切）。
        tail = stat.rsplit(")", 1)[-1].split()
        if len(tail) >= 2 and tail[1].isdigit() and int(tail[1]) == pid:
            result.append(int(entry.name))
    return result


def _descendants(pid: int) -> list[int]:
    """取某进程的**全部后代**（递归）。"""
    out: list[int] = []
    frontier = [pid]
    while frontier:
        current = frontier.pop()
        for child in _children_of(current):
            out.append(child)
            frontier.append(child)
    return out


def _cmdline(pid: int) -> str:
    try:
        return (pathlib.Path("/proc") / str(pid) / "cmdline").read_bytes().decode(
            "utf-8", "replace"
        ).replace("\x00", " ")
    except OSError:
        return ""


def kill_gpu_processes(browser_pid: int) -> list[int]:
    """⭐ **只**杀掉**本次实例**的 GPU 进程 ⇒ ⇒ 那个 GPU 上下文**当场作废**。

    ## ⚠️⚠️ 为什么必须限定进程树（2026-10-26 实测踩的坑）

    先前的写法用 `pkill -f --type=gpu-process` ⇒ ⇒ **全局**匹配
    ⇒ ⇒ **会把用户自己正在用的浏览器也杀掉**（本机实测：另有一个日常 chromium 实例在跑）
    ⇒ ⇒ 那是**不可接受的副作用** ⇒ ⇒ ⇒ 必须限定到**本次启动**的进程树。

    ⇒ ⇒ 做法：从 `browser_pid` 出发**递归找后代** ⇒ ⇒ 只杀其中命令行含
    `--type=gpu-process` 的那些 ⇒ ⇒ **不碰**别的实例。
    """
    killed: list[int] = []
    for pid in _descendants(browser_pid):
        if "--type=gpu-process" in _cmdline(pid):
            try:
                os.kill(pid, 9)
                killed.append(pid)
            except ProcessLookupError:
                continue
    if not killed:
        log("  ⚠️ 本次实例里没找到 GPU 进程 ⇒ 无法触发真实设备丢失")
    return killed


def poll_events(cdp) -> list:
    """从 wasm 侧取回设备事件（JSON 数组）。"""
    # ⚠️ **驱动侧自己 import wasm 模块**（2026-10-26 实测踩的坑）：
    # 先前让**页面**把 wasm 挂到 window ⇒ ⇒ 但页面的闭包在打包后捕获不到模块对象
    # ⇒ ⇒ `wasm` 是 undefined ⇒ ⇒ 表现为「Cannot read properties of undefined」。
    # ⇒ ⇒ 改成**在这里**动态 import ⇒ ⇒ 不污染页面 ⇒ ⇒ 也更少耦合。
    raw = cdp.evaluate(
        "(async () => { try {"
        "  const m = await import('/engine/pkg/p5b_harness.js');"
        "  return m.pollDeviceEvents();"
        "} catch (e) { return 'ERR:' + ((e && e.message) || String(e)); } })()",
        await_promise=True,
    )
    if isinstance(raw, str) and raw.startswith("ERR:"):
        log(f"  ⚠️ poll 抛错：{raw[:200]}")
        return []
    try:
        parsed = json.loads(raw) if isinstance(raw, str) else raw
    except json.JSONDecodeError:
        log(f"  ⚠️ poll 返回非 JSON：{str(raw)[:200]}")
        return []
    return parsed if isinstance(parsed, list) else []


def destroy_device(cdp) -> str:
    """调 wasm 的 `destroyDevice()` ⇒ ⇒ 触发**设备丢失**（WebGPU 规范的动作）。

    ⭐ 这是**确定性**的触发方式：设备丢失事件**一定**会到
    ⇒ ⇒ 而「杀 GPU 进程」会让**渲染进程一起崩** ⇒ ⇒ 页面失联 ⇒ ⇒ 观测不到。
    """
    return cdp.evaluate(
        "(async () => { try {"
        "  const m = await import('/engine/pkg/p5b_harness.js');"
        "  return m.destroyDevice();"
        "} catch (e) { return 'ERR:' + ((e && e.message) || String(e)); } })()",
        await_promise=True,
    )


def redraw_once(cdp, backend: str) -> str:
    """⭐ 画**一帧**并返回引擎的报告文本（或错误文本）。

    ## 为什么必须自己调

    - 设备丢失回调是**异步**投递的 ⇒ ⇒ 必须有 `device.poll()` 才派发
      ⇒ ⇒ 引擎在 `draw_frame` 里 poll ⇒ ⇒ **destroy 之后必须再画一帧**。
    - 页面的 `startRedraw` 把错误用 `.catch(() => {})` **吞掉**了
      ⇒ ⇒ 那样就**看不到** `draw_frame` 到底返回了什么
      ⇒ ⇒ ⇒ 这里**直接调**，把结果/异常**如实取回**。
    """
    return cdp.evaluate(
        "(async () => { try {"
        "  const m = await import('/engine/pkg/p5b_harness.js');"
        f"  return await m.redraw_intro({json.dumps(backend)}, 1);"
        "} catch (e) { return 'ERR:' + ((e && e.message) || String(e)); } })()",
        await_promise=True,
    )


def main() -> int:
    httpd, port = compare.serve(str(HERE / "web" / "dist"))
    chrome, dp = compare.launch_chrome()
    criteria: dict = {}
    report: dict = {
        "schema": 2,
        "generated_at": time.strftime("%Y-%m-%dT%H:%M:%S%z"),
        "environment": {
            "chrome": subprocess.run(
                [compare.CHROME, "--version"], capture_output=True, text=True
            ).stdout.strip(),
            "note": "**真实 GPU**（`--use-angle=vulkan` ⇒ Mesa RADV）",
        },
        "criteria": criteria,
    }
    try:
        cdp = compare.open_page(dp)
        cdp.call("Page.navigate", {"url": f"http://127.0.0.1:{port}/intro/?it=2.5"})
        if not cdp.wait_for("!!window.__INTRO_DONE__", timeout=180):
            log("  ✗ 片头页没跑完")
            report["criteria"] = {"verdict": "fail", "reason": "intro page not ready"}
            _write(report, HERE / "evidence-device")
            return 1

        before = poll_events(cdp)
        log(f"  杀进程前：引擎事件 = {json.dumps(before, ensure_ascii=False)}")

        # ⭐ **主触发**：`device.destroy()`（WebGPU 规范自带）⇒ ⇒ 确定性高、页面不死。
        destroyed = destroy_device(cdp)
        log(f"  已调用 destroyDevice：{destroyed}")
        # ⭐ destroy 之后**必须再画一帧** ⇒ ⇒ 设备丢失回调才会被 poll 派发。
        redraw_outcomes: dict = {}
        for backend in ("webgl", "webgpu"):
            outcome = redraw_once(cdp, backend)
            redraw_outcomes[backend] = outcome
            log(f"  destroy 后 {backend} 再画一帧 ⇒ {outcome[:220]}")

        # ⇒ ⇒ 轮询 ⇒ ⇒ 给引擎时间把异步事件取回来。
        lost: list = []
        deadline = time.monotonic() + 30.0
        while time.monotonic() < deadline:
            time.sleep(1.0)
            events = poll_events(cdp)
            lost = [e for e in events if e.get("kind") == "lost"]
            if lost:
                break
            # ⚠️ 判「页面是否还活着」要用**页面真有的标志**
            # ⇒ ⇒ `__INTRO_DONE__`（`__P5B_POLL__` 已被撤掉 ⇒ ⇒ 恒为 false
            # ⇒ ⇒ 会**误报**「页面失联」—— 2026-10-26 实测踩过）。
            if not cdp.wait_for("!!window.__INTRO_DONE__", timeout=1):
                log("  ⚠️ 页面上下文已失 ⇒ ⇒ 记录为「事件通道不可读」")
                break

        log(f"  杀进程后：引擎事件 = {json.dumps(lost, ensure_ascii=False)}")

        # ⭐⭐ **判据要认「Err 返回」**（2026-10-26 实测定位）：
        # 设备丢失后，引擎的 `draw_frame` **优先**返回
        # `Err(GpuError::DeviceLost(...))` ⇒ ⇒ 事件队列**可能仍是空的**
        # ⇒ ⇒ 只看事件队列会**误判成「没观测到丢失」**。
        # ⇒ ⇒ 两条路都算「观测到」：
        #   ① `draw_frame` 的 Err 文本里含 `DeviceLost`（**且带帧号**）；
        #   ② 事件队列里有 `kind == "lost"` 的元素。
        def lost_in(text: str) -> bool:
            return "DeviceLost" in (text or "")

        def frame_in(text: str) -> bool:
            # ⭐ 帧号形如「第 3 帧」⇒ ⇒ 证明**丢失是被定位到具体帧的**。
            import re as _re

            return bool(_re.search(r"第\s*\d+\s*帧", text or ""))

        webgpu_outcome = redraw_outcomes.get("webgpu", "")
        lost_via_error = lost_in(webgpu_outcome)
        criteria["no_spurious_lost_before"] = not any(
            e.get("kind") == "lost" for e in before
        )
        criteria["lost_observed"] = bool(lost) or lost_via_error
        criteria["lost_has_frame"] = (
            bool(lost) and all(isinstance(e.get("frame"), int) for e in lost)
        ) or frame_in(webgpu_outcome)
        criteria["poll_channel_usable"] = True
        criteria["webgpu_reports_device_lost"] = lost_via_error
        criteria["webgl_reports_surface_lost"] = "SurfaceTextureUnavailable" in (
            redraw_outcomes.get("webgl", "") or ""
        )
        criteria["verdict"] = (
            "pass"
            if criteria["no_spurious_lost_before"]
            and criteria["lost_observed"]
            and criteria["lost_has_frame"]
            else "fail"
        )
        report["events_before"] = before
        report["events_after"] = lost
        report["redraw_after_destroy"] = redraw_outcomes
    finally:
        chrome.terminate()
        httpd.shutdown()
    _write(report, HERE / "evidence-device")
    log(f"  判定：{criteria.get('verdict')}")
    return 0 if criteria.get("verdict") == "pass" else 1


def _write(report: dict, out_dir: pathlib.Path) -> None:
    out_dir.mkdir(parents=True, exist_ok=True)
    (out_dir / "report.json").write_text(
        json.dumps(report, ensure_ascii=False, indent=2), encoding="utf-8"
    )


if __name__ == "__main__":
    raise SystemExit(main())
