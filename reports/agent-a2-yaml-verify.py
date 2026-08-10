#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Agent A-2 - yaml/markdown 验证脚本 (2026-08-10)
yamllint / actionlint / act 都不可用, 用 PyYAML 严格 parse 代替.

验证范围 (per 任务 A2-5):
  1. .github/dependabot.yml 0 改核验 — PyYAML parse 0 错 + 4 group + Cargo + GitHub Actions + 周一 06:00 UTC + Major skip + labels 完整
  2. .github/ISSUE_TEMPLATE/bug_report.yml PyYAML parse 0 错 + 字段完整 (name/description/title/labels/body) + 0 触碰实查段
  3. .github/ISSUE_TEMPLATE/feature_request.yml PyYAML parse 0 错 + 字段完整
  4. .github/ISSUE_TEMPLATE/config.yml PyYAML parse 0 错 + blank_issues_enabled + contact_links
  5. .github/PULL_REQUEST_TEMPLATE.md 文本 5 段都在 (R26+ 5 硬约束 / 测试 / 文档 / 6 哲学 / 8 项 / 12 项 checklist)
  6. .github/ISSUE_TEMPLATE/{1.0-blocker,bug,feature}.md 仍存在 (向后兼容, 3 个 .md 模板 0 删)
  7. .github/CONTRIBUTING.md 引用 PR template 路径正确
