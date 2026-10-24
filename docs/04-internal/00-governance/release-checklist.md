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

> **版本一致性门禁现状（2026-10-22 已修）**：2026-10-21 对账曾实测 **exit 1**——脚本递归扫
> `crates/**/Cargo.toml`，把两个独立 fuzz 子工程（`crates/engine/image/codecs/fuzz/Cargo.toml`、
> `crates/engine/image/psd/fuzz/Cargo.toml`；各自声明 `[workspace]`、fuzz 惯例 `version = "0.0.0"`）
> 判为「显式版本 ≠ workspace 2.0.0-rc.4」。**已修**：脚本排除 `*/fuzz/Cargo.toml` 并在报告里显式列
> `[SKIP]`（不静默），复跑 **exit 0**（`-Json`：passed=true，35 条 = 33 检查 + 2 SKIP，FAIL 0；
> `-ExpectedVersion 2.0.0-rc.4 -Strict` exit 0；`-ExpectedVersion 9.9.9` 反证仍 exit 1）。
> `.github/workflows/publish-release.yml` 的「Verify tag matches version sources」首关本地已绿；
> 远程 CI 首跑仍未验证（无 tag 推送权限动作）。见 §5 R-1（已收敛）。

> **发布就绪状态（2026-10-22 收口）**：仓库侧发布门禁已全绿——R-1（版本门禁）exit 0、
> R-2（发布说明模板口径+未签名声明）、R-3（`SHA256SUMS` 命名）、R-4（30 members / 111 edges）、
> R-5（测试数字二源指针）、R-6（`cargo run -- --version`）全部收敛或模板侧收敛；
> `docs_truth.py --verify --check` / `arch_guard.py --strict` / `arch_doc.py --check` 均绿。
> **剩余只等三件 owner/发布线动作（仓库侧无待办）**：① `push`（本地跨线提交未推；推后远程 CI
> 首跑才发生，其记录是本清单 L31/§4 的最后一格证据）；② 已发布 `v2.0.0-rc.4` Release 页正文/
> 资产清单按新模板编辑或重发（本机无 gh / Release 面通道）；③ Authenticode 签名证书（外部持有；
> 未签名声明已入模板，不阻塞内部测试版）。另：B6-2a 试点设计页
> [b6-2a-sealed-session-design-2026-10-22.md](../backend-enhancement/b6-2a-sealed-session-design-2026-10-22.md)
> 已出，待 owner 拍板三问（试点范围 `sessions.data` / daemon 密钥供给 / KDF+回滚）。
> **push 实测（2026-10-22 09:2x）**：`git push origin main` 被远端**分支保护拒绝**
> （`main` 为 locked branch，required status check `kani` 未满足）⇒ 已把全量提交推到
> 侧分支 **`batch-2026-10-22`**（221 commits；远端已建，fetch 后 `main...HEAD` = 0 behind / 221 ahead）。
> 待 owner 动作：开 PR 合并（一键：`https://github.com/Apeireth/Apeireth/pull/new/batch-2026-10-22`）
> 或临时解锁/绕过 `main` 保护；PR 一开即触发远端 CI（其首跑记录即本清单最后一格证据）。

## 1. 发布物一致性

- [x] `cargo run --bin apeireth -- --version` 输出 = 新版本号
  - 实证（2026-10-21）：原文 `cargo run --bin apeireth --version` 会被 cargo 自身参数解析拒绝
    （实测 `error: unexpected argument '--version' found` + 提示改用 `-- --version`）——已改命令。
    **2026-10-22 补证（cargo 直跑闭环）**：`cargo run -p apeireth-cli --bin apeireth --locked -j 2 -- --version`
    → 编译成功并输出 **`apeireth 2.0.0-rc.4`**，exit 0（此前被在途 cli 编译红阻断，R-6 已收敛）。
