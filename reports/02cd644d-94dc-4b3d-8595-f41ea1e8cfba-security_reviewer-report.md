# 自检-SEC1: 依赖合规与供应链（安全审查）

- 任务ID: 02cd644d-94dc-4b3d-8595-f41ea1e8cfba
- 角色: security_reviewer ｜ 日期: 2025-08-11 ｜ 类型: 只读自检

## 结论: ✅ 通过（含 3 项低危清理建议）

## 1. deny.toml 配置审查
- 文件存在: `deny.toml`（215 行），配置完整且策略严格。
- [licenses]: allowlist 白名单模式，仅允许 17 种许可证（MIT/Apache-2.0/BSD 系等），
  每项附用途注释（如 Artistic-2.0/CC0-1.0 仅因 notify v5 组合）。无 copyleft 放行。
- [bans]: `multiple-versions = "deny"`（强制单版本）；`wildcards = "allow"`；
  黑名单为空；skip 列表含 Windows/proc-macro 类豁免并附 reason。
- [sources]: `unknown-registry = "deny"`、`unknown-git = "deny"`，
  仅允许 crates.io 官方 registry，git 依赖白名单为空 → 供应链来源收敛良好。
- [advisories]: RustSec db 已配置；`ignore` 列表为空（0 项豁免），有验证记录注释。

## 2. 运行时检查（cargo-deny 0.20.2，本机已安装，无需安装）
- 命令: `cargo deny check bans licenses`
- 结果: **bans ok, licenses ok**，退出码 0。

## 3. 发现的问题（均为低危，不阻断）
1. ⚠️ `unnecessary-skip`: 若干 skip 项对应 crate 当前仅单版本（如 `heck`），配置已过时。
2. ⚠️ `unmatched-skip`: skip 中 `async-channel` 等 crate 已不在依赖图中。
3. ⚠️ 上述过期 skip 不影响安全，但会掩盖未来真实的多版本问题，建议清理。

## 4. 建议（可延后，非阻断）
- 删除 deny.toml 中过期 skip 条目（heck、async-channel 等），保持 skip 与实际依赖同步。
- 保持 CI 中 cargo-deny advisories 检查（本次按任务范围仅跑 bans+licenses，
  advisories 由 CI workflow 覆盖，配置上无豁免项）。

## 判定
供应链治理配置健全：许可证白名单 + 来源锁定 + 多版本禁用 + 零漏洞豁免。
✅ 自检通过，无阻断性供应链风险。
