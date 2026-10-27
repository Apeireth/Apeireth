#!/usr/bin/env python3
"""CI **逐步**门禁覆盖检查：CI 跑的每一步，本地门禁都跑了（或被显式豁免并写了理由）。

**为什么要有它（2026-10-27 实战踩到）**：
CI 的 `Secret scan (R215 防御层)` job 有**两步**：
① `pwsh scripts/secret-scan.ps1 -Mode scan-all`（扫工作树）
② `pwsh scripts/secret-scan.ps1 -Mode scan-history`（扫**整个 git 历史**，防 force-push 偷渡）

⇒ 我此前的本地门禁**只按 workflow 文件清单提取**，只跑了第 ① 步
⇒ ⇒ **CI 红（历史里的假密钥字面量）而本地绿** ⇒ ⇒ 「门禁全绿」**不等于**「CI 会绿」。
⇒ ⇒ 本脚本把这条教训变成**机制**：CI 的每一步要么在本地门禁里，要么被显式归类
为 **CI 专属**（需网络 / 需特殊工具链 / 跨平台 / 每 target 矩阵）**并写明理由**。

**用法**:
    python3 scripts/check_ci_gate_coverage.py            # 报告（有问题则退出码 1）
    python3 scripts/check_ci_gate_coverage.py --quiet    # 只在有问题时输出
"""
import argparse
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

# ── CI 专属步：需要网络 / 特殊工具链 / 跨平台 / 按矩阵逐个跑 ⇒ 本地**不**逐条跑。
#    ⇒ **每条都必须写清理由**（否则本脚本把它当"漏了"）。
CI_ONLY: dict[str, str] = {
    "cargo install": "装工具链（CI 一次性准备，本地不需要）",
    "cargo audit": "需访问 crates.io advisory 数据库（网络）",
    "cargo deny": "需访问 crates.io + advisory 数据库（网络）",
    "cargo deb": "跨平台打包（aarch64 目标需额外工具链）",
    "cargo rpm": "跨平台打包",
    "cargo kani": "Kani 模型检验（需 kani-verifier 专用工具链）",
    "cargo +nightly miri": "Miri（需专用 nightly + 解释器，比测试慢一个量级）",
    "cargo +nightly doc": "文档构建（CI 专属产物检查）",
    "cargo nextest": "CI 用 nextest 跑测试（本地用 cargo test ⇒ 等价覆盖）",
    "cargo +nightly fuzz run": "fuzz 矩阵（CI 按 target 逐个跑；本地只跑 + 全 target 编译关卡）",
    "cargo run -p apeireth-guard": "导出场景样例（CI 专属产物）",
    "docker": "容器相关",
}

# ── 本地门禁脚本（仓外，不在 git 里）：比对它**实际跑了什么**。
GATES_SH = Path("/root/Documents/deepseek-harness/default-workspace/gates.sh")

# ── 只关心这些前缀（其余是安装/打包/产物类）。
GATE_PREFIXES = (
    "cargo fmt",
    "cargo clippy",
    "cargo test",
    "cargo check",
    "cargo +nightly check",
    "python3 scripts/",
    "pwsh scripts/",
)


def classify(step: str) -> str:
    """返回 `local` / `ci-only` / `other`。"""
    normalized = " ".join(step.split())
    for needle, _reason in CI_ONLY.items():
        if normalized.startswith(needle):
            return "ci-only"
    for prefix in GATE_PREFIXES:
        if normalized.startswith(prefix):
            return "local"
    return "other"


def extract_steps() -> list[tuple[str, str]]:
    """产出 `(workflow 文件, 步命令)`。"""
    steps: list[tuple[str, str]] = []
    workflows = sorted((ROOT / ".github" / "workflows").glob("*.yml"))
    for workflow in workflows:
        text = workflow.read_text(encoding="utf-8", errors="replace")
        for line in text.splitlines():
            match = re.match(r"^\s*run:\s*(.+?)\s*$", line)
            if not match:
                continue
            command = match.group(1).strip().strip("'\"")
            # 去掉 YAML 块标量（`run: |`）——那些是多行脚本，按第一行粗略取
            if command in ("|", ">", "|-", ">-"):
                continue
            steps.append((workflow.name, command))
    return steps


