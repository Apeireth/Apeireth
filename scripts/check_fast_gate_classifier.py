#!/usr/bin/env python3
"""fast gate（`scripts/verify_changed.py`）的**分类口径**回归自检。

背景（2026-11-17，缺陷账本 `docs/04-internal/ui-engine-defect-ledger.md` E-122）：
`verify_changed.py` 曾把**任何** `.toml`（含 `docs/architecture/contract.toml`）当成 Rust 变更
⇒ 映射到根 manifest ⇒ 影响面按**全 workspace**计 ⇒ 把 cargo metadata 里的
**三方依赖名**也一起塞进 `cargo -p` ⇒ 撞上多版本依赖即报
`error: specification 'bit-set' is ambiguous` ⇒ **门禁假红**（与被测改动无关）。

⇒ ⇒ 后果：**任何碰了契约/配置 `.toml` 的批次都跑不过强制的 fast gate**
⇒ ⇒ ⇒ 纪律链在真正需要它的地方断掉（守卫失效比红更坏）。

本脚本把「怎么分类」钉成**可判真**的断言：
- 契约类 `.toml` / 文档 / 脚本 ⇒ **不是** Rust 变更（docs-only 路径）；
- `.rs` ⇒ 归属最近的祖先 crate；
- `Cargo.lock` / 根 `Cargo.toml` / `rust-toolchain.toml` ⇒ 全 workspace；
- 影响面**只含根 workspace 成员**（不含三方依赖名）；
- 反向依赖闭包**非空**（改 core ⇒ 依赖它的 crate 也在影响面里）。

用法：`python3 scripts/check_fast_gate_classifier.py`（只读；不跑 cargo 编译）
退出码：0 = 全部断言成立；1 = 有断言不成立（口径漂移）。
"""

from __future__ import annotations

import importlib.util
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
TARGET = ROOT / "scripts" / "verify_changed.py"

# Windows 控制台默认 GBK ⇒ 中文/箭头字符会抛 UnicodeEncodeError ⇒ 强制 UTF-8
# （与仓内 clean_test_binaries.py 同一处置）。
for _stream in (sys.stdout, sys.stderr):
    try:
        _stream.reconfigure(encoding="utf-8", errors="replace")
    except Exception:  # noqa: BLE001 —— 老解释器没有 reconfigure 时不阻断
        pass


