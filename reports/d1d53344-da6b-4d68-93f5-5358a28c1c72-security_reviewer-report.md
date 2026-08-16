# 自审报告 — 任务 d1d53344: TP3/N21 apeireth-credentials 新 crate（§10 装配主链第一环）

> 角色: security_reviewer | 日期: 2026-08-17 | crate: apeireth-credentials (新建, 独占改动)

## 1. 任务目标 → 交付对照

| 验收项 | 状态 | 证据 |
|---|---|---|
| 统一接口 CredentialsStore trait + 文件形态后端 | ✅ | `CredentialsStore` (get/set/delete/list/contains 按服务名) + `FileCredentialsStore` (JSON 单文件后端) |
| 衔接权限洋葱: master token 走既有审批链 | ✅ | `CredentialGate` trait 口 + `GatedCredentialsStore` 装饰器: master/master_token/master-token 高危名单, 读写删须 gate 放行; 复用 sovereignty master token 批准语义 (companion::principles 同款"比对不落日志"), **不改 sovereignty/constraint/tool-approval 本体** |
| 不存明文到日志/错误消息 (写入前脱敏) | ✅ | `SecretString` 覆写 Debug/Display 恒 `[REDACTED len=N]`; `CredentialsError` 各变体只含服务名元信息; 回归测试 `debug_never_leaks_plaintext` / `error_messages_do_not_leak_secret` 等 |
| 文件权限 600 语义标注 | ✅ | unix `set_permissions(0o600)` + 测试 `file_permission_is_owner_only` (cfg(unix)); 非 unix 依赖默认 ACL, 0 装标注 |
| 未知名报错 | ✅ | `CredentialsError::UnknownService` (get/delete), 测试 `get_unknown_service_errors` / `delete_unknown_service_errors` |
| 0 装"安全存储"边界如实标注 | ✅ | lib.rs 模块头 + 模块地图行 + 本报告 §4: 明文静态存储非加密保险库, 加密属后续层 |
| workspace 注册 + 消费方声明 (N18 规范) | ✅ | 根 Cargo.toml members 注册 (注释 TP3/N21); lib.rs "消费方声明" 段: companion 装配侧随 N17 统一接入 (trait 口就绪 0 装) |
| cargo test -p apeireth-credentials -j 4 全绿 | ✅ | **25 passed, 0 failed** |
| 台账 N21 划✅ + 模块地图 | ✅ | backlog.md N21 行 ✅ (提交 6ec1ac7, 后被并行 rebuild 提交 65b5fdf4 收编, 内容已在 HEAD); maintenance-guide.md 模块地图行 (6ec1ac7) |
| 自审报告 | ✅ | 本文件 |

## 2. 测试证据 (0 装)

`cargo test -p apeireth-credentials -j 4` → **25 passed / 0 failed**:
- secret.rs 6 测: Debug/Display 不泄漏 / 长度元信息 / expose / 空值 / 展示脱敏短长串
- error.rs 3 测: 各变体消息只含元信息
- store.rs 9 测: 读写往返 / 未知名 get+delete 报错 / list+contains / 非法名拒绝 (含 `.`/`..`/`.hidden` 路径穿越) / 错误不泄漏明文 / 校验规则 / unix 权限 600
- gate.rs 6 测: 高危读被拒 (错误不含明文) / 普通服务放行 / AllowAll 放行 / 高危写被拒 / list 直通只出名称 / 高危名单
- lib.rs 1 测: 端到端冒烟 (普通读写 + 未知名报错 + 高危 fail-closed 一体)

## 3. 提交清单

| 提交 | 内容 |
|---|---|
| f59ce83f | feat: 新 crate 骨架入库 (Cargo.toml + 4 模块 + workspace 注册) — 防 sweep 先行落库 |
| 6ec7995e | fix: 服务名点开头拒绝 (`.`/`..`/`.hidden` 路径穿越, 测试抓出) + gate.rs 无用占位清理 |
| 6ec1ac7c | docs: 模块地图登记 (台账 N21 ✅ 被并行提交收编, 内容在 HEAD) |

## 4. 0 假装边界 (诚实标注, 与 lib.rs 模块头一致)

