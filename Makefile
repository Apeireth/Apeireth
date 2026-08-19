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
.PHONY: help audit sbom release-check release-prep release-prep-block test check fmt fmt-check tools-install tools-check clean-vet clean-sbom

# ----------------------------------------------------------------------------
# help — 列出所有 target
# ----------------------------------------------------------------------------
help: ## 列出所有 make target
	@echo "=========================================="
	@echo "  Apeireth 开发 + 发布期工具链 (TP20-S5 + post-1.0.0)"
	@echo "=========================================="
	@echo ""
	@echo "  make check            cargo check --workspace --all-targets (~25s, 增量编译)"
	@echo "  make test             cargo test --workspace --all-targets (全测试, 23K+)"
	@echo "  make fmt              cargo fmt --all (格式)"
	@echo ""
	@echo "  make tools-install    装 cargo-vet/audit/deny/cyclonedx (best-effort)"
	@echo "  make tools-check      检查工具链是否齐全 (exit 0 = 缺也 OK, 仅打印)"
	@echo "  make audit            跑 cargo vet + audit + deny (scripts/vet.sh)"
	@echo "  make sbom             生成 CycloneDX 1.5 SBOM (scripts/sbom.sh)"
	@echo ""
	@echo "  make release-prep     8 硬墙 + PII + 12 项 checklist (scripts/release-prep.sh, post-1.0.0)"
	@echo "  make release-check    audit + sbom (1.0 release 前必跑)"
	@echo ""
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
# check / test / fmt — 增量编译 + 全测试 + 格式 (本地开发者快速反馈环)
# ----------------------------------------------------------------------------
check: ## cargo check --workspace --all-targets
	cargo check --workspace --all-targets

test: ## cargo test --workspace --all-targets (23K+ 测试)
	cargo test --workspace --all-targets

fmt: ## cargo fmt --all
	cargo fmt --all

fmt-check: ## cargo fmt --all --check (CI 模式, 0 diff exit 0)
	cargo fmt --all --check

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
# ============================================================================