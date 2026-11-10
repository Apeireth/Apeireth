"""**控制台编码守卫**：让门禁脚本在**非 UTF-8 控制台**下也能安全打印。

背景（真事故）：Windows 控制台默认编码是 ANSI 代码页
（GitHub `windows-latest` = `cp1252`，中文 Windows = `cp936`）⇒ ⇒ `print` 打印
`⇒` / `✅` / `❌` 这类字符时抛 `UnicodeEncodeError` ⇒ ⇒ 脚本 exit 1
⇒ ⇒ **把本来通过的 CI job 判成红**（上游 run 42769798792：
`cargo nextest (windows-latest)` 的 `回收测试二进制（腾磁盘）` 步炸掉，
而**测试本身是过的**）。

用法（**必须在本脚本任何打印之前**调用）：

    import sys
    from pathlib import Path

    sys.path.insert(0, str(Path(__file__).resolve().parent))
    from utf8_stdout import force_utf8_stdout  # noqa: E402

    force_utf8_stdout()

设计约束（**每一条都是踩过的坑**）：
- **绝不抛异常**：stdout/stderr 可能被 pytest 捕获、可能是管道、可能根本不存在
  ⇒ ⇒ 任何一步失败都静默跳过（守卫不该自己变成新的失败源）。
- `errors="replace"`：宁可输出替代字符，也**绝不**因为一个字符崩掉整个门禁。
- 只改**编码**：不改脚本的打印内容、不改退出码、不改逻辑。
"""
from __future__ import annotations

import sys

__all__ = ["force_utf8_stdout"]


def force_utf8_stdout() -> None:
    """把 stdout/stderr 重配成 UTF-8（**永不抛异常**）。"""
    for stream in (sys.stdout, sys.stderr):
        reconfigure = getattr(stream, "reconfigure", None)
        if reconfigure is None:
            continue
        try:
            reconfigure(encoding="utf-8", errors="replace")
        except Exception:  # pragma: no cover - 非 TextIOWrapper / 已读过的流 / 老解释器
            pass
