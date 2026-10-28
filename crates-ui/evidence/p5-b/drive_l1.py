#!/usr/bin/env python3
"""⭐ **L1 能力浏览器探针驱动**（台账 E-98）。

跑法（在 `crates-ui/evidence/p5-b/` 下）：

```bash
python3 drive_l1.py            # 跑探针，结果写 evidence/l1/report.json
python3 drive_l1.py --out DIR  # 指定输出目录
```

退出码：0 全过；1 有失败项；2 探针未产出结果（装置问题）。

判据（与 `harness/src/lib.rs::l1_probe` 对应）：
- **G1** 引擎 `bounding_box` 与浏览器 `getBoundingClientRect` 逐值一致（≤0.5px）；
- **G2** 真实 `input` 事件载荷可读（`target_value` / `checked` / 坐标字段存在）；
- **G3** `focus_at_path` 真的驱动 `document.activeElement`；`compositionstart/end`
  经 **document 级**监听翻转 `Dom::is_composing()`；
- **G6** `bounding_box_in_device` 按 `dpr × density` 换算（2.0 × 1.25 = 2.5）；
- **G8** 错误边界：强制渲染失败 ⇒ **兜底视图真的进了 DOM** 且 `captured_error()` 是**原始错误**；
  panic hook 可安装（⚠️ 不在浏览器里触发真 panic：wasm 可能 `panic=abort`，会 trap 掉实例）。

⚠️ 本脚本**只用标准库 + harness 既有 `compare` 工具**（`websocket-client` / `pillow` 之外无新依赖）。
"""
from __future__ import annotations

import argparse
import json
import pathlib
import sys
import time

HERE = pathlib.Path(__file__).resolve().parent
sys.path.insert(
    0,
    str(HERE.parent.parent / "apeireth-ui-demo" / "harness" / "tools"),
)
sys.path.insert(0, str(HERE))

import compare  # noqa: E402

DIST = HERE / "web" / "dist"


def main() -> int:
    parser = argparse.ArgumentParser(description="L1 能力浏览器探针")
    parser.add_argument("--out", default=str(HERE / "evidence" / "l1"))
    parser.add_argument("--timeout", type=float, default=25.0)
    args = parser.parse_args()

    if not (DIST / "l1" / "index.html").exists():
        print(f"❌ 缺少构建产物 {DIST}/l1/index.html（先跑 build.sh 或 vite build）", file=sys.stderr)
        return 2

    httpd, port = compare.serve(DIST)
    chrome, devtools = compare.launch_chrome()
    try:
        cdp = compare.open_page(devtools)
        cdp.call("Page.navigate", {"url": f"http://127.0.0.1:{port}/l1/index.html"})
        deadline = time.time() + args.timeout
        while time.time() < deadline and not cdp.evaluate("!!window.__L1_DONE__"):
            time.sleep(0.2)
        raw = cdp.evaluate(
            "JSON.stringify(window.__L1__ || {ok:false,failures:['探针未产出结果'],probe_missing:true})"
        )
        report = json.loads(raw)
    finally:
        try:
            chrome.kill()
        except Exception:
            pass
        httpd.shutdown()

    out_dir = pathlib.Path(args.out)
    out_dir.mkdir(parents=True, exist_ok=True)
    (out_dir / "report.json").write_text(
        json.dumps(report, ensure_ascii=False, indent=2), encoding="utf-8"
    )
    print(json.dumps(report, ensure_ascii=False, indent=2))
    if report.get("probe_missing"):
        return 2
    if not report.get("ok"):
        print(f"❌ 失败项：{report.get('failures')}", file=sys.stderr)
        return 1
    print("✅ L1 浏览器探针全过（G1 测量 / G2 载荷 / G3 焦点与 IME / G6 缩放 / G8 错误边界与 panic hook）")
    return 0


if __name__ == "__main__":
    sys.exit(main())
