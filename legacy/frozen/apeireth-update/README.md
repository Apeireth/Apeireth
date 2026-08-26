# apeireth-update

Apeireth **autoupdate skeleton** — 借鉴 Golutra P3 minisign 签名 + autoupdate endpoint
(per [`analysis/golututra/BORROW_FROM_GOLUTRA.md` §8 P3 第 10-11 项](../../../../../../..analysis/golututra/BORROW_FROM_GOLUTRA.md))
1:1 翻译, 纯 Rust, 0 网络依赖, 用现成 [`minisign`](https://crates.io/crates/minisign) crate 验签.

⚠️ **STUB MODE (R21 autoupdate 估补)**: 当前 crate 是 **skeleton**. **0 真连 GitHub Releases**,
**0 真下载 asset**, **0 真应用更新**. 所有外部交互 (GitHub API / 真实 minisign 验签 /
真解压 / 真切换版本) 都是 stub. R21+ 续真接.

## 状态

| 组件 | 编译期常量 | 状态 |
|------|----------|------|
| `Updater` trait (3 方法) | 3 method 固定 (check/apply/verify) | ✅ trait + async-trait |
| `DefaultUpdater` impl | 1 默认实现 | ✅ GitHub Releases check stub + minisign verify |
| minisign 验签 | `SignatureAlgorithm::COUNT = 1` (Ed25519) / `TrustedKey::COUNT = 3` | ✅ 用现成 `minisign` crate, 0 重复造轮子 |
| `Channel` 枚举 | `Channel::COUNT = 3` (Stable/Beta/Nightly) | ✅ 编译期 hardcode |
| `Asset` + `Release` + `UpdateInfo` | 5 K-1 强校验 | ✅ SHA-256 + size + signature + algorithm + version |
| HTTP endpoint stub | `ENDPOINT_COUNT = 2` | ✅ GET /v1/update/check + POST /v1/update/apply |
| 错误类型 | `UPDATE_ERROR_VARIANT_COUNT = 11` | ✅ 11 thiserror variant + 5 K-1 校验函数 |
| 公开 API 100% 文档化 | — | ✅ 全部 doc comment |
| 1 update 流程例子 | `examples/update_check_demo.rs` | ✅ 8 步演示 |
| 集成测试 | 28 集成测试 | ✅ mock GitHub + minisign 真签真验 + endpoint contract |

## 借鉴文档

`analysis/golututu/BORROW_FROM_GOLUTRA.md` §8 P3 第 10-11 项 (minisign 签名 + autoupdate endpoint 借鉴决策).

## minisign 借鉴

- **Golutra 公钥**: `99F790EC4BE6E38D` (per 借鉴文档 §8 P3 第 10 项)
- **协议**: `/api/desktop-updater/check?current_version=` (per 借鉴文档 §8 P3 第 11 项)
- **Apeireth 翻译**:
  - `TrustedKey::TestFixture` 用 `99F790EC4BE6E38D` 占位 (R21+ 真接时换 Apeireth 真公钥)
  - `GET /v1/update/check?current_version={semver}&channel={stable|beta|nightly}` (Apeireth v1 路径)
  - `POST /v1/update/apply` (Apeireth v1 新增, Golutra 无 apply endpoint)
- **0 重复造轮子**: 用现成 [`minisign`](https://crates.io/crates/minisign) crate
  (jedisct1/rust-minisign, 0.9.x), 不自造 Ed25519 / SHA-512 / scrypt

## 6 哲学 anchor

- **S-1 北极星 (走在前人经验上)**: 1:1 翻译 Golutra P3 minisign + autoupdate 协议 + 用现成 `minisign` crate
- **S-2 实事求是 (不假装)**: 当前 100% stub, 不假装已对接 GitHub / 真实验签 / 真下载
- **O-2 走在前人肩上 (用户看结果不看哲学)**: 用户只关心 "有更新吗 + 装得上吗"
- **O-3 干到底 (信息密度"高")**: 5 模块各 1 表 + 23 K-1 强校验守门
- **O-4 任何人都能接手 (干净状态)**: 跟 `apeireth-oauth` / `cache` / `metrics` 同骨架
- **O-5 不假装 (6 哲学 anchor 穿透)**: 所有 stub 返 `warn!` 日志 + 标 ⏳ R21+

## 8 项不修改承诺

1. 0 触碰 24 LOCKED crate (per `scripts/audit/8-promise-audit.sh` LOCKED_CRATES 24 个全过)
2. 0 改 workspace Cargo.toml 其他字段 (只 + 1 个 members 行, version 0 改)
3. 0 引 pyo3 / qt / GDI (纯 Rust + 1 个 minisign 外部 crate)
4. 0 改 K-1 强校验守门 (5 + 5 + 5 + 4 + 4 = 23 K-1 校验必保留)
5. 0 改 3 Channel 枚举顺序 (Stable → Beta → Nightly)
6. 0 改 3 TrustedKey 枚举顺序 (TestFixture → Stable → Beta)
7. 0 改 1 SignatureAlgorithm 枚举 (Ed25519)
8. 0 重复造 minisign 验签轮子 (用现成 `minisign` crate)

## 跑例子

```bash
cargo run -p apeireth-update --example update_check_demo
```

## 跑测试

```bash
cargo test -p apeireth-update
```

## 状态: ⏳ R21 skeleton (估 2026-Q4 真接)

- ⏳ R21+ 续真接: GitHub API / 真实 minisign 验签 / 真下载 / 真应用 (跟 `apeireth-upgrade` 7 阶段 OTA 集成)
