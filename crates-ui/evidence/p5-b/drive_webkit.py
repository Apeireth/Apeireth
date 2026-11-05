#!/usr/bin/env python3
"""⭐⭐⭐ **WebKitGTK 侧引擎取证驱动**（E-175）。

## 它做什么

1. 编译 **Tauri 2** 壳（Linux 上 = **WebKitGTK** ⇒ ⇒ **不是** Chromium）；
2. 把引擎 wasm（`wasm-bindgen` 产物）复制进壳的 `web/engine/pkg/`；
3. 用 **Xvfb** 起**真实窗口** ⇒ ⇒ 页面加载引擎 wasm 并跑引擎侧探针
   （键盘语义 E-169 / 焦点与模态 E-174 / DOM 基础）；
4. 读回结果 ⇒ ⇒ **判红绿**（退出码）。

## 为什么必须有它

引擎此前的**全部**浏览器证据都来自 **Chromium** ⇒ ⇒ 「真实 WebKitGTK」
**0 次验证** ⇒ ⇒ 而「焦点陷阱」「IME」「Tab 走引擎」这些**依赖内核行为**的能力
⇒ ⇒ ⇒ **恰恰是最可能跨内核不一致**的 ⇒ ⇒ ⇒ 必须换内核验一遍。

## 复跑

```bash
cd crates-ui/evidence/p5-b
python3 drive_webkit.py
```

## 依赖（**本机已就绪**，实测 2026-11-04）

- `webkit2gtk-4.1`（实测 **2.52.6**）+ `gtk3` + `libsoup-3.0` + `jscore-4.1`；
- `Xvfb`（无 GPU 环境下用虚拟显示）；
- `cargo` 能取到 `tauri 2`（crates.io 稀疏索引可达）。

## ⚠️ 0 装口径

- 本驱动**只**证明「引擎在 **Linux WebKitGTK** 里成立」；
- **Windows WebView2** 仍然**未验**（本机没有 Windows）⇒ ⇒ 不得外推；
- 结果里的 `errors` 字段若非空 ⇒ ⇒ **不判红**，但**必须原样记录**（可能是
  wasm-bindgen glue 的闭包生命周期行为 ⇒ ⇒ 需**单独归因**，不得混入判据）。
"""

from __future__ import annotations

import argparse
import json
import os
import pathlib
import shutil
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parent
SHELL = ROOT / "webkit" / "shell"
# ⭐ harness（引擎浏览器宿主）就在本目录下 ⇒ ⇒ wasm 产物从它出。
HARNESS_DIR = ROOT / "harness"
TARGET_DIR = pathlib.Path(os.environ.get("APEIRETH_WEBKIT_TARGET", "/home/bai/Projects/webkit-probe-target"))
RESULT_FILE = pathlib.Path("/tmp/webkit-probe-result.json")


def fail(message: str) -> int:
    print(f"❌ {message}", file=sys.stderr)
    return 1


def loadavg_hint() -> str:
    """机器负载（1/5/15 分钟 load average）—— 用来判断「是不是机器被压住了」。

    ⭐ 为什么要有它：2026-11-05 实测踩过 —— 别的线在并行编译（load average 528）
    ⇒ ⇒ WebKit 的 wasm JIT 编不完 ⇒ ⇒ 页面停在「loading-wasm」⇒ ⇒⇒ 看着像
    「引擎坏了」⇒ ⇒⇒ 实则是**时间预算不够** ⇒ ⇒⇒ 失败时**先把负载摆出来**，
    免得下一个人又去怀疑代码。
    """
    try:
        return pathlib.Path("/proc/loadavg").read_text(encoding="utf-8").strip()
    except OSError:
        return "（读不到 /proc/loadavg）"


