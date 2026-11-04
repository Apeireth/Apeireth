# ============================================================================
# Makefile — TP20-S5 塞缝批 + post-1.0.0 增量 (2026-08-19)
# ----------------------------------------------------------------------------
# 一键发布期供应链验证 + SBOM 生成 + 日常开发检查。
#
# 与 scripts/vet.sh + scripts/sbom.sh + scripts/release-prep.sh 的关系:
#   - scripts/* 是真正的执行体, Makefile 只是 thin wrapper + 入口聚合
#   - CI release.yml 调 scripts/ 直跑 (跨平台 shell 不依赖 make)
#   - 本地开发者 + 主人手动快速验证用 Makefile (跨平台兼容性: Linux/macOS
#     make 都有, Windows 走 WSL 或 chocolatey install make)
#
# 用法见 help target.
#
# 口径对齐 (2026-11-04, 工具链面):
#   - 版本轴真源   = 根 Cargo.toml [workspace.package].version (现 2.0.0-rc.5)
#   - crate 数真源 = AGENTS.md §3 (workspace = 31 members)
#   - 全量门禁唯一执行器 = scripts/gate_full.ps1 (基线数字真源 = docs/03-reference/test-baseline.md §1)
#   - make ci = rust.yml 部分步骤的本地复刻, **不等于** push 后 GitHub Actions 必绿
# ============================================================================

# 强制 bash (脚本用 bash 语法, Windows make 默认 sh 不一定支持)
SHELL := /usr/bin/env bash

# 路径
SCRIPTS_DIR := scripts
REPORTS_DIR := reports
TOOLS_DIR   := tools

# 默认 target: help
.DEFAULT_GOAL := help

# 假目标声明 (避免与同名文件冲突)
.PHONY: help audit sbom release-check release-prep release-prep-block test check clippy fmt fmt-check gate-full tools-install tools-check clean-vet clean-sbom ci-build ci-test ci-release ci

# ----------------------------------------------------------------------------
# help — 列出所有 target
# ----------------------------------------------------------------------------
help: ## 列出所有 make target
	@echo "=========================================="
	@echo "  Apeireth 开发 + 发布期工具链 (v2.0 工程重构, 版本轴 2.0.0-rc.5)"
	@echo "=========================================="
	@echo ""
	@echo "  === 日常开发 (本地反馈环) ==="
	@echo "  make check            cargo check --workspace --all-targets --locked (增量编译)"
	@echo "  make test             cargo test --workspace --all-targets --locked --no-fail-fast -j 2"
	@echo "                         (31 members 单次全量; 图像家族 11 crate 未拆出独立跑)"
	@echo "  make clippy           cargo clippy --workspace --all-targets --locked -- -D warnings"
	@echo "  make fmt              cargo fmt --all (格式)"
	@echo "  make fmt-check        cargo fmt --all --check (CI 模式, 0 diff exit 0)"
	@echo ""
	@echo "  === 全量门禁 (唯一执行器) ==="
	@echo "  make gate-full        pwsh scripts/gate_full.ps1 -FailIfBusy"
	@echo "                         (test + clippy + fmt; 图像家族 11 crate 拆出独立跑)"
	@echo "  ↑ 全量数字与基线口径真源 = docs/03-reference/test-baseline.md §1 (本文件不抄数字)"
	@echo ""
	@echo "  === CI 局部复刻 (本地 == GitHub Actions 的部分步骤, 非全量等价) ==="
	@echo "  make ci               ci-build + ci-test + ci-release"
	@echo "  make ci-build         cargo build --workspace --tests --locked  (rust.yml line 71)"
	@echo "  make ci-test          cargo nextest run --workspace --profile ci --locked (rust.yml line 75)"
	@echo "  make ci-release       cargo build --release --workspace --locked (本地 release 前置构建; rust.yml 无对应 step)"
	@echo "  ↑ make ci 不覆盖: legacy 依赖检查 / 3 OS 矩阵 / 进程隔离与治理链专项测试 /"
	@echo "    8 硬墙守门 job / secret-scan (scan-all + scan-history)"
	@echo "  ↑ 任 1 个 exit != 0 → 不要 push (本地失败必红, 但本地绿 ≠ CI 必绿)"
	@echo ""
	@echo "  === 发布期 (版本轴 2.0.0-rc.5) ==="
	@echo "  make tools-install    装 cargo-vet/audit/deny/cyclonedx (best-effort)"
	@echo "  make tools-check      检查工具链是否齐全 (exit 0 = 缺也 OK, 仅打印)"
	@echo "  make audit            跑 cargo vet + audit + deny (scripts/vet.sh)"
	@echo "  make sbom             生成 CycloneDX 1.5 SBOM (scripts/sbom.sh)"
	@echo "  make release-prep     8 硬墙 + PII + 12 项 checklist (scripts/release-prep.sh)"
	@echo "  ↑ master-only gate (per 601e5c21), 在 main 上脚本自动跳过"
	@echo "  make release-prep-block  同上, BLOCKING 模式 (1 P0 fail 退出 1)"
	@echo "  make release-check    audit + sbom (v2 release 前必跑)"
	@echo ""
	@echo "  === 清理 ==="
	@echo "  make clean-vet        清 reports/ 里 tp20-s5-* 日志"
	@echo "  make clean-sbom       删 cyclonedx-sbom.json + sbom-cyclonedx.stderr.txt"
	@echo "  make help             本帮助"
	@echo ""