def _load_verify_changed():
    spec = importlib.util.spec_from_file_location("_verify_changed_under_test", TARGET)
    if spec is None or spec.loader is None:
        raise SystemExit(f"[fast-gate-classifier] 无法加载 {TARGET}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def main() -> int:
    vc = _load_verify_changed()
    failures: list[str] = []

    def check(name: str, ok: bool, detail: str = "") -> None:
        if ok:
            print(f"  ok   {name}")
        else:
            print(f"  FAIL {name}{(' — ' + detail) if detail else ''}")
            failures.append(name)

    print("[fast-gate-classifier] 载入 cargo metadata …")
    meta = vc.load_metadata()
    by_manifest, members = vc.build_index(meta)
    member_names = {pkg["name"] for pkg in meta["packages"] if pkg["id"] in members}
    all_names = {pkg["name"] for pkg in meta["packages"]}

    print("[fast-gate-classifier] 分类口径：")
    # ① 契约类 .toml ⇒ 不是 Rust 变更（这条就是 E-122 的回归点）
    classified = vc.classify_files(["docs/architecture/contract.toml"], by_manifest)
    check(
        "契约类 .toml 不算 Rust 变更（docs-only）",
        not classified["code_files"] and not classified["manifest_files"],
        f"code={classified['code_files']} manifest={classified['manifest_files']}",
    )
    # ② 文档 / 脚本同样不算
    classified = vc.classify_files(["AGENTS.md", "scripts/cargo.ps1"], by_manifest)
    check(
        "文档与脚本不算 Rust 变更",
        not classified["code_files"] and not classified["manifest_files"],
        f"code={classified['code_files']} manifest={classified['manifest_files']}",
    )
    # ③ .rs 归属最近的祖先 crate
    classified = vc.classify_files(["crates/foundation/core/src/kernel/ids.rs"], by_manifest)
    seed_names = sorted({by_manifest_name(meta, pid) for pid in classified["seed_ids"]})
    check(
        ".rs 归属最近的祖先 crate（apeireth-core）",
        seed_names == ["apeireth-core"],
        f"seed={seed_names}",
    )
    # ④ 根 Cargo.lock ⇒ 全 workspace
    classified = vc.classify_files(["Cargo.lock"], by_manifest)
    check("Cargo.lock ⇒ 全 workspace", classified["all_workspace"] is True)
    # ⑤ 根 Cargo.toml ⇒ 全 workspace
    classified = vc.classify_files(["Cargo.toml"], by_manifest)
    check("根 Cargo.toml ⇒ 全 workspace", classified["all_workspace"] is True)
    # ⑥ crate 级 Cargo.toml ⇒ 只影响该 crate
    classified = vc.classify_files(["crates/foundation/core/Cargo.toml"], by_manifest)
    seed_names = sorted({by_manifest_name(meta, pid) for pid in classified["seed_ids"]})
    check(
        "crate 级 Cargo.toml ⇒ 只归属该 crate",
        classified["all_workspace"] is False and seed_names == ["apeireth-core"],
        f"all_workspace={classified['all_workspace']} seed={seed_names}",
    )
    # ⑦ 独立 workspace 的 .rs ⇒ 明确报「不覆盖」，不静默吞掉
    classified = vc.classify_files(["some-independent-workspace/src/lib.rs"], by_manifest)
    check(
        "独立 workspace 的 .rs 进 foreign 清单（明说不覆盖）",
        classified["foreign_files"] == ["some-independent-workspace/src/lib.rs"],
        f"foreign={classified['foreign_files']}",
    )

    print("[fast-gate-classifier] 影响面口径：")
    # ⑧ 全 workspace 影响面里不得出现三方依赖名（E-122 的根因）
    affected = vc.affected_packages([], meta, True)
    leaked = sorted(set(affected) - member_names)
    check(
        "全 workspace 影响面只含根 workspace 成员（不含三方依赖）",
        not leaked,
        f"泄漏的三方依赖名: {leaked[:8]}",
    )
    # ⑨ 反向依赖闭包非空：改 core ⇒ 依赖它的 crate 也在影响面里
    core_ids = [pkg["id"] for pkg in meta["packages"] if pkg["name"] == "apeireth-core"]
    if not core_ids:
        check("找得到 apeireth-core 包", False, "cargo metadata 里没有该包")
    else:
        affected = vc.affected_packages(core_ids, meta, False)
        check(
            "反向依赖闭包非空（不止改动 crate 自己）",
            len(affected) > 1 and "apeireth-core" in affected,
            f"affected={affected[:6]}…共 {len(affected)}",
        )
        check(
            "反向依赖闭包同样只含根 workspace 成员",
            not (set(affected) - member_names),
            f"泄漏: {sorted(set(affected) - member_names)[:8]}",
        )

    print("[fast-gate-classifier] 参考：")
    print(f"  cargo metadata 包总数 = {len(all_names)}；根 workspace 成员 = {len(member_names)}")
    print(f"  成员名单 = {json.dumps(sorted(member_names), ensure_ascii=False)}")

    if failures:
        print(f"[fast-gate-classifier] FAIL: {len(failures)} 条断言不成立 → {failures}")
        print("  修法：口径见 scripts/verify_changed.py classify_files() 的注释；"
              "回归点见 docs/04-internal/ui-engine-defect-ledger.md E-122")
        return 1
    print("[fast-gate-classifier] OK：分类与影响面口径全部成立")
    return 0


def by_manifest_name(meta: dict, package_id: str) -> str:
    for pkg in meta["packages"]:
        if pkg["id"] == package_id:
            return pkg["name"]
    return "<unknown>"


if __name__ == "__main__":
    raise SystemExit(main())
