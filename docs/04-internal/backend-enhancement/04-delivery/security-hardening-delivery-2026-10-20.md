# 安全加固线交付报告（B1-B8 · 8 项外部审计隐患收口 · 2026-10-19/20）

```text
[Document-Meta]
Document:        security-hardening-delivery-2026-10-20.md
Version:         1.0（B1-B8 全收口）
Last-Modified:   2026-10-20
Status:          🟢 交付记录
Author:          安全加固线 lead（本线主代理）
```

> 来源：owner 提供《Apeireth-安全隐患分析.md》（外部安全审计，8 项风险）。
> 处置：8 项全部修复入库（B1-B4 于 2026-10-19，B5-B8 于 2026-10-20）。
> 与生物机制线（W3.3）并行施工，同文件 hunk 级分账，零裹挟。
> **全 commit 链（10）**：8 修复（`8f71d0c0`/`06246661`/`a786b733`/`2aab3add`/`1574335f`/`333c3471`/`1f65852b`/`1213fbdd`）+ 交付记录（`e3c3a463`）+ 索引收工（`5f52813d`）。
> 索引入口：[README.md](../../README.md) Index 表（交付报告/治理演练/发布校验三行）。

## 一、8 项修复对照表

| # | 审计风险 | 批次 | Commit | 修法 | 残余（如实） |
|---|---|---|---|---|---|
| 1 | SQL 只读限制可被 WITH DML 绕过（高危） | B1 | `8f71d0c0` | 词法级只读闸（fail-closed）+ PG/MySQL 服务端只读事务双保险 | 保守启发式可能误拒冷门合法 SQL（fail-closed 方向） |
| 2 | 桌面终端权限高于「shell 已关闭」印象 | B4 | `2aab3add` | UI 权限边界明示 + 三层权限面文档 + 文案测试钉子 | 超时树杀待接（既有债，文案如实标注） |
| 3 | Provider 中继允许任意主机（SSRF 面） | B2 | `06246661` | 目标类闸 + DNS 钉址 + 重定向逐跳闸 + 头白名单 | 回环类级口子待「本地端点注册面」；公网 allowlist 待续 |
| 4 | 文件操作不限定工作区 | B3 | `a786b733` | 登记根派生 + canonicalize 祖先校验（junction 实测拒） | TOCTOU 窗口未闭（已缓解）；live E2E 未实测 |
| 5 | Gateway TLS 未接线 + none 逃生阀 | B5 | `1574335f` | PEM 真加载 fail-closed + tokio-rustls listener + 三态状态面；live 握手 E2E 真测 | TLS 1.2/1.3 矩阵、ECC、证书轮换未实测；none 逃生阀保留（显式自担+审计） |
| 6 | 会话/记忆静态未加密 + 备份恢复不完整 | B6 | `333c3471` | 分区 DEK + AES-256-GCM 信封（AAD 绑分区）+ KEK 三槽 + SealedBackup 宁拒不损 | 列级改造未接线（原语已交）；KekAnchor 平台锚定未接线；P1 字段定级待拍板 |
| 7 | 治理链「组件已写、整条链未演练」 | B7 | `1f65852b` | M-of-N 逻辑票链三路径演练 + 对抗用例 + 演练记录落档 | 缺口 6 项（编排未接线/物理多签留 v2.1/审计持久化面缺位等，详演练记录 §5） |
| 8 | 安装包来源验证不足 | B8 | `1213fbdd` | 发布资产校验脚本（哈希三口径 + 签名如实报）+ 流程文档 | 签名证书/可复现构建/平台校验/分通道 = 排期 4 项 owner 决策 |

## 二、门禁真账（各批收口实测，2026-10-19/20）

- 桌面 crate（B1-B4）：`cargo test --locked` 151 passed / 0 failed；clippy 0 警告；fmt 0 diff；pnpm 82/82 套件。
- gateway（B5）：304 passed / 0 failed（含 live TLS 握手 E2E）；clippy 0；fmt 0。
- storage（B6）：184 passed / 0 failed（三不变量钉死）；clippy 0；fmt 0。
- governance（B7）：197 passed / 0 failed；clippy 0；fmt 0。
- scripts（B8）：7 用例真二进制实测全符预期；check-neutral-terms 0 命中。
- **全 workspace 三件套**：见本文档末尾「收口补记」（与生物机制线 W3.3 并行期跑动，分账如实）。
- LOCKED 5 项 0 触碰（每批收口自验）；secret-scan 全过；0 新外部包（B5 +4 依赖边、B6 +7 依赖边，均锁内既有包）。

## 三、工程方法真账（多 AI 并行）

- 本线 8 批次 10 个施工者接力（3 次子代理静默中断，均按 MANIFESTO §13 #3 主代理自验接手/重派，无弃尸）。
- 与生物机制线并行施工期，`crates/adapters/cli/src/lib.rs` 同文件混两线改动 → **hunk 级分账提交**（git apply --cached 选择性暂存），两线各自 commits 零裹挟。
- Cargo.lock 依赖边同理分账（B5 gateway 边 / B6 storage 边分开入 commit）。
- 负载 flake 登记未消：admin_config 热更新 / supervisor_lifecycle 端口竞争（D4/D26 族，隔离复跑即绿）。

## 四、后续排期登记（owner 决策位）

1. B2「本地端点注册面」（回环类级口子收口）+ 公网提供方 allowlist —— 建议下一批立项。
2. B6 P1 敏感字段定级（memory_vectors 保明检索 vs 加密）→ 归口记忆线；KDF 参数基准校准。
3. B6 列级改造逐表接入（原语+证明已交，归口各存储 owner）；KekAnchor 平台锚定。
4. B7 治理缺口 6 项（编排接线/物理多签 v2.1/审计持久化等）——与 D2 挂账行合并排期。
5. B8 排期 4 项（签名证书/可复现构建/平台校验/分通道）。
6. B5 未实测面补测（TLS 协议矩阵/证书轮换流程）。

## 收口补记

- **全 workspace 三件套实测（2026-10-20，`-j 2` 口径）**：`cargo test --workspace --all-targets --locked --no-fail-fast` = **0 failed**（FAILED/failures 过滤零命中）；`cargo clippy --workspace --all-targets --locked -j 2 -- -D warnings` = **0 警告**（exit 0，Finished 15m03s）；`cargo fmt --all -- --check` = **0 diff**。passed 总数不抄写（单一真源 test-baseline.md 随下一批实测刷新）。
- 收口 commit 含 CHANGELOG [Unreleased] 安全加固批段 + 本交付文档；另一条线（生物机制 W3.3）的 4 个 docs 在途改动未裹挟。
