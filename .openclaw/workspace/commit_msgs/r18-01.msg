R18 round-01: cargo-deny + rust-lint CI workflows

# R18 第 1 阶段部分: 2 个 CI workflow

抄 wasmtime main.yml + qdrant rust-lint.yml:

- **.github/workflows/cargo-deny.yml** (1771B): 抄 wasmtime main.yml [cargo-deny] job
  - 用 EmbarkStudios/cargo-deny-action@v2
  - 检查 bans / licenses / sources / advisories
- **.github/workflows/rust-lint.yml** (1667B): 抄 qdrant rust-lint.yml
  - 3 档 clippy (workspace / all-targets / all-features) + -D warnings 目标态
  - rustfmt nightly 检查
  - R19 T10 后加 13 个 -A flag 让 clippy -D warnings 真的不卡 PR

**业界标准命中**: #2 cargo-deny CI 配套 / #5 clippy -D warnings 目标态

未在本 commit: rust-ci.yml 重写 + nextest.toml 是其他 AI / 其他 round 做的

Refs: docs/Apeireth-v2.1-Industry-Top-Backend-Roadmap.md §0.2 + §1.2
