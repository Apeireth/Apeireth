"""修复 apeireth 项目 UTF-8 BOM bug — 22 个文件 line 1 有 U+FEFF.

主 9:15 真哲学审计:
  - 主人: "Apeireth 应该修好, 而不是新建"
  - 我建了 Phase 47 + Phase 7, 但没修这 22 个 broken 模块
  - 这是真技术债, 立刻修

MEMORY 教训 (2026-07-13 VCP):
  - 症状: PluginManager 静默跳过插件, 'Loaded manifest:' 但没你的插件
  - 根因: UTF-8 BOM (\ufeff)
  - 修复: utf-8-sig 读 → utf-8 写 (去 BOM)
  - 规则: 写任何 JSON 永远 ensure_ascii=False + 不加 BOM

这次是 .py 文件 BOM (同样问题).
"""
from __future__ import annotations

import os
import sys
from pathlib import Path


def fix_bom(file_path: Path) -> tuple[bool, str]:
    """修复单个文件的 UTF-8 BOM.

    Returns:
        (was_fixed, message)
    """
    raw = file_path.read_bytes()
    # 检查 BOM
    has_bom = raw.startswith(b'\xef\xbb\xbf')
    if not has_bom:
        return False, "no BOM"

    # 读 (utf-8-sig 自动去 BOM) → 写 (utf-8 无 BOM)
    try:
        text = raw.decode('utf-8-sig')
    except UnicodeDecodeError as e:
        return False, f"decode fail: {e}"

    file_path.write_bytes(text.encode('utf-8'))
    return True, "BOM removed"


def main():
    # 22 个已知 broken 文件
    broken_files = [
        'asi_demo_v5.py', 'deep_research_dual.py', 'extra_research_v10_night.py',
        'extra_research_v9.py', 'rust-substrate-check.py', 'v5_demo.py',
        'apeireth/active_inf.py', 'apeireth/asi_coordinator.py', 'apeireth/autopoiesis.py',
        'apeireth/complexity.py', 'apeireth/deliberation.py', 'apeireth/dgm_archive.py',
        'apeireth/karpathy_principles.py', 'apeireth/meta_cognition.py', 'apeireth/phi_proxy.py',
        'apeireth/physical_emergence.py', 'apeireth/self_model.py', 'apeireth/skill_library.py',
        'apeireth/systems_theory.py', 'apeireth/variety.py', 'apeireth/zvec_store.py',
        'apeireth/__init__.py',
    ]

    # 脚本在 scripts/ 子目录, project_root 是父目录
    project_root = Path(__file__).parent.parent
    fixed = 0
    skipped = 0
    failed = []

    print(f"扫描 {len(broken_files)} 个文件...")
    print(f"工作目录: {project_root}\n")

    for f in broken_files:
        path = project_root / f
        if not path.exists():
            print(f"  SKIP: {f} (文件不存在)")
            skipped += 1
            continue
        was_fixed, msg = fix_bom(path)
        if was_fixed:
            print(f"  FIXED: {f}")
            fixed += 1
        else:
            print(f"  OK:    {f} ({msg})")
            skipped += 1

    print(f"\n修复完成: {fixed} fixed, {skipped} skipped")
    print(f"\n下一步: 重新跑 ast.parse 验证")


def verify():
    """验证所有 .py 文件都没 BOM."""
    import ast
    project_root = Path(__file__).parent.parent
    broken = []
    checked = 0

    for root, dirs, files in os.walk(project_root):
        if '.git' in root or 'code-deep-study' in root or '__pycache__' in root:
            continue
        for f in files:
            if not f.endswith('.py'):
                continue
            path = Path(root) / f
            raw = path.read_bytes()
            if raw.startswith(b'\xef\xbb\xbf'):
                rel = path.relative_to(project_root)
                broken.append(str(rel))
            else:
                checked += 1

    print(f"\n验证结果:")
    print(f"  无 BOM 文件: {checked}")
    print(f"  有 BOM 文件: {len(broken)}")
    for b in broken:
        print(f"    - {b}")


if __name__ == "__main__":
    if len(sys.argv) > 1 and sys.argv[1] == "verify":
        verify()
    else:
        main()
        print()
        verify()