- [ ] 安装包文件名内嵌版本 = 新版本号；`Get-AuthenticodeSignature` 状态如实记录（未签名则发布说明写明）
  - 部分实证（2026-10-21）：本地 NSIS 产物 `target/desktop-nsis/Apeireth Companion_2.0.0-rc.4_x64-setup.exe`
    （13,472,128 B，2026-10-17 21:01:50 构建）文件名内嵌 **2.0.0-rc.4** = 当前版本 ✓；
    `Get-AuthenticodeSignature` 实测 **`NotSigned`**（平台原话：文件未进行数字签名）。
    「未签名则发布说明写明」：**2026-10-22 已补进发布说明模板**
    （`.github/workflows/publish-release.yml` 下载段新增 Unsigned build notice：`Get-AuthenticodeSignature`
    = `NotSigned`，提示先核对 SHA-256 或从源码构建）。**未闭环**：已发布的 `v2.0.0-rc.4` Release 页
    正文仍是旧文案（未重发/未编辑）⇒ 该页待发布线处理（见 §5 R-2，模板侧已收敛）。
- [ ] **`SHA256SUMS` 与各资产 `.sha256` 一致**（E-01 前科：两文件互相矛盾——重生成覆盖全部资产，
      保留 per-asset `.sha256`；CI 增下载自校验一步。历史文件名 `SHA256SUMS.txt`，现行流水线产物为
      无后缀 `SHA256SUMS`）
  - 部分实证（2026-10-21，本地 smoke，非 Release 面；复现目录 `%TEMP%\apeireth-relcheck-20261021`）：
    ① `pwsh -File scripts/generate-release-manifest.ps1 -DistDir <临时目录> -Version 2.0.0-rc.4
    -ReleaseTag v2.0.0-rc.4` → exit 0，产出 `SHA256SUMS` + `releaseon`；
    ② `pwsh -File scripts/verify-release-assets.ps1 -ArtifactPath <rc.4 exe> -ManifestPath <SHA256SUMS>`
    → **PASS，exit 0**；
    ③ 同 exe 副本改末字节 1 位后复验 → **FAIL，exit 1**（fail-closed 有据）；
    ④ 本地 rc.4 exe 实际 SHA-256 = `AE144780E1AB1743D5AA66F67B3A4A08620F5DEB6870412C87B6C28B585A0406`
    = 同目录 `.sha256` sidecar 值逐字相同。
    未实测：Release 页 `SHA256SUMS` 与各资产逐件一致（远程 CI 首跑未发生、本机无 Release 面访问通道）；
    「CI 增下载自校验一步」流水线未见（现为上传前校验，`publish-release.yml` L62–L83）⇒ 待发布线。
- [ ] Release 页资产清单与发布指南引用的文件名逐字一致
  - **指南侧已统一（2026-10-22）**：`docs/beta/rc2-beta-guide.md:8` 由 `SHA256SUMS.txt` 改为
    `SHA256SUMS`（= 流水线产物名，`publish-release.yml` L115 / L62–L83 生成与校验同名）；
    安装包文件名两面一致（指南 L7 = 本地产物名）。**仍待核**：已发布 Release 页资产清单逐字核对
    （无 Release 面访问通道）⇒ Release 页侧待发布线（见 §5 R-3，仓库侧已收敛）。

## 2. 文档口径

- [x] 测试基线刷新：跑 `pwsh scripts/gate_full.ps1 -FailIfBusy`（R6-A 唯一全量执行器：进程锁 +
      失败归因；等价于全 workspace 口径），更新 [test-baseline.md](../../03-reference/test-baseline.md)
      （单一真源）§1 与「历史口径」表；**其余文档只引用不抄写**
  - 实证（2026-10-21，复核记录，未重跑）：test-baseline §1 = **341 套件 / 8570 passed / 0 failed /
    51 ignored**（日期 2026-10-21；工具链 rustc/cargo 1.97.1）；§3「历史口径」表已含 2026-10-21
    现行行与 2026-10-20 旧行（336 套件 / 9075 passed / 1 failed）；`logs/gate-full.log` 尾部
    `GATE FULL PASSED (2026-10-21 15:52:59)`，`artifacts/wave4-close-full-2026-10-21.log`
    尾部 `GATE FULL PASSED`。全量执行权 2026-10-21 起在效率线；本轮未重跑（见 §3.1）。
