# 真机实测 · 挂账族解锁手册（D1，2026-10-26）

> **目的**：把 D1（`00-backlog.md` 的「台账 §2 挂账族」）从「一堆挂着的行」变成
> **「每行缺什么、谁能解、一条命令、多久」**的清单 —— 让 owner 的真机时间**只花在机器能做的部分**。
>
> - 挂账台账（真源）：[live-verification-ledger.md](live-verification-ledger.md) 的 **§2**
> - 债栏：`docs/04-internal/backend-enhancement/00-backlog.md` 的 **D1** 行
> - 验收制依据：`docs/01-architecture/founding-design-v2.md` **§7 四层验收制**
>   （① 形式验证 ② 基准复现 ③ **场景实测 = 真机走查** ④ **主人签核**）

## 0. 一句话结论

**九行里只有三件事真的需要人**：
① **一个可用的 provider 凭证**（解锁行 1/4/9 —— 三行共用同一个阻塞）；
② **一个 embeddings 端点**（行 8）；
③ **一次 UI 视觉确认**（行 3/4 的最后一厘米，协议侧我已验完）。

其余六行要么**已经完成**（行 2），要么**本机已出结论**（下面 §2），要么**不是人工能解的**（行 6/7）。

## 1. 归并：九行 → 四个解锁项

| 解锁项 | 解锁哪几行 | 谁能解 | 需要什么 | 预计耗时 |
|---|---|---|---|---|
| **U1 provider 凭证** | **1 / 4 / 9** | owner | 一个**有效的** OpenAI-compatible key（本机无任何 provider 凭证；旧 DeepSeek key 尾号 `3d17` 已被官方撤销） | 取得凭证 ≈ 1 分钟；跑完 ≈ 10 分钟 |
| **U2 embeddings 端点** | **8** | owner | 一个真 `/embeddings` 端点（DeepSeek **不提供**；本机无本地推理服务、无 ollama） | 取得端点 ≈ 取决于是否有预算 |
| **U3 UI 视觉确认** | **3 / 4 的最后一厘米** | owner | 打开桌面端，看一眼事件流与审批卡 | ≈ 5 分钟 |
| **U4 平台/硬件** | **5 / 6 / 7** | 视情况 | macOS 或 Linux 真机（行 5）；MSI 是否成主力分发（行 6）；voice/screen 硬件（行 7） | 不定 |

> ⚠️ **行 1/4/9 此前被当成三件独立的事，实际共用一个阻塞**：本轮实测发现 gateway **没有
> `/v1/tools/*/invoke` 路由** ⇒ **待审批记录只能由 LLM 驱动的对话产生** ⇒ **无 key ⇒ 行 4 的 happy path
> 在本机根本不可达**（不是"忘了测"，是"测不了"）。⇒ **给一个 key，三行一起解锁。**

## 2. 我已经代做的部分（**不占用 owner 时间**）

| 行 | 我做了什么 | 结论 |
|---|---|---|
| **3**（events 端到端） | 起真 gateway（`apeireth gateway serve --port 18080`）→ `curl -N /v1/apeireth/events` | ✅ **HTTP 200 + `content-type: text/event-stream`**，并立即推一条 `stream_snapshot`（内容是「gateway token not configured」的诚实告警）⇒ **端点是活流**；**只剩 UI 消费的人工确认** |
| **4**（approvals HTTP 闭环） | 对 `/v1/approvals`（GET）与 `/v1/approvals/resolve`（POST）做参数校验探针 | ✅ 缺 `session` ⇒ **400**（GET）/ **422**（POST）；`session` 非 UUID ⇒ **422** 且给出精确位置 ⇒ **参数校验层诚实且精确**；**happy path 阻塞于 U1** |
| **行 2 之外的"shell 到达真机"** | 跑仓内 `#[ignore]` live 探针 `shell_reach_probe`（**无需 key**） | ✅ `shell_reach_reads_outside_workspace` **通过**（`cat /etc/hostname` 读到工作区外 ⇒ 非沙箱现状实锤，**跨机器第二证**：不是 Windows 特有）；⚠️ `shell_sandbox_probe_walls_user_space` **失败**，原因是 `IsolationRequirementUnsatisfied{FilesystemIsolation(Enforced), Network…}` ⇒ **Linux 无等价隔离后端** ⇒ 该探针是 **Windows 语义**，**如实标注平台**，不是产品缺陷 |
| **行 8 的端点排查** | 扫本机常见端口（11434/8080/8000/1234/5000）+ 查 ollama | **本机无任何本地推理/embedding 服务** ⇒ 行 8 确实缺端点（不是漏配） |
| **行 5 的打包前置** | 查 `webkit2gtk-4.1` | **本机缺** ⇒ **Linux 上打不出 Tauri 桌面包** ⇒ 行 5 需真机（或一次系统依赖安装） |

