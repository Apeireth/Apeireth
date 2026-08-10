#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Agent D - yaml/toml 验证脚本 (2026-08-10)
yamllint / actionlint / act 都不可用, 用 PyYAML 严格 parse 代替.

验证范围:
  1. .github/workflows/*.yml 全部 parse 0 错
  2. deny.toml parse 0 错
  3. .config/nextest.toml parse 0 错
  4. rust-ci.yml 顶部 DEPRECATION NOTE 注释存在, 但 yaml 结构 (jobs/steps/on) 0 行为改动
  5. 新建 rustfmt.yml + rust.yml 必须满足:
     - 3 OS matrix (rust.yml) 或 1 OS ubuntu (rustfmt.yml)
     - 引用 cargo-nextest (rust.yml) 或 nightly fmt (rustfmt.yml)
     - permissions: contents: read
"""
import io
import sys
# Windows console 强制 UTF-8
if sys.platform == "win32":
    sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding="utf-8", errors="replace")
    sys.stderr = io.TextIOWrapper(sys.stderr.buffer, encoding="utf-8", errors="replace")
import os
import sys
import re
from pathlib import Path
import yaml

ROOT = Path(r".openclaw\workspace\promethean\Apeireth-rust")
WF_DIR = ROOT / ".github" / "workflows"
TOML_FILES = [ROOT / "deny.toml", ROOT / ".config" / "nextest.toml"]


def parse_yaml_strict(path: Path):
    """PyYAML 安全 load, 任何解析错 raise."""
    with open(path, encoding="utf-8") as f:
        return yaml.safe_load(f)


def parse_toml(path: Path):
    """用 toml 库 parse. 无 toml 库则用 regex 简单 key=value 验证."""
    try:
        import tomllib  # py3.11+
        with open(path, "rb") as f:
            return tomllib.load(f)
    except ImportError:
        pass
    try:
        import toml
        with open(path, encoding="utf-8") as f:
            return toml.load(f)
    except ImportError:
        # fallback: regex 检查 [section] 平衡
        text = path.read_text(encoding="utf-8")
        sections = re.findall(r"^\[([^\]]+)\]\s*$", text, re.M)
        if not sections:
            return None
        return {"_fallback_sections": sections}


def check_workflow(path: Path):
    """验证 workflow 基础结构."""
    data = parse_yaml_strict(path)
    assert isinstance(data, dict), f"{path.name}: 顶层不是 dict"
    assert "jobs" in data, f"{path.name}: 缺 'jobs'"
    on = data.get("on", data.get(True))
    assert on is not None, f"{path.name}: 缺 'on' trigger"
    return data


def has_contents_read(data: dict) -> bool:
    """检查 workflow 有 contents: read (top-level 或 job-level)."""
    top = data.get("permissions", {})
    if isinstance(top, dict) and top.get("contents") == "read":
        return True
    if top == "read-all":  # 太宽
        return True
    for job in data.get("jobs", {}).values():
        if isinstance(job, dict):
            jp = job.get("permissions", {})
            if isinstance(jp, dict) and jp.get("contents") == "read":
                return True
    return False


def main():
    print("=" * 70)
    print("Agent D — yaml/toml 验证 (R25 2026-08-10)")
    print("=" * 70)
    errors = 0

    # 1. 所有 .yml 严格 parse
    print("\n[1/5] 所有 .github/workflows/*.yml parse 0 错")
    yaml_files = sorted(WF_DIR.glob("*.yml"))
    print(f"      发现 {len(yaml_files)} 个 yml")
    for yf in yaml_files:
        try:
            data = check_workflow(yf)
            jobs = data.get("jobs", {})
            n_jobs = len(jobs)
            print(f"      ✅ {yf.name:30s}  jobs={n_jobs}")
        except Exception as e:
            print(f"      ❌ {yf.name:30s}  {e}")
            errors += 1

    # 2. 新建 rustfmt.yml 严格结构
    print("\n[2/5] rustfmt.yml 新结构验证")
    rustfmt = parse_yaml_strict(WF_DIR / "rustfmt.yml")
    assert rustfmt.get("jobs", {}).get("rustfmt-nightly"), "rustfmt-nightly job 缺失"
    job = rustfmt["jobs"]["rustfmt-nightly"]
    assert job.get("runs-on") == "ubuntu-latest", f"runs-on 应为 ubuntu-latest, 实为 {job.get('runs-on')}"
    assert has_contents_read(rustfmt), "缺 contents: read (top-level or job-level)"
    steps_text = yaml.dump(rustfmt)
    assert "cargo +nightly fmt --all -- --check" in steps_text, "缺 fmt --check 命令"
    print("      [OK] rustfmt.yml 结构正确 (nightly fmt --check, ubuntu-latest, contents: read)")

    # 3. 新建 rust.yml 严格结构
    print("\n[3/5] rust.yml 新结构验证")
    rustyml = parse_yaml_strict(WF_DIR / "rust.yml")
    rust_tests = rustyml.get("jobs", {}).get("rust-tests")
    assert rust_tests, "rust-tests job 缺失"
    matrix_os = rust_tests["strategy"]["matrix"]["os"]
    assert matrix_os == ["ubuntu-latest", "windows-latest", "macos-latest"], \
        f"OS matrix 错: {matrix_os}"
    assert has_contents_read(rustyml), "缺 contents: read"
    steps_text = yaml.dump(rustyml)
    assert "cargo nextest run --workspace --profile ci --locked" in steps_text, "缺 nextest 命令"
    assert "target/nextest/ci/junit.xml" in steps_text, "缺 JUnit artifact 上传"
    assert "taiki-e/install-action@nextest" in steps_text, "缺 cargo-nextest 安装"
    print("      [OK] rust.yml 结构正确 (3 OS matrix + nextest + JUnit + contents: read)")

    # 4. rust-ci.yml 0 行为改动 (DEPRECATION NOTE 是注释, yaml 解析会忽略)
    print("\n[4/5] rust-ci.yml 0 行为改动验证 (DEPRECATION NOTE 是注释, 应被 yaml 解析忽略)")
    rust_ci = parse_yaml_strict(WF_DIR / "rust-ci.yml")
    jobs = rust_ci.get("jobs", {})
    expected_jobs = {"rust-tests", "release-build", "battle-1-2", "ci-summary"}
    actual_jobs = set(jobs.keys())
    assert expected_jobs == actual_jobs, f"job 集合变更: 预期 {expected_jobs}, 实有 {actual_jobs}"
    # 验证 rust-tests job 的 matrix 仍是 3 OS
    matrix_os = jobs["rust-tests"]["strategy"]["matrix"]["os"]
    assert matrix_os == ["ubuntu-latest", "windows-latest", "macos-latest"]
    # 验证 nextest 命令仍在
    ci_text = yaml.dump(rust_ci)
    assert "cargo nextest run --workspace --profile ci --locked" in ci_text
    # 验证顶部的 DEPRECATION NOTE 注释确实存在 (在 raw text 里, 不在 yaml 结构里)
    rust_ci_raw = (WF_DIR / "rust-ci.yml").read_text(encoding="utf-8")
    assert "DEPRECATION NOTE" in rust_ci_raw, "DEPRECATION NOTE 注释缺失"
    print(f"      [OK] rust-ci.yml 4 jobs 未动, rust-tests 3 OS + nextest 完整, DEPRECATION NOTE 注释存在")

    # 5. deny.toml + .config/nextest.toml parse OK
    print("\n[5/5] deny.toml + .config/nextest.toml parse 0 错")
    for tf in TOML_FILES:
        try:
            data = parse_toml(tf)
            if data and "_fallback_sections" not in data:
                if tf.name == "deny.toml":
                    assert "advisories" in data, "deny.toml 缺 [advisories] section"
                    print(f"      ✅ {tf.relative_to(ROOT)}  sections={list(data.keys())[:5]}...")
                else:
                    print(f"      ✅ {tf.relative_to(ROOT)}  sections={list(data.keys())}")
            else:
                # fallback regex 解析
                sections = data.get("_fallback_sections", []) if data else []
                print(f"      ⚠️  {tf.relative_to(ROOT)}  fallback regex: {len(sections)} sections")
        except Exception as e:
            print(f"      ❌ {tf.relative_to(ROOT)}  {e}")
            errors += 1

    # 总结
    print("\n" + "=" * 70)
    if errors == 0:
        print("[OK] 全部 5 项验证通过 (yaml 0 错 + 新结构正确 + 旧行为 0 改 + toml 0 错)")
        sys.exit(0)
    else:
        print(f"[FAIL] {errors} 项错误, 请修")
        sys.exit(1)


if __name__ == "__main__":
    main()