- [x] crate 口径核对：`workspace = 30 members（29 crates + bench harness）`（2026-10-20 R4b 归档
      `apeireth-perception` 后；历史 31 members）；成员有变时先改 ARCHITECTURE.md / INSTALL.md /
      README 本体，再全仓 grep 旧口径
  - 真源实证（2026-10-21）：`python -c "import tomllib; ..."` → **30 members**；版本 → `2.0.0-rc.4`。
    **2026-10-22 旧口径清零（现值文档）**：`README.md:13` Workspace 徽章 31 → **30 members**；
    `README.zh-CN.md:13` 徽章同步、正文「31 成员工作区」→ **30 成员工作区**；
    `ARCHITECTURE.md:56`「2026-10-20：31 members / 118 production edges」→ **「2026-10-22：30 members /
    111 production edges」**（生成物 `ARCHITECTURE.generated.md:7` 同值）；全仓 grep 现值文档零命中。
    带日期历史标注的命中（`SECURITY.md` 2026-10-18 行、user-manual 注记、team-handover 2026-10-18 口径、
    CHANGELOG 历史）按「历史不追改」保留（见 §5 R-4，已收敛）。
- [x] 徽章数字与链接目标对齐（Tests 徽章 → capabilities-matrix §0.1 有据）
  - 半实证（2026-10-21）：`python scripts/docs_truth.py --verify --check` →
    `OK: STATUS.md matches claims.yaml; README badges match baseline`，exit 0；`README.md:12` /
    `README.zh-CN.md:12` Tests 徽章 = `tests-8570 passed | 0 failed (2026-10-21)`，与
    test-baseline §1 对齐 ✓。但两项未闭环：① README 注记（`README.md:22` / `README.zh-CN.md:21`）
    称证据面在 capabilities-matrix §0.1，该表仍 **336 套件 / 9075 passed / 52 ignored
    （2026-10-20 行）**，未跟 2026-10-21 基线（§0.1 属他线真源，本清单不改）；
    ② Workspace 徽章 31 members ≠ 30（见上条）⇒ 待他线同步后勾（见 §5 R-5）。
    **2026-10-22 已闭环**：① README 注记日期改为 2026-10-21，matrix §0.1 行加现行指针
    （「历史口径；现行 §1 = 341/8570/0/51（2026-10-21）」）；② Workspace 徽章已改 30
    ⇒ 两项闭环，`docs_truth --verify --check` 仍 exit 0（R-5 已收敛）。
- [x] 未接线能力的标注仍在（WebSocket 服务端 / DSL 洋葱 / SDK STUB_MODE 等，见
      [capabilities-matrix.md](../../03-reference/capabilities-matrix.md) §0.2）
  - 实证（2026-10-21）：capabilities-matrix §0.2「未接线（不是默认开关问题）」行（L76）列
    第三洋葱 DSL 层 `DslOnionLayer` / 双洋葱 env / `memory.update`（`reason=not_implemented`）/
    WebSocket 服务端；§1 `gw.duplex_ws` 行（L119）「HTTP 服务端未接线」、L211 同口径。
    代码标记在场：`crates/adapters/sdk/src/client.rs:112` `pub const STUB_MODE: bool = true;`
    （编译期守门 L129–L131）；`DslOnionLayer` 在 `crates/foundation/governance/src/colang.rs`。

## 3. 校验命令（全量门禁 / 快检口径）

```powershell
pwsh scripts/gate_full.ps1 -FailIfBusy                             # 全量单一执行器（R6-A；main + 图像家族；exit 0 = 绿）
cargo clippy --workspace --all-targets --locked -- -D warnings     # 快检：0 warnings（-j 2 口径）
cargo fmt --check                                                   # 快检：0 diff
```

### 3.1 本轮对账注记（2026-10-21）

- 本轮只做文档级实证对账，**未重跑** §3 全量门禁：全量执行权 2026-10-21 起在效率线，且他线在途改
  `crates/adapters/cli` 会引发编译竞争；全量结果只引单一真源
  [test-baseline.md](../../03-reference/test-baseline.md) §1（2026-10-21）。
- §3 命令文本本身未变；版本一致性门禁脚本现状与阻塞见 §0 注记 / §5 R-1。

## 4. 发布后