## 3. owner 侧的操作清单（**按这个顺序做，每步都有确切判据**）

### U1 —— 拿到 key 之后（**一次解锁三行**）

```bash
# 环境口径（抄台账 §3；key 不进 repo、不进 commit）
export OPENAI_API_KEY='<新key>'
export APEIRETH_OPENAI_URL='https://api.deepseek.com/v1'
export APEIRETH_OPENAI_MODELS='deepseek-v4-flash'

# 行 9：provider live E2E（2 支）
cargo test -p apeireth-provider --test openai_compatible_live -- --ignored
# 行 9：organ live（2 支）
cargo test -p apeireth-organ --test world_model -- --ignored
cargo test -p apeireth-organ --test causal_world_model -- --ignored
# 行 9（附）：council live
cargo test -p apeireth-orchestration --test council_live -- --ignored
# 行 1：MiniMax（**另需 MiniMax key**，与 DeepSeek 不同家）
cargo test -p apeireth-provider --test minimax_llm_factory -- --ignored
```
**判据**：全部 **绿**（不是「不红」）⇒ 挂账行删除，证据搬进台账 §1（填「何时/证据/复现命令」）。

### U2 —— 有 embeddings 端点之后（行 8）

```bash
export APEIRETH_EMBEDDING_URL='<端点>'
export APEIRETH_EMBEDDING_MODEL='<模型>'
# 起 gateway 真聊一轮，观察 recall 的 retrieval_stats
```
**判据**：recall 命中且 `retrieval_stats` 有真值（非 null）。

### U3 —— UI 最后一厘米（行 3/4）

1. 桌面端登录后触发一次事件（如开一个新会话）⇒ 看事件流是否实时出现；
2. 触发一个需要审批的工具（如 shell）⇒ 看审批卡出现 ⇒ 点批准 ⇒ 看是否**真执行**。

**判据**：① 事件流有动静；② 审批卡出现→批准→真执行（这就是行 4 的 happy path，**必须人工**）。

### 行 2 —— **可能可以直接核销**（请确认一句话）

台账 §2 行 2 写着「✅ **核心 10 步全绿（2026-09-28 用户真机签署）**」⇒ **实质已完成**，
只是还留在挂账区。**唯一需要你确认的是**：那次签署对应的构建**是否仍代表当前版本**？
若仍代表 ⇒ **行 2 可直接删**（挂账区只留未完成项，台账的规则也是「挂账解除 → 从 §2 删行」）。

## 4. 我**做不了**的（如实登记，不装）

| 行 | 为什么我做不了 | 谁能 |
|---|---|---|
| 5（macOS/Linux 装机） | 本机缺 `webkit2gtk-4.1` ⇒ 打不出包；且**装机验证要在目标机做** | 有 macOS/Linux 的人 |
| 6（MSI 卸载对齐） | **不是人工能解**（缺 WiX CustomAction 代码）⇒ 这是**工程活**；但需先拍板「MSI 是否成主力分发」⇒ 无人拍板前不动 | owner 拍板后我来做 |
| 7（voice/screen） | **待硬件** | 有硬件的人 |

## 5. 若 owner 要我把 U1 之外的事也做掉

我可以立刻开工的（等你一句话）：
1. **行 6 的 WiX CustomAction**（工程活，我来做；前提：MSI 要成主力分发）；
2. **行 5 的 Linux 打包链路**（装系统依赖 + 出 `.deb`/AppImage ⇒ 至少证明「能打出来」，
   装机仍需目标机）；
3. **行 3/4 的自动化回归**（把今天的 curl 探针固化成仓内脚本 + CI，让 UI 侧只剩「看一眼」）。

## 6. 本手册的边界

- 上面每个「✅/❌」都是**我本机实测的**（2026-10-26，Linux），**不是**在 owner 的 Windows 机上测的；
- 台账 §1 的历史行**一条未改**（历史不改写）⇒ 本手册是**增量**；
- **行 2 的核销需要你一句话**（我无法代替真机签署）。