1. **本层是凭据存取抽象, 不是加密保险库**: FileCredentialsStore 为**明文静态存储**, 靠 OS 文件权限 (unix 600) 收敛访问。加密静态存储 (OS keyring/KMS/age) 属后续层, 届时实现 `CredentialsStore` 换后端即可, 接口不变。
2. **SecretString 非内存安全容器**: 未做 zeroize/mlock, 只保证不泄漏到输出通道 (日志/错误/Debug/Display)。内存级擦除属后续层。
3. **审批门是 trait 口 (0 装)**: `DenyAllGate` 默认 fail-closed (未挂真审批链时高危一律拒, AI 不接触明文 token); 真审批链 (master token 批准) 挂接在 companion 装配侧, 随装配主链后续环实施 — 与 N15 Classifier 注入口同模式。
4. **消费方当前为 0**: 按 N18 规范如实声明 — 消费方 = companion 装配侧 (随 N17 工具装配统一接入), 当前 trait 口 + 后端就绪, 无引用方。

## 5. 过程实录 (供 Leader 复盘)

- **在途工作两次被并行 rebase sweep 卷走** (stash "N14-rebase"/"N14-rebase2: 保护他人 WIP"): 整个 crate 目录 + Cargo.toml 注册行两次消失。均从 `stash@{0}^3` (未跟踪文件提交) 用 `git checkout <hash> -- crates/apeireth-credentials/` 恢复; 第二次恢复后采取"骨架先落库再补修复"策略 (f59ce83f 先提交, 6ec7995e 再修复), 后续不再丢失。stash 归属他人, **未越权 drop**。
- 提交均用 `git commit <pathspec>` 限定自身路径, 避免卷入他人 staged 文件 (上次任务曾误扫, 已吸取)。
- Cargo.lock 工作区差异含并行工作混入 (非纯本 crate条目), 未纳入提交, 留给整合收敛。
- 路径穿越防护是**测试先行抓出**的: 初版校验允许 `..` 通过, `invalid_service_name_rejected` 测试失败暴露, 补 `.`/`..`/点开头拒绝后全绿 — 0 装证据链完整。

## 6. 后续 (非本任务范围, 留口)

- 装配主链后续环: companion 装配侧构造 `GatedCredentialsStore<FileCredentialsStore, 真审批门>`, 工具/插件按服务名取凭据替代各读 env (随 N17)。
- 加密静态存储层 (OS keyring/KMS/age): 实现 `CredentialsStore` 换后端, 接口已为此设计。

## 7. Round 2 复核实录 (评审返工轮)

Round 1 评审报 "deliverable_missing / 缺 lib.rs / 无提交 / 无报告" — **复核结论: 评审基于过期 integration 快照** (快照仅拾取 3 个未跟踪脏文件), 非交付缺失。复核证据 (master HEAD = 181a017d, 2026-08-17):

| 核验项 | 命令 | 结果 |
|---|---|---|
| 5 源文件入库 | `git ls-tree HEAD crates/apeireth-credentials/src/` | error/gate/lib/secret/store.rs 全在 |
| lib.rs 导出 CredentialsStore | `git show HEAD:.../lib.rs \| grep 'pub use store::'` | 1 命中 |
| workspace 注册 | `git show HEAD:Cargo.toml \| grep -n apeireth-credentials` | 第 84 行 |
| 提交在祖先链 | `git merge-base --is-ancestor 6ec7995e HEAD` | YES; f59ce83f + 6ec7995e 均在 HEAD 历史 |
| 台账 N21 ✅ / 模块地图 | grep HEAD 版 backlog/maintenance-guide | 各 1 命中 |
| 自审报告 | `git ls-tree HEAD reports/ \| grep d1d53344` | 1 命中 |
| 测试 | `cargo test -p apeireth-credentials -j 4` | **25 passed, 0 failed** |

当前 integration 分支 (934509527a) 已含全部 crate 文件与注册 (blob 哈希与 HEAD 逐一相同); 本报告 Round 2 追加段随本提交入库后交付链在 master 闭环, 待下一轮 integration 同步。三源文件本体 (error/secret/store) 与评审确认方向一致, 未重写。