- [ ] git tag 与 Release 页互相指向；workspace.version 随 release 推进（rc.n → GA）
  - 半实证（2026-10-21）：tag 侧 ✓ —— `git tag --list` 有 `v2.0.0-rc.4`；`git show
    v2.0.0-rc.4:Cargo.toml` 的 `[workspace.package] version = "2.0.0-rc.4"` = 根 `Cargo.toml` 现值；
    `git for-each-ref` 显示该 tag 指向 commit `2a8b46fe`（轻量 tag；当前 HEAD `2ce83d45`）。
    Release 页侧**未实测**：本机无 `gh` CLI；按仓内 `docs/04-internal/github-fetch-guide.md` §〇，
    GitHub 域 `web_fetch` 定死 `fetch failed`，无可用通道核对「页面互相指向」
    ⇒ 待发布线（见 §5 E-11）。
- [x] 历史文档不追溯改写；新口径在「当前基线」类文档（INSTALL/docs README/ROADMAP §3）落一处即可
  - 实证（2026-10-21）：test-baseline §3「历史口径（带日期保留，不追溯改写）」表保留 2026-10-20 旧行
    （336 套件 / 9075 passed / 1 failed）与 2026-10-21 现行行；`CHANGELOG.md:197` 保留许可搬运线当期
    「31 members」历史口径；`INSTALL.md:334` / `SECURITY.md:30` / `AGENTS.md:26` 均以「历史 31 members
    见 CHANGELOG」保留旧值。新口径单点：test-baseline §1（其余文档只引）。

_本清单由「落差清账·文本可信度批」创建（2026-10-16）。_

## 5. 本轮实证记录（2026-10-21）

> 对账环境：仓库 `C:\Users\31683\Apeireth-rust`，`main` HEAD `2ce83d45`，日期 2026-10-21
> （Asia/Shanghai）。工作树含他线在途改动（`crates/adapters/cli/src/lib.rs` 等，非本线）；
> 本轮只改本文件。工具链：rustc 1.97.1 (8bab26f4f 2026-07-14) / cargo 1.97.1 (c980f4866 2026-06-30)
> （同 test-baseline §1）。**勾选结果：4 勾 / 6 未勾**（未勾项均有实证与归口，无 vibe 勾选）。

| # | 检查 | 命令（可复核） | 2026-10-21 实测结果 |
|---|---|---|---|
| E-1 | 二进制版本输出 | `target\debug\apeireth.exe --version` | `apeireth 2.0.0-rc.4`，exit 0 |
| E-2 | 安装包文件名 | `Get-ChildItem target\desktop-nsis` | `Apeireth Companion_2.0.0-rc.4_x64-setup.exe`（13,472,128 B） |
| E-3 | Authenticode 状态 | `Get-AuthenticodeSignature '<exe>'` | `Status = NotSigned` |
| E-4 | sidecar 哈希一致 | `Get-FileHash '<exe>' -Algorithm SHA256` | `AE1447…A0406` = 同目录 `.sha256` 值 |
| E-5 | 清单生成 | `scripts\generate-release-manifest.ps1 -DistDir <temp> -Version 2.0.0-rc.4 -ReleaseTag v2.0.0-rc.4` | exit 0，产出 `SHA256SUMS` + `releaseon` |
| E-6 | 资产校验正路 | `scripts\verify-release-assets.ps1 -ArtifactPath <exe> -ManifestPath <SHA256SUMS>` | `PASS`，exit 0 |
| E-7 | 资产校验反路 | 同上，对篡改末字节的 exe 副本 | `FAIL`，exit 1（fail-closed） |
| E-8 | 版本一致性门禁脚本 | `pwsh -NoProfile -File scripts/check-release-version.ps1`；`-Json`；`-ExpectedVersion 9.9.9` | **2026-10-22 修复后 exit 0**：PASS 33 行 + `[SKIP]` 2 行（fuzz 子工程），JSON passed=true；反证 9.9.9 → exit 1（见 R-1 已收敛） |
| E-9 | 文档真源门禁 | `python scripts/docs_truth.py --verify --check` | `OK: STATUS.md matches claims.yaml; README badges match baseline`，exit 0 |
| E-10 | crate 口径 | `python -c "import tomllib; ... print(len(d['workspace']['members']))"` | `30`（版本 `2.0.0-rc.4`） |
| E-11 | tag ↔ 版本 | `git tag --list`；`git show v2.0.0-rc.4:Cargo.toml` | tag `v2.0.0-rc.4` 存在（→ commit `2a8b46fe`），version = `2.0.0-rc.4` |
| E-12 | 全量门禁记录（只复核） | `logs/gate-full.log`；`artifacts/wave4-close-full-2026-10-21.log` 尾部 | `GATE FULL PASSED (2026-10-21 15:52:59)`；`GATE FULL PASSED` |