def main() -> int:
    parser = argparse.ArgumentParser(description="CI 逐步门禁覆盖检查")
    parser.add_argument("--quiet", action="store_true", help="只在有问题时输出")
    args = parser.parse_args()

    steps = extract_steps()
    local_steps = [s for s in steps if classify(s[1]) == "local"]
    ci_only_steps = [s for s in steps if classify(s[1]) == "ci-only"]

    gates_text = ""
    if GATES_SH.exists():
        gates_text = GATES_SH.read_text(encoding="utf-8", errors="replace")
    else:
        print(f"[ci-gates] ⚠️ 找不到本地门禁脚本: {GATES_SH}")
        return 1

    # ⚠️⚠️⚠️ **比对必须只取「真正执行的命令」**（2026-10-27 证伪了两次才发现）：
    # ① 第一版拿**整个文件**的文本匹配 ⇒ ⇒ **注释里提到某命令**就算"已覆盖" ⇒ **假绿**；
    # ② 改成「只取非注释行」后仍假绿 ⇒ 因为 `printf '...%s' "cargo check --workspace ..."`
    #    这类**显示名（label）**里也含该命令 ⇒ ⇒ 必须解析 `run "<标签>" <命令>` 的
    #    **命令部分**（标签 ≠ 命令）。
    # ⇒ ⇒ 现在：**只收 `run`/`run_long` 的命令参数** + **独立的命令行**；
    #      `printf`（标签）、注释、`if`/`fi` 等结构行**一律不算**。
    RUN_RE = re.compile("^\\s*run(?:_long)?\\s+(?:\"[^\"]*\"|'[^']*')\\s+(.+?)\\s*$")
    commands: list[str] = []
    for line in gates_text.splitlines():
        stripped = line.strip()
        if not stripped or stripped.startswith("#"):
            continue
        match = RUN_RE.match(line)
        if match:
            commands.append(match.group(1))
            continue
        # 独立的命令行（不在 run 调用里，也不是控制结构/标签）
        # `if <命令>; then` / `elif <命令>; then` ⇒ **条件里就是被执行的命令**
        # （仓内门禁脚本大量用这种写法跑 python 检查）⇒ ⇒ 提取出来。
        if stripped.startswith(("if ", "elif ")):
            body = stripped[3:].strip()
            for terminator in ("; then", " then"):
                if body.endswith(terminator):
                    body = body[: -len(terminator)].strip()
            if body:
                commands.append(body)
            continue
        if stripped.startswith(("printf", "then", "else", "fi", "for", "do", "done",
                                "while", "case", "esac", "}", "{", "export", "set", "cd ", "echo ")):
            continue
        commands.append(stripped)
    normalized_gates = " ".join(" ".join(commands).split())

    # ── **包含关系**：CI 跑的是本地门禁的**子集** ⇒ 算覆盖
    # （例：CI 的 `cargo test -p apeireth-tools-canonical` ⊂ 本地 `cargo test --workspace`）
    SUBSUMED_BY = {
        "cargo test": "cargo test --workspace",
        "cargo check": "cargo check --workspace --all-targets",
        "cargo clippy": "cargo clippy --workspace --all-targets",
        "cargo fmt": "cargo fmt --all --check",
    }

    missing: list[tuple[str, str]] = []
    for workflow, command in local_steps:
        normalized = " ".join(command.split())
        core = normalized.split("--")[0].strip()
        if not core:
            continue
        probe = " ".join(core.split()[:3])
        if probe and probe in normalized_gates:
            continue
        # 包含关系判定
        head = core.split()[0:2]
        head = " ".join(head)
        covered = False
        for prefix, broader in SUBSUMED_BY.items():
            if head.startswith(prefix) and broader in normalized_gates:
                covered = True
                break
        if covered:
            continue
        missing.append((workflow, normalized))

    if not missing:
        if not args.quiet:
            print(
                f"[ci-gates] ✅ CI 的 **{len(local_steps)}** 个本地门禁型步骤"
                f"**全部**在本地门禁里；**{len(ci_only_steps)}** 个 CI 专属步骤已显式归类"
            )
        return 0

    print(f"[ci-gates] ❌ {len(missing)} 个 CI 门禁步**本地没跑**（且未归类为 CI 专属）：")
    for workflow, command in missing:
        print(f"   · {workflow}: {command[:120]}")
    print("   → 要么把它加进本地门禁（`gates.sh`），要么在 `CI_ONLY` 里显式豁免并写明理由")
    print("   → 教训：**门禁表必须照抄 CI job 的每一步**，不是照抄 job 的名字")
    return 1


if __name__ == "__main__":
    sys.exit(main())
