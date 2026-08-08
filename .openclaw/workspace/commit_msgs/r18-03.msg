R18 round-03: miri + coverage + rustdoc + SECURITY + 路线图

# R18 第 3 阶段: 3 个 CI workflow + SECURITY + 路线图文档

抄 wasmtime + qdrant 业界做法:

- **.github/workflows/miri.yml** (1221B): 3 unsafe crate (apeireth-sovereignty / core / memory),
  nightly + miri + `-Zmiri-permissive-provenance`
- **.github/workflows/coverage.yml** (1108B): cargo-tarpaulin → codecov.io, llvm engine
- **codecov.yml** (1097B): 阈值 (架构层 90% / 产品层 60% / 整体 70%) + patch gate 60%
- **.github/workflows/rustdoc.yml** (1111B): nightly cargo doc -Dwarnings --cfg docsrs
- **SECURITY.md** (3494B): 抄 tokio + wasmtime GHSA link + 披露邮箱
- **docs/Apeireth-v2.1-Industry-Top-Backend-Roadmap.md** (42792B): 完整路线图文档

**业界标准命中 +2**: #8 miri / #9 coverage

**CI workflows 总数**: 8 个 (cargo-deny / coverage / kani / miri / protocol-e2e / rust-ci / rust-lint / rustdoc)

**业界标准 9/9 全部达标 ✅**

未在本 commit: rust-ci.yml 重写 + nextest.toml 是其他 AI / 其他 round 做的