# ----------------------------------------------------------------------------
# tools-install — best-effort 装 4 个工具链 (失败不阻断)
# ----------------------------------------------------------------------------
tools-install: ## 装 cargo-vet / cargo-audit / cargo-deny / cargo-cyclonedx
	@echo "=== 装 cargo-vet (进 tools/) ==="
	cargo install cargo-vet --locked --root $(TOOLS_DIR) || \
		echo "  ⚠️  cargo-vet 装失败 (本批 fallback: SKIP vet 这一步, CI release.yml 必装)"
	@echo ""
	@echo "=== 装 cargo-audit (进 ~/.cargo/bin) ==="
	cargo install cargo-audit --locked || \
		echo "  ⚠️  cargo-audit 装失败 (本批 fallback: SKIP audit 这一步)"
	@echo ""
	@echo "=== 装 cargo-deny (进 ~/.cargo/bin) ==="
	cargo install cargo-deny --locked || \
		echo "  ⚠️  cargo-deny 装失败 (本批 fallback: SKIP deny 这一步)"
	@echo ""
	@echo "=== 装 cargo-cyclonedx (进 ~/.cargo/bin) ==="
	cargo install cargo-cyclonedx --locked || \
		echo "  ⚠️  cargo-cyclonedx 装失败 (本批 fallback: SKIP SBOM 这一步)"
	@echo ""
	@echo "=== tools-install 完成 (含失败项) ==="

# ----------------------------------------------------------------------------
# tools-check — 检查 4 个工具链是否齐全 (exit 0, 仅打印状态)
# ----------------------------------------------------------------------------
tools-check: ## 检查工具链是否齐全
	@echo "=== 工具链状态 (TP20-S5) ==="
	@command -v cargo-vet        >/dev/null 2>&1 && echo "  ✅ cargo-vet"        || echo "  ❌ cargo-vet (装: cargo install cargo-vet --locked)"
	@command -v cargo-audit      >/dev/null 2>&1 && echo "  ✅ cargo-audit"      || echo "  ❌ cargo-audit (装: cargo install cargo-audit --locked)"
	@command -v cargo-deny       >/dev/null 2>&1 && echo "  ✅ cargo-deny"       || echo "  ❌ cargo-deny (装: cargo install cargo-deny --locked)"
	@command -v cargo-cyclonedx  >/dev/null 2>&1 && echo "  ✅ cargo-cyclonedx"  || echo "  ❌ cargo-cyclonedx (装: cargo install cargo-cyclonedx --locked)"
	@command -v jq               >/dev/null 2>&1 && echo "  ✅ jq"               || echo "  ❌ jq (装: choco/apt/brew install jq)"
	@echo ""