E-5/E-6/E-7 在 `%TEMP%\apeireth-relcheck-20261021` 复现（临时目录，不落仓库）；E-7 篡改方式 =
读 exe 字节数组后 `d[-1] ^= 0xFF` 写回副本。

**未实测 / 待发布线（不勾项归口）**：L29 的 cargo 直跑路径（他线在途编译红，R-6）；L30 发布说明
未签名声明（R-2）；L31 Release 页 `SHA256SUMS` ↔ 资产一致性与「下载自校验」（需远程 CI 首跑 +
Release 面通道）；L33 资产名逐字一致（R-3）；L40–L42 旧口径清零（R-4）；L43 徽章证据面同步（R-5）；
L57 tag ↔ Release 页互相指向（只有 tag 侧，E-11）。

**残余风险（按阻塞度排序；均不在本文件处置范围，交发布线/owner/他线）**：

- **R-1（已收敛，2026-10-22）** 版本一致性门禁红：`scripts/check-release-version.ps1` 曾把
  `crates/engine/image/codecs/fuzz/Cargo.toml` 与 `crates/engine/image/psd/fuzz/Cargo.toml`
  （独立 `[workspace]`、fuzz 惯例 `version = "0.0.0"`）判 FAIL ⇒ exit 1（PASS 33 / FAIL 2）。
  **已修**：脚本按 `*/fuzz/Cargo.toml` 排除并登记 `[SKIP]`（0 装：跳过要写出来），复跑 exit 0；
  `-Json` passed=true（33 检查 + 2 SKIP / FAIL 0）；`-Strict` exit 0；`-ExpectedVersion 9.9.9` 反证 exit 1
  （判定仍有效）。`.github/workflows/publish-release.yml` 首关本地已绿；**远程 CI 首跑未验证**。
- **R-2（模板侧已收敛，2026-10-22）** 发布说明口径滞后：`.github/workflows/publish-release.yml`
  正文曾写「18-Crate Kernel + Assembly, 3600+ Verified Tests」（旧口径）——**已改**为
  「30-Member Workspace (29 crates + 1 bench harness), 8570+ Verified Tests (2026-10-21)」；
  下载段**新增 Unsigned build notice**（NotSigned + 先核对 SHA-256/从源码构建提示）。
  残留：已发布的 `v2.0.0-rc.4` Release 页正文仍是旧文案（本机无 gh/Release 面通道）⇒ 待发布线编辑/重发。
- **R-3（已收敛，2026-10-22）** 资产名漂移：指南 `docs/beta/rc2-beta-guide.md:8` 引
  `SHA256SUMS.txt` —— **已统一**为 `SHA256SUMS`（= 流水线产物名）；仓库侧零漂移。
  残留：已发布 Release 页资产清单需发布线核对（无 Release 面通道）。
- **R-4（已收敛，2026-10-22）** crate 口径残留：`README.md:13` Workspace 徽章、`README.zh-CN.md:13`
  徽章与正文、`ARCHITECTURE.md:56`（31 members / 118 edges）**已全部清零**为
  **30 members / 111 production edges**（与 `ARCHITECTURE.generated.md:7` 一致）；历史带日期标注按
  「历史不追改」保留。
- **R-5（已收敛，2026-10-22）** 测试数字二源漂移：`capabilities-matrix.md` §0.1 保留 336/9075/52
  的历史实测行（带日期），**已加现行指针**「现行 §1 = 341 套件 / 8570 passed / 0 failed / 51 ignored
  （2026-10-21）」；README 注记日期改为 2026-10-21。真源（test-baseline §1）未动，`docs_truth` exit 0。
- **R-6（已收敛，2026-10-22）** L29 的 `cargo run` 直跑路径曾被他线在途编译红阻断（E0308，
  `GovernanceChain::new` 期望 `Arc<GovernanceChain>`）；安全线 R8-B2/B3 收口（`4e380a69`/`e812fda9`）
  后 **已复跑**：`cargo run -p apeireth-cli --bin apeireth --locked -j 2 -- --version` →
  编译成功、输出 `apeireth 2.0.0-rc.4`、exit 0。
