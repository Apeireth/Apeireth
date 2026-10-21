# 发版清单（Release Checklist）

> **用途**：每次 rc/GA 发版前逐项勾选。**性质**：文本可信度批新建（审计修 B-03 / F-04），
> 目标是消除「版本号残留 / 发布物与文档口径漂移」类落差。
> 版本轴现值以根 `Cargo.toml` `[workspace.package] version` 为准（现 **2.0.0-rc.4**）。

## 0. 版本号残留清扫（必做，本清单的核心）

1. **grep 上一版号**（把 `<上一版>` 换成被替换的版本串，例如 `2.0.0-rc.3`）：

   ```powershell
   # 全仓扫描（含隐藏目录除外的全部文本文件）
   rg -n "2\.0\.0-rc\.<上一版>" --glob '!legacy/**' --glob '!target/**'
   ```

   覆盖面至少包含：`INSTALL.md` / `Dockerfile` / `packaging/**` / `docker-compose.yml` /
   `ROADMAP.md` / `deploy/` / `docs/beta/**`（发布指南）/ `frontend/**/README.md` / 发布说明正文。
2. **逐命中判定**：现值行 → 改为新版本；带日期的历史记录（CHANGELOG、reports/、
   docs/04-internal 历史卷宗、tag 名、git 记录引用）→ **保留原值**，必要时补「历史口径（日期）」标注。
3. **grep 新版号**复核已改齐：

   ```powershell
   rg -n "2\.0\.0-rc\.<新版>" INSTALL.md packaging docker-compose.yml ROADMAP.md docs/beta
   ```
4. 发布指南（`docs/beta/*`）标题、安装包文件名、日志路径、服务商清单逐一对照发布物实况。

## 1. 发布物一致性

- [ ] `cargo run --bin apeireth --version` 输出 = 新版本号
- [ ] 安装包文件名内嵌版本 = 新版本号；`Get-AuthenticodeSignature` 状态如实记录（未签名则发布说明写明）
- [ ] **SHA256SUMS.txt 与各资产 `.sha256` 一致**（E-01 前科：两文件互相矛盾——重生成覆盖全部资产，
      保留 per-asset `.sha256`；CI 增下载自校验一步）
- [ ] Release 页资产清单与发布指南引用的文件名逐字一致

## 2. 文档口径

- [ ] 测试基线刷新：跑 `pwsh scripts/gate_full.ps1 -FailIfBusy`（R6-A 唯一全量执行器：进程锁 +
      失败归因；等价于全 workspace 口径），更新 [docs/03-reference/test-baseline.md](../03-reference/test-baseline.md)
      （单一真源）§1 与「历史口径」表；**其余文档只引用不抄写**
- [ ] crate 口径核对：`workspace = 30 members（29 crates + bench harness）`（2026-10-20 R4b 归档
      `apeireth-perception` 后；历史 31 members）；成员有变时先改 ARCHITECTURE.md / INSTALL.md /
      README 本体，再全仓 grep 旧口径
- [ ] 徽章数字与链接目标对齐（Tests 徽章 → capabilities-matrix §0.1 有据）
- [ ] 未接线能力的标注仍在（WebSocket 服务端 / DSL 洋葱 / SDK STUB_MODE 等，见
      [capabilities-matrix.md](../03-reference/capabilities-matrix.md) §0.2）

## 3. 校验命令（全量门禁 / 快检口径）

```powershell
pwsh scripts/gate_full.ps1 -FailIfBusy                             # 全量单一执行器（R6-A；main + 图像家族；exit 0 = 绿）
cargo clippy --workspace --all-targets --locked -- -D warnings     # 快检：0 warnings（-j 2 口径）
cargo fmt --check                                                   # 快检：0 diff
```

## 4. 发布后

- [ ] git tag 与 Release 页互相指向；workspace.version 随 release 推进（rc.n → GA）
- [ ] 历史文档不追溯改写；新口径在「当前基线」类文档（INSTALL/docs README/ROADMAP §3）落一处即可

_本清单由「落差清账·文本可信度批」创建（2026-10-16）。_