def main() -> int:
    # ⭐ 证据落盘目录（E-181）：与 L1 / W0 驱动**同构** ⇒ ⇒ 三份 report.json
    #   都在 `evidence/<name>/report.json` ⇒ ⇒ 主线一处收齐 ⇒ ⇒ 不用再去
    #   `/tmp` 里翻。
    parser = argparse.ArgumentParser(description="WebKitGTK 侧引擎取证探针")
    parser.add_argument(
        "--out",
        default=str(pathlib.Path(__file__).resolve().parent / "evidence" / "webkit"),
    )
    args = parser.parse_args()

    if shutil.which("xvfb-run") is None:
        return fail("缺 xvfb-run（无头环境下起真实窗口需要它）")
    if shutil.which("pkg-config") is None:
        return fail("缺 pkg-config")

    # ① WebKitGTK 系统库在位吗
    probe = subprocess.run(
        ["pkg-config", "--exists", "webkit2gtk-4.1"], capture_output=True
    )
    if probe.returncode != 0:
        return fail("缺 webkit2gtk-4.1（Tauri 2 在 Linux 上的 WebView 后端）")

    # ② 引擎 wasm：先构建 harness（wasm32）⇒ ⇒ 再 wasm-bindgen ⇒ ⇒ 复制进壳
    wasm_pkg = SHELL / "web" / "engine" / "pkg"
    wasm_pkg.mkdir(parents=True, exist_ok=True)
    print("→ 构建引擎 harness（wasm32）…", flush=True)
    build = subprocess.run(
        [
            "cargo", "build", "--release", "--target", "wasm32-unknown-unknown",
            "--offline",
        ],
        cwd=HARNESS_DIR,
        capture_output=True,
        text=True,
    )
    if build.returncode != 0:
        RESULT_FILE.unlink(missing_ok=True)   # ⭐ 别让旧结果冒充本轮
        print(build.stderr[-2000:], file=sys.stderr)
        return fail("引擎 harness 的 wasm32 构建失败")

    print("→ wasm-bindgen 打包…", flush=True)
    wasm_file = (
        pathlib.Path(os.environ.get("CARGO_TARGET_DIR", "/home/bai/Projects/apeireth-ui-target"))
        / "wasm32-unknown-unknown"
        / "release"
        / "p5b_harness.wasm"
    )
    if not wasm_file.exists():
        return fail(f"找不到 wasm 产物：{wasm_file}")
    bindgen = subprocess.run(
        [
            "wasm-bindgen", "--target", "web", "--out-dir", str(wasm_pkg),
            "--out-name", "p5b_harness", str(wasm_file),
        ],
        capture_output=True,
        text=True,
    )
    if bindgen.returncode != 0:
        print(bindgen.stderr[-2000:], file=sys.stderr)
        return fail("wasm-bindgen 失败")
    # ⭐⭐⭐ **产物守卫**（2026-11-05 实测踩过）：wasm-bindgen **退出码 0 ≠ 产物齐全**
    #   ⇒ ⇒⇒ 曾经因为输出被吞 ⇒ ⇒ `pkg/` 是空的 ⇒ ⇒⇒ 页面 import 失败 ⇒ ⇒⇒⇒
    #   ⇒ ⇒⇒⇒ 探针静默不跑 ⇒ ⇒⇒⇒⇒ 报「未在时限内回传结果」⇒ ⇒⇒⇒⇒ 误判方向完全跑偏。
    glue = wasm_pkg / "p5b_harness.js"
    if not glue.exists():
        return fail(f"wasm-bindgen 退出码 0 但缺产物：{glue}")

    # ③ 编译 Tauri 壳
    print("→ 编译 Tauri 壳（WebKitGTK）…", flush=True)
    # ⭐⭐⭐ **强制重建**（2026-11-05 实测踩过）：Tauri 的 `frontendDist` 是
    #   ⇒ ⇒ **构建期嵌入** ⇒ ⇒ 改了 `web/` 下的页面但没触发重建 ⇒ ⇒ 壳里跑的还是旧页面
    #   ⇒ ⇒⇒ ⇒ 于是出现「我明明改了页面却没生效」的假象 ⇒ ⇒⇒⇒⇒ 一度误判为「修复无效」。
    #   ⇒ ⇒⇒⇒ ⇒ 所以每次跑之前 touch 一次 build.rs ⇒ ⇒ 强制 build 脚本重跑 ⇒ ⇒ 重新嵌入。
    (SHELL / "build.rs").touch()
    env = dict(os.environ)
    env["CARGO_TARGET_DIR"] = str(TARGET_DIR)
    shell_build = subprocess.run(
        ["cargo", "build"], cwd=SHELL, capture_output=True, text=True, env=env
    )
    if shell_build.returncode != 0:
        print(shell_build.stderr[-3000:], file=sys.stderr)
        return fail("Tauri 壳构建失败")

    # ④ 用 Xvfb 起真实窗口跑探针
    print("→ 起真实 WebKitGTK 窗口（Xvfb）…", flush=True)
    # ⭐⭐⭐ **必须先删旧结果文件** ⇒ ⇒ 否则「本轮失败」会被**上一轮的旧结果**掩盖
    #   ⇒ ⇒⇒ （2026-11-05 实测踩过：引擎 wasm 构建失败 ⇒ ⇒ 驱动提前退出
    #   ⇒ ⇒⇒ 但**旧结果文件还在** ⇒ ⇒⇒ 我据此得出「修法无效」的**错误结论**）。
    RESULT_FILE.unlink(missing_ok=True)
    binary = TARGET_DIR / "debug" / "webkit-probe"
    if not binary.exists():
        return fail(f"找不到壳二进制：{binary}")
    run = subprocess.run(
        [
            "xvfb-run", "-a", "--server-args=-screen 0 1280x820x24",
            str(binary), str(RESULT_FILE),
        ],
        capture_output=True,
        text=True,
        # ⭐⭐⭐ **超时与壳的轮询预算对齐**（2026-11-05 实测踩过）：机器上**同时有别的
        #   ⇒ ⇒ 线在并行编译**时（实测 `load average` 到 **528**）⇒ ⇒ WebKit 的 wasm
        #   ⇒ ⇒ JIT 编 5.9 MB 模块**编不完 180 秒** ⇒ ⇒ 页面停在「loading-wasm」
        #   ⇒ ⇒⇒ 看着像「引擎坏了」⇒ ⇒⇒ 实则是**时间预算不够**（老规矩：先怀疑自己的测量）
        #   ⇒ ⇒⇒⇒ 故壳内预算 900 秒、这里 1200 秒。
        timeout=1200,
    )
    if not RESULT_FILE.exists():
        print("── 宿主 stdout ──", file=sys.stderr)
        print(run.stdout[-3000:], file=sys.stderr)
        print("── 宿主 stderr ──", file=sys.stderr)
        print(run.stderr[-3000:], file=sys.stderr)
        print(f"── 机器负载 ──\n{loadavg_hint()}", file=sys.stderr)
        return fail("探针没产出结果文件（WebView 里可能没跑起来）")

    report = json.loads(RESULT_FILE.read_text(encoding="utf-8"))
    print(json.dumps(report, ensure_ascii=False, indent=2))

    # ⑤ 判据
    failures: list[str] = []
    user_agent = report.get("userAgent") or ""
    if "AppleWebKit" not in user_agent or "Chrom" in user_agent:
        failures.append(f"不是 WebKitGTK（UA={user_agent!r}）⇒ ⇒ 这次**不算** WebKitGTK 证据")
    for key, label in (
        ("keyboardSemantics", "键盘语义（E-169）"),
        ("focusModal", "焦点与模态（E-174）"),
        ("dom", "DOM 基础"),
    ):
        section = report.get(key) or {}
        if not section.get("ok"):
            failures.append(f"{label} 未过：{section.get('failures')}")
    if report.get("errors"):
        # ⭐⭐⭐ **判红**（E-176 起）：这些 error **不是**无害噪声 ⇒ ⇒
        #   ⇒ ⇒ 「closure invoked recursively or after being dropped」= **死闭包被调用**
        #   ⇒ ⇒ ⇒ 意味着有「wasm 侧已死、JS 侧还挂着」的监听器 ⇒ ⇒ ⇒ **真缺陷**
        #   ⇒ ⇒ ⇒ 曾经「不判红」是因为当时**未归因** ⇒ ⇒⇒ 归因后（E-176 修完）
        #   ⇒ ⇒ ⇒ ⇒ 改判红 ⇒ ⇒ ⇒ 这样本驱动就是这条缺陷的**回归门**。
        failures.append(f"页面报了 {len(report['errors'])} 个 error：{report['errors'][:3]}")
    # ⭐ 证据落盘（E-181）：与 L1 / W0 驱动**同构** ⇒ ⇒ 三份 report.json
    #   都在 `evidence/<name>/report.json` ⇒ ⇒ 主线一处收齐 ⇒ ⇒ 不用再去
    #   `/tmp` 里翻 ⇒ ⇒ 而且**失败也落盘**（失败证据同样要留）。
    out_dir = pathlib.Path(args.out)
    out_dir.mkdir(parents=True, exist_ok=True)
    (out_dir / "report.json").write_text(
        json.dumps(
            {**report, "ok": not failures, "failures": failures},
            ensure_ascii=False,
            indent=2,
        ),
        encoding="utf-8",
    )

    if failures:
        for item in failures:
            print(f"❌ {item}", file=sys.stderr)
        # ⭐⭐ 把**机器负载**摆出来（先怀疑自己的测量 ⇒ ⇒ 别急着怀疑代码）。
        print(f"── 机器负载 ──\n{loadavg_hint()}", file=sys.stderr)
        print(f"── 页面阶段 ──\n{report.get('stage')!r}", file=sys.stderr)
        # ⭐⭐⭐ **宿主 stdout/stderr 总是打印**（不只在「结果文件缺失」时）：
        #   壳每 400ms 打一行 `[probe] url=…` ⇒ ⇒ 那就是**页面 hash 的时间线**
        #   ⇒ ⇒⇒ 失败时它是**唯一**能区分「页面没跑」与「页面卡在哪一步」的轨迹。
        print("── 宿主 stdout ──", file=sys.stderr)
        print(run.stdout[-4000:], file=sys.stderr)
        print("── 宿主 stderr（末段） ──", file=sys.stderr)
        print("\n".join(run.stderr.splitlines()[-40:]), file=sys.stderr)
        return 1
    print("✅ WebKitGTK 侧引擎取证全过（键盘语义 / 焦点与模态 / DOM 基础）")
    return 0


if __name__ == "__main__":
    sys.exit(main())