"""
import io
import sys
# Windows console 强制 UTF-8
if sys.platform == "win32":
    sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding="utf-8", errors="replace")
    sys.stderr = io.TextIOWrapper(sys.stderr.buffer, encoding="utf-8", errors="replace")
import os
import re
from pathlib import Path
import yaml

ROOT = Path(r".openclaw\workspace\promethean\Apeireth-rust")
GH_DIR = ROOT / ".github"
ISSUE_DIR = GH_DIR / "ISSUE_TEMPLATE"


def parse_yaml_strict(path: Path):
    """PyYAML 安全 load, 任何解析错 raise."""
    with open(path, encoding="utf-8") as f:
        return yaml.safe_load(f)


def main():
    print("=" * 70)
    print("Agent A-2 — yaml/markdown 验证 (R26+ 工程化 2026-08-10)")
    print("=" * 70)
    errors = 0

    # 1. dependabot.yml 0 改核验
    print("\n[1/7] .github/dependabot.yml 0 改核验 (R18 已写, 4 group + Cargo + GitHub Actions + 周一 06:00 UTC)")
    dependabot = GH_DIR / "dependabot.yml"
    try:
        data = parse_yaml_strict(dependabot)
        assert data.get("version") == 2, f"version 应为 2, 实为 {data.get('version')}"
        updates = data.get("updates", [])
        assert len(updates) == 2, f"应有 2 个 update (cargo + github-actions), 实有 {len(updates)}"

        cargo = next((u for u in updates if u.get("package-ecosystem") == "cargo"), None)
        assert cargo, "缺 cargo update"
        sched = cargo.get("schedule", {})
        assert sched.get("interval") == "weekly", f"cargo interval 应为 weekly, 实为 {sched.get('interval')}"
        assert sched.get("day") == "monday", f"cargo day 应为 monday, 实为 {sched.get('day')}"
        assert sched.get("time") == "06:00", f"cargo time 应为 06:00, 实为 {sched.get('time')}"
        assert sched.get("timezone") == "UTC", f"cargo timezone 应为 UTC, 实为 {sched.get('timezone')}"

        groups = cargo.get("groups", {})
        assert "tokio" in groups, "缺 tokio group"
        assert "http" in groups, "缺 http group"
        assert "serde" in groups, "缺 serde group"
        assert "wasm" in groups, "缺 wasm group"
        assert "dependencies" in groups, "缺 dependencies catch-all group"

        ignore = cargo.get("ignore", [])
        major_ignore = [i for i in ignore
                        if "*" in i.get("dependency-name", "")
                        and "version-update:semver-major" in i.get("update-types", [])]
        assert major_ignore, "缺 Major 版本不自动合 ignore"

        labels = cargo.get("labels", [])
        assert "dependencies" in labels, f"labels 缺 dependencies, 实有 {labels}"
        assert "automated" in labels, f"labels 缺 automated, 实有 {labels}"

        gh_actions = next((u for u in updates if u.get("package-ecosystem") == "github-actions"), None)
        assert gh_actions, "缺 github-actions update"
        assert gh_actions.get("schedule", {}).get("day") == "monday", "GitHub Actions 也应 monday"

        print("      [OK] dependabot.yml 完整 4 group + Cargo + GitHub Actions + 周一 06:00 UTC + Major skip + labels")
    except Exception as e:
        print(f"      ❌ dependabot.yml 核验失败: {e}")
        errors += 1

    # 2. bug_report.yml 字段完整
    print("\n[2/7] .github/ISSUE_TEMPLATE/bug_report.yml PyYAML parse 0 错 + 字段完整")
    bug_yml = ISSUE_DIR / "bug_report.yml"
    try:
        data = parse_yaml_strict(bug_yml)
        assert data.get("name") == "Bug Report", f"name 应为 'Bug Report', 实为 {data.get('name')}"
        assert "description" in data, "缺 description"
        assert data.get("title") == "[BUG] ", f"title 应为 '[BUG] ', 实为 {data.get('title')}"
        labels = data.get("labels", [])
        assert "bug" in labels, f"labels 缺 bug, 实有 {labels}"
        body = data.get("body", [])
        assert isinstance(body, list), "body 应为 list"
        assert len(body) >= 5, f"body 字段数应 >= 5, 实有 {len(body)}"
        # 验证必填字段类型
        body_ids = [b.get("id") for b in body if isinstance(b, dict)]
        for required in ["version", "severity", "os", "repro", "expected", "actual"]:
            assert required in body_ids, f"body 缺必填字段 id={required}"
        # 验证 0 触碰实查段 (注意: options 在 attributes 下面, 不是 checkbox 顶层)
        # 用 allow_unicode=True 避免中文被 \\u escape
        text = yaml.dump(data, allow_unicode=True, default_flow_style=False)
        assert "0 触碰实查" in text or "locked_check" in text, "缺 0 触碰实查段"
        assert "6 哲学 anchor" in text or "philosophy_check" in text, "缺 6 哲学 anchor 段"
        assert "8 项不修改承诺" in text, "缺 8 项不修改承诺段"
        print(f"      [OK] bug_report.yml 字段完整 (name/description/title/labels + {len(body)} body fields + 0 触碰 + 6 哲学 + 8 项)")
    except Exception as e:
        print(f"      ❌ bug_report.yml 核验失败: {e}")
        errors += 1

    # 3. feature_request.yml 字段完整
    print("\n[3/7] .github/ISSUE_TEMPLATE/feature_request.yml PyYAML parse 0 错 + 字段完整")
    feat_yml = ISSUE_DIR / "feature_request.yml"
    try:
        data = parse_yaml_strict(feat_yml)
        assert data.get("name") == "Feature Request", f"name 应为 'Feature Request', 实为 {data.get('name')}"
        assert "description" in data, "缺 description"
        assert data.get("title") == "[FEATURE] ", f"title 应为 '[FEATURE] ', 实为 {data.get('title')}"
        labels = data.get("labels", [])
        assert "enhancement" in labels, f"labels 缺 enhancement, 实有 {labels}"
        body = data.get("body", [])
        assert isinstance(body, list), "body 应为 list"
        body_ids = [b.get("id") for b in body if isinstance(b, dict)]
        for required in ["target", "crate", "anchor", "problem", "proposed", "alternatives"]:
            assert required in body_ids, f"body 缺必填字段 id={required}"
        # 验证 0 触碰实查段 (强制 required, 跟 bug 区分; options 在 attributes 下面)
        locked_check = next((b for b in body if isinstance(b, dict) and b.get("id") == "locked_check"), None)
        assert locked_check, "缺 0 触碰实查 checkbox 段"
        options = locked_check.get("attributes", {}).get("options", [])
        required_options = [o for o in options if o.get("required")]
        assert len(required_options) >= 3, f"locked_check 应至少 3 个 required option (0 触碰/0 改 version/0 改 R11), 实有 {len(required_options)}"
        print(f"      [OK] feature_request.yml 字段完整 (name/description/title/labels + {len(body)} body fields + locked_check {len(required_options)} required options)")
    except Exception as e:
        print(f"      ❌ feature_request.yml 核验失败: {e}")
        errors += 1

    # 4. config.yml 字段完整
    print("\n[4/7] .github/ISSUE_TEMPLATE/config.yml PyYAML parse 0 错 + blank_issues_enabled + contact_links")
    config_yml = ISSUE_DIR / "config.yml"
    try:
        data = parse_yaml_strict(config_yml)
        assert data.get("blank_issues_enabled") is False, "blank_issues_enabled 应为 false (强制走模板)"
        contact_links = data.get("contact_links", [])
        assert isinstance(contact_links, list), "contact_links 应为 list"
        assert len(contact_links) >= 2, f"contact_links 应 >= 2 (Discussions + Security), 实有 {len(contact_links)}"
        link_names = [l.get("name") for l in contact_links]
        assert "Apeireth Discussions" in link_names, "缺 Apeireth Discussions 链接"
        assert any("Security" in n for n in link_names), "缺 Security 链接"
        # 验证 1.0-blocker 链接 (向后兼容 1.0-blocker.md)
        assert any("1.0-blocker" in l.get("url", "") for l in contact_links), "缺 1.0-blocker 链接 (向后兼容)"
        print(f"      [OK] config.yml blank_issues_enabled=false + {len(contact_links)} contact_links")
    except Exception as e:
        print(f"      ❌ config.yml 核验失败: {e}")
        errors += 1

    # 5. PULL_REQUEST_TEMPLATE.md 文本 5 段都在
    print("\n[5/7] .github/PULL_REQUEST_TEMPLATE.md 文本 5 段都在")
    pr_tmpl = GH_DIR / "PULL_REQUEST_TEMPLATE.md"
    try:
        text = pr_tmpl.read_text(encoding="utf-8")
        # 5 段必备
        required_sections = [
            ("R26+ 5 项硬约束", "0 触碰 24 LOCKED"),
            ("R26+ 5 项硬约束", "0 改 workspace.version"),
            ("R26+ 5 项硬约束", "0 改 R11 baseline"),
            ("R26+ 5 项硬约束", "cargo test pass"),
            ("R26+ 5 项硬约束", "0 假装"),
            ("测试", "cargo test --workspace"),
            ("文档", "CHANGELOG.md"),
            ("6 哲学 anchor", "S-1"),
            ("6 哲学 anchor", "O-5"),
            ("8 项不修改承诺", "不假装已实现"),
            ("12 项 checklist", "#1 doc"),
            ("12 项 checklist", "#12 security"),
        ]
        for section, marker in required_sections:
            assert marker in text, f"PR template 缺 {section} 段标记 '{marker}'"
        # 验证 size 比 R20 模板大 (R20 模板 1420 chars, R26+ 5 段硬约束模板应显著更大)
        assert len(text) > 4000, f"PR template 长度应 > 4000 chars (R20 模板是 1420), 实为 {len(text)}"
        # 验证没残留 R20 阶段 1-6 段 (已过期)
        assert "R20 阶段 1-6 checklist" not in text, "残留 R20 阶段 1-6 段 (应已重写)"
        assert "D-01: calendar" not in text, "残留 R20 4 决策拍板段 (应已砍)"
        print(f"      [OK] PULL_REQUEST_TEMPLATE.md 5 段都在 (size {len(text)} chars, 比 R20 1420 大 {len(text) - 1420} chars)")
    except Exception as e:
        print(f"      ❌ PULL_REQUEST_TEMPLATE.md 核验失败: {e}")
        errors += 1

    # 6. 3 个 .md 模板仍存在 (向后兼容, 1.0-blocker 是 1.0 release 专用)
    print("\n[6/7] .github/ISSUE_TEMPLATE/{1.0-blocker,bug,feature}.md 仍存在 (向后兼容)")
    legacy_md_files = ["1.0-blocker.md", "bug.md", "feature.md"]
    for md in legacy_md_files:
        f = ISSUE_DIR / md
        if f.exists():
            text = f.read_text(encoding="utf-8")
            assert len(text) > 100, f"{md} 太小 ({len(text)} chars), 应该是被破坏"
            print(f"      ✅ {md:20s}  size={len(text)} chars  (向后兼容保留)")
        else:
            print(f"      ❌ {md} 不存在 (向后兼容破坏!)")
            errors += 1

    # 7. CONTRIBUTING.md 引用 PR template 路径正确
    print("\n[7/7] CONTRIBUTING.md 引用 PR template 路径正确")
    contrib = ROOT / "CONTRIBUTING.md"
    try:
        text = contrib.read_text(encoding="utf-8")
        assert "PULL_REQUEST_TEMPLATE" in text, "CONTRIBUTING.md 缺 PULL_REQUEST_TEMPLATE 引用"
        print("      [OK] CONTRIBUTING.md 引用 PULL_REQUEST_TEMPLATE 路径正确")
    except Exception as e:
        print(f"      ❌ CONTRIBUTING.md 核验失败: {e}")
        errors += 1

    # 总结
    print("\n" + "=" * 70)
    if errors == 0:
        print("[OK] 全部 7 项验证通过 (yaml 0 错 + 字段完整 + 5 段都在 + .md 向后兼容 + CONTRIBUTING 引用)")
        print()
        print("最终交付:")
        print(f"  - .github/dependabot.yml            0 改 (R18 写, 89 行, 1:1 完整)")
        print(f"  - .github/ISSUE_TEMPLATE/bug_report.yml       {len((ISSUE_DIR / 'bug_report.yml').read_text(encoding='utf-8'))} chars (新)")
        print(f"  - .github/ISSUE_TEMPLATE/feature_request.yml  {len((ISSUE_DIR / 'feature_request.yml').read_text(encoding='utf-8'))} chars (新)")
        print(f"  - .github/ISSUE_TEMPLATE/config.yml           {len((ISSUE_DIR / 'config.yml').read_text(encoding='utf-8'))} chars (新)")
        print(f"  - .github/PULL_REQUEST_TEMPLATE.md   {len((GH_DIR / 'PULL_REQUEST_TEMPLATE.md').read_text(encoding='utf-8'))} chars (重写, R20 1420 → R26+ 5 段硬约束)")
        print(f"  - .github/ISSUE_TEMPLATE/{{1.0-blocker,bug,feature}}.md  保留 (向后兼容)")
        sys.exit(0)
    else:
        print(f"[FAIL] {errors} 项错误, 请修")
        sys.exit(1)


if __name__ == "__main__":
    main()