# ----------------------------------------------------------------------------
# audit — 三件套 vet + audit + deny (scripts/vet.sh)
# ----------------------------------------------------------------------------
audit: ## 跑 cargo vet + audit + deny
	@bash $(SCRIPTS_DIR)/vet.sh

# ----------------------------------------------------------------------------
# check / test / clippy / fmt — 增量编译 + 全测试 + lint + 格式 (本地反馈环)
# ----------------------------------------------------------------------------
# 口径: 命令形态按 AGENTS.md §5 (三件套, 必带 --locked / --no-fail-fast -j 2);
#       test 的**全量口径** (图像家族 11 crate 拆出独立跑) 与基线数字真源 =
#       docs/03-reference/test-baseline.md §1, 唯一执行器 = gate-full 目标。
# ----------------------------------------------------------------------------
check: ## cargo check --workspace --all-targets --locked
	cargo check --workspace --all-targets --locked

test: ## cargo test --workspace --all-targets --locked --no-fail-fast -j 2 (本地单次全量)
	cargo test --workspace --all-targets --locked --no-fail-fast -j 2

clippy: ## cargo clippy --workspace --all-targets --locked -- -D warnings
	cargo clippy --workspace --all-targets --locked -- -D warnings

fmt: ## cargo fmt --all
	cargo fmt --all

fmt-check: ## cargo fmt --all --check (CI 模式, 0 diff exit 0)
	cargo fmt --all --check

# ----------------------------------------------------------------------------
# gate-full — 全量门禁唯一执行器 (test-baseline.md §1)
# ----------------------------------------------------------------------------
# 说明: 本地 `test` 目标 = 单次全量 (图像家族 11 crate 未拆出); 全量门禁口径 =
#       拆出图像家族独立跑 + clippy + fmt, 由 scripts/gate_full.ps1 独家实现
#       (vendored 清单真源 = contract.toml [vendored].image_family, 禁止手抄)。
#       本目标只做转发, 不复制执行逻辑。
# ----------------------------------------------------------------------------
gate-full: ## 唯一全量门禁 (pwsh scripts/gate_full.ps1 -FailIfBusy)
	@echo "=== gate-full: pwsh scripts/gate_full.ps1 -FailIfBusy ==="
	pwsh scripts/gate_full.ps1 -FailIfBusy

# ----------------------------------------------------------------------------
# CI 局部复刻 — 对应 .github/workflows/rust.yml 的部分步骤
# ----------------------------------------------------------------------------
# 覆盖: rust.yml `rust-tests` job 的编译 + 测试主命令 (line 71 / line 75), 本地单 OS 跑。
# 不覆盖: legacy 依赖检查 (scripts/check_no_legacy_deps.py) / 3 OS 矩阵 + Linux 系统库 /
#   canonical 进程隔离集成测试 (line 78) / 治理链演练 (line 82) / 8 硬墙守门 job /
#   secret-scan (scan-all + scan-history) / JUnit 上传, 以及其余 workflow
#   (lint / miri / rustdoc / audit / deny / coverage 等)。
# 纪律: 本地 exit 0 只说明本地这几条命令通过, **不承诺** push 后 GitHub Actions 必绿。
# 全量门禁口径见 docs/03-reference/test-baseline.md §1 (唯一执行器 = make gate-full)。
# ----------------------------------------------------------------------------

ci-build: ## cargo build --workspace --tests --locked  (rust.yml line 71)
	@echo "=== ci-build: cargo build --workspace --tests --locked ==="
	cargo build --workspace --tests --locked

ci-test: ## cargo nextest run --workspace --profile ci --locked  (rust.yml line 75)
	@echo "=== ci-test: cargo nextest run --workspace --profile ci --locked ==="
	@command -v cargo-nextest >/dev/null 2>&1 || { \
		echo "  ❌ cargo-nextest 未装, 跑 'cargo install cargo-nextest --locked' 后重试"; \
		exit 1; }
	cargo nextest run --workspace --profile ci --locked

ci-release: ## cargo build --release --workspace --locked  (release validation)
	@echo "=== ci-release: cargo build --release --workspace --locked ==="
	cargo build --release --workspace --locked

