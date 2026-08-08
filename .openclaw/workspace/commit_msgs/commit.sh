#!/bin/bash
# Run from promethean/ root (not Apeireth-rust/)
set -e

cd ".openclaw/workspace/promethean"

# === R18 round-00: workspace.lints + deny + rustfmt + clippy ===
# ⚠️ Cargo.toml 包含其他 AI 加的 shell-words (Tech-Review 2026-08-05), 一起 commit
git add Apeireth-rust/Cargo.toml Apeireth-rust/rustfmt.toml Apeireth-rust/clippy.toml Apeireth-rust/deny.toml
git commit -F ".openclaw/workspace/commit_msgs/r18-00.msg"

# === R18 round-01: cargo-deny + rust-lint CI workflows ===
git add Apeireth-rust/.github/workflows/cargo-deny.yml Apeireth-rust/.github/workflows/rust-lint.yml
git commit -F ".openclaw/workspace/commit_msgs/r18-01.msg"

# === R18 round-02: 12 product crate integration tests ===
# tests/ 在 gitignore, 用 -f 强制加
git add -f Apeireth-rust/crates/apeireth-protocol/tests/wire_format.rs
git add -f Apeireth-rust/crates/apeireth-api/tests/endpoints.rs
git add -f Apeireth-rust/crates/apeireth-tools/tests/e2e.rs
git add -f Apeireth-rust/crates/apeireth-tool-registry/tests/registry.rs
git add -f Apeireth-rust/crates/apeireth-tool-runtime/tests/parser.rs
git add -f Apeireth-rust/crates/apeireth-tool-approval/tests/rules.rs
git add -f Apeireth-rust/crates/apeireth-pipeline/tests/pipeline.rs
git add -f Apeireth-rust/crates/apeireth-agent/tests/agent.rs
git add -f Apeireth-rust/crates/apeireth-mcp/tests/conformance.rs
git add -f Apeireth-rust/crates/apeireth-memory/tests/sqlite.rs
git add -f Apeireth-rust/crates/apeireth-vector/tests/store.rs
git add -f Apeireth-rust/crates/apeireth-web/tests/templates.rs
git add Apeireth-rust/crates/apeireth-mcp/Cargo.toml Apeireth-rust/crates/apeireth-memory/Cargo.toml Apeireth-rust/crates/apeireth-vector/Cargo.toml
git commit -F ".openclaw/workspace/commit_msgs/r18-02.msg"

# === R18 round-03: miri + coverage + rustdoc + SECURITY + 路线图 ===
git add Apeireth-rust/.github/workflows/miri.yml Apeireth-rust/.github/workflows/coverage.yml Apeireth-rust/.github/workflows/rustdoc.yml Apeireth-rust/SECURITY.md Apeireth-rust/codecov.yml Apeireth-rust/docs/Apeireth-v2.1-Industry-Top-Backend-Roadmap.md
git commit -F ".openclaw/workspace/commit_msgs/r18-03.msg"

# === R19 round-10: 34 lib.rs 删 #![warn(missing_docs)] ===
git add Apeireth-rust/crates/apeireth-action/src/lib.rs
git add Apeireth-rust/crates/apeireth-asi/src/lib.rs
git add Apeireth-rust/crates/apeireth-bench/src/lib.rs
git add Apeireth-rust/crates/apeireth-bus/src/lib.rs
git add Apeireth-rust/crates/apeireth-central/src/lib.rs
git add Apeireth-rust/crates/apeireth-cli/src/lib.rs
git add Apeireth-rust/crates/apeireth-cognition/src/lib.rs
git add Apeireth-rust/crates/apeireth-consciousness/src/lib.rs
git add Apeireth-rust/crates/apeireth-constraint/src/lib.rs
git add Apeireth-rust/crates/apeireth-core/src/lib.rs
git add Apeireth-rust/crates/apeireth-council/src/lib.rs
git add Apeireth-rust/crates/apeireth-evolution/src/lib.rs
git add Apeireth-rust/crates/apeireth-extension/src/lib.rs
git add Apeireth-rust/crates/apeireth-formal/src/lib.rs
git add Apeireth-rust/crates/apeireth-http-client/src/lib.rs
git add Apeireth-rust/crates/apeireth-life-force/src/lib.rs
git add Apeireth-rust/crates/apeireth-mcp/src/lib.rs
git add Apeireth-rust/crates/apeireth-memory/src/lib.rs
git add Apeireth-rust/crates/apeireth-motivation/src/lib.rs
git add Apeireth-rust/crates/apeireth-onion/src/lib.rs
git add Apeireth-rust/crates/apeireth-perception/src/lib.rs
git add Apeireth-rust/crates/apeireth-pipeline/src/lib.rs
git add Apeireth-rust/crates/apeireth-protocol/src/lib.rs
git add Apeireth-rust/crates/apeireth-pybridge/src/lib.rs
git add Apeireth-rust/crates/apeireth-relation/src/lib.rs
git add Apeireth-rust/crates/apeireth-sdk/src/lib.rs
git add Apeireth-rust/crates/apeireth-sovereignty/src/lib.rs
git add Apeireth-rust/crates/apeireth-tool-approval/src/lib.rs
git add Apeireth-rust/crates/apeireth-tool-registry/src/lib.rs
git add Apeireth-rust/crates/apeireth-tool-runtime/src/lib.rs
git add Apeireth-rust/crates/apeireth-upgrade/src/lib.rs
git add Apeireth-rust/crates/apeireth-value/src/lib.rs
git add Apeireth-rust/crates/apeireth-vector/src/lib.rs
git add Apeireth-rust/crates/apeireth-verify/src/lib.rs
git commit -F ".openclaw/workspace/commit_msgs/r19-t10.msg"

echo "=== ALL 5 COMMITS DONE ==="
git log --oneline -5
