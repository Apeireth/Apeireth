# Companion daemon 部署与接线说明（2026-08-16）

## 环境变量（companion_daemon example）

| 变量 | 作用 |
|---|---|
| `APEIRETH_API_KEY` | MiniMax key（或读 `apikey-ultra.txt`） |
| `APEIRETH_TICK_SECS` / `APEIRETH_MAX_TICKS` | 心跳间隔 / 最大轮数 |
| `APEIRETH_MEMORY_PATH` | 记忆库路径（默认 `%APPDATA%\apeireth\memory.sqlite`） |
| `APEIRETH_SUBJECT` | 记忆检索 session id（默认 `me`） |
| `APEIRETH_MIN_LLM_INTERVAL_SECS` | 两次主动最小间隔（默认 60） |
| `APEIRETH_SINK=lark` | 飞书通道（需下面 4 个 LARK 变量） |
| `APEIRETH_LARK_APP_ID` / `APP_SECRET` / `RECEIVE_ID` / `BASE_URL` | 飞书凭据 |
| `APEIRETH_DREAM=1` | 做梦：6h 无互动 → 合并 + LLM 摘要写回真库 |
| `APEIRETH_REFLECT=1` | 反思：24h 一轮 → 反思记录写回真库 |
| `APEIRETH_SEED_DEMO=1` | demo 种子（预填 7 天作息） |

## 飞书真发（等凭据）

- 接线已完成：`LarkSink::from_env()`（`APEIRETH_LARK_*`）+ `APEIRETH_SINK=lark` 切换。
- 拿到 app_id/app_secret/receive_id 后：`set APEIRETH_SINK=lark` 等 → `cargo run -p apeireth-companion --example companion_daemon` 即真发。
- 注意 `apeireth-lark` 的 `LarkRealImpl` 走飞书 Open API（tenant_access_token 自动缓存 7200s）。

## Windows Hello / 物理多签（口子已备，绑定待做）

- sovereignty 已有守门：`ExecutionPhysicalMultisigAction`（`crates/apeireth-sovereignty/src/action_rail.rs`）——物理多签守门三号位。
- 权限包哲学：签包前一次强确认（Windows Hello 起步，FIDO2/YubiKey 留口）；`sudo -v` 包级扩展。
- **待做**：真实 Windows Hello SDK 绑定（平台 API → 确认回调 → `report_violation`/签包登记）。当前 `MockBiometric`/`CoercionBehavior` 口子已备。

## lightmemo L1-L4（独立记忆系统，未接 daemon）

- `apeireth-memory::lightmemo`：L1 文件 / L2 向量 / L3 标签 / L4 LCM 渐进 + DecayEngine（Ebbinghaus 遗忘曲线，**已虚拟时钟化**：`with_config_and_clock` + VirtualClock 快进测试）。
- 与 companion 的 SQLite episodes 记忆是**两轨**；接 daemon 需先定「主记忆 = episodes，L1-L4 作分层索引」的语义。未接，评估后接（backlog）。

## 时间机制虚拟化清单（全部可快进测试）

- SleepCycle（做梦安静期）✓ / DecayEngine（遗忘曲线）✓ / PackRegistry（到期/续签，参数驱动）✓ / Emergence 节律（参数驱动）✓ / ReflectionCycle（参数驱动）✓ / DreamScheduler（虚拟时钟）✓ / 模拟验收 `virtual_time_simulation`（14 项，3ms）。