ci: ci-build ci-test ci-release ## ci-build + ci-test + ci-release (rust.yml 主编译/测试两步 + 本地 release 构建)
	@echo ""
	@echo "=========================================="
	@echo "  ✅ make ci 通过 — 仅代表本地这几条命令 exit 0"
	@echo "  ↑ 不等于 push 后 GitHub Actions 必绿: rust.yml 另有本地不覆盖的步骤"
	@echo "    (legacy 依赖检查 / 3 OS 矩阵 / 进程隔离与治理链专项测试 /"
	@echo "     8 硬墙守门 job / secret-scan)"
	@echo "  ↑ 全量门禁口径见 docs/03-reference/test-baseline.md §1 (make gate-full)"
	@echo "=========================================="

# ----------------------------------------------------------------------------
# release-prep — 8 硬墙 + PII + 12 项 checklist (post-1.0.0)
# ----------------------------------------------------------------------------
release-prep: ## 8 硬墙 + PII + 12 项 checklist (scripts/release-prep.sh, post-1.0.0)
	@bash $(SCRIPTS_DIR)/release-prep.sh --dry-run

release-prep-block: ## 8 硬墙 + PII + 12 项 checklist BLOCKING 模式 (1 P0 fail 退出 1)
	@bash $(SCRIPTS_DIR)/release-prep.sh

# ----------------------------------------------------------------------------
# sbom — 生成 CycloneDX 1.5 SBOM (scripts/sbom.sh)
# ----------------------------------------------------------------------------
sbom: ## 生成 CycloneDX 1.5 SBOM
	@bash $(SCRIPTS_DIR)/sbom.sh

# ----------------------------------------------------------------------------
# release-check — audit + sbom (1.0 release 前必跑)
# ----------------------------------------------------------------------------
release-check: audit sbom ## 1.0 release 前必跑 (audit + sbom)
	@echo ""
	@echo "=========================================="
	@echo "  ✅ TP20-S5 release-check 完成"
	@echo "  报告: $(REPORTS_DIR)/tp20-s5-cargo-{vet,audit,deny}-stdout-*.txt"
	@echo "  SBOM:  cyclonedx-sbom.json"
	@echo "=========================================="

# ----------------------------------------------------------------------------
# clean — 清 TP20-S5 产物
# ----------------------------------------------------------------------------
clean-vet: ## 清 reports/ 里 tp20-s5-* 日志
	rm -f $(REPORTS_DIR)/tp20-s5-cargo-vet-stdout-*.txt
	rm -f $(REPORTS_DIR)/tp20-s5-cargo-audit-stdout-*.txt
	rm -f $(REPORTS_DIR)/tp20-s5-cargo-deny-stdout-*.txt
	rm -f audit-report.json audit-audit.stderr.txt

clean-sbom: ## 删 cyclonedx-sbom.json + sbom-cyclonedx.stderr.txt
	rm -f cyclonedx-sbom.json sbom-cyclonedx.stderr.txt

# ----------------------------------------------------------------------------
# 内部约定: 不要在 Makefile 里塞业务逻辑, 业务在 scripts/ 里
# ponytail: 升级路径 — 若需要 release 全自动 (vet+audit+deny+sbom+test+build+release-prep),
# 加一个 `make release-full` target 串 6 个 script, 但**不**替 CI release.yml
# (CI 用 GitHub Actions matrix 更可靠, Makefile 只给本地开发者用)
#
# post-1.0.0 新增: release-prep (8 硬墙 + PII + 12 项 checklist 本地化, commit cf0cafc2)
#
# 2026-11-04 工具链面口径归位批:
#   - 版本轴 (alpha.1 → 2.0.0-rc.5) / crate 数 (18 → 31 members) 向真源对齐
#   - rust.yml 行号引用 (line 63/67 → line 71/75) 重新计数
#   - check 补 --locked; test 补 --no-fail-fast -j 2; 新增 clippy / gate-full 目标
#   - make ci 的覆盖范围改为如实口径 (不再宣称 push 后必绿)
# ============================================================================
