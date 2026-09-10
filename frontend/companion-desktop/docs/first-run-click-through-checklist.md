# 桌面 UI 点击流人工实测清单（做一次即可）

> 台账挂账 #2（`docs/04-internal/live-verification-ledger.md`）。各链路段全部有自动化验证
> （install-e2e / supervisor_lifecycle / 前端套件 / 真机流式探针），但**真窗口里的端到端
> 点击流从未人工点过**——这是唯一建议后人做一次的测试。做完后把结果写回台账挂账 #2
> 并删掉本清单头部这句话。

## 前置

1. 装最新 NSIS 包（`target\desktop-nsis\Apeireth Companion_*_x64-setup.exe` /S），
   或直接跑 `scripts/install-e2e.ps1` 后再装一次留机。
2. 准备 DeepSeek key（或任意 OpenAI 兼容服务商 key）。
3. 打开日志尾巴备用：`%LOCALAPPDATA%\Apeireth Companion\logs\apeireth-backend.log`。

## 步骤（预期结果逐条对照）

| # | 操作 | 预期 |
|---|---|---|
| 1 | 启动 companion-desktop | 出现**首启向导**（选服务商/填 key/开始使用） |
| 2 | 选 DeepSeek → 填 key → 点"开始使用" | 向导关闭；日志出现 `backend.ready pid=… port=…`（网关重启过，端口非 8080） |
| 3 | 在聊天框发一句较长问题（如"分三步介绍机器学习"） | 回复**逐字/逐块出现**（token 级流式），不是整段蹦出 |
| 4 | 打开"运行时诊断"弹窗 → 点"拉取快照" | 快照 JSON 显示 providers（含 openai-compatible）/ 状态 |
| 5 | 设置 → 高级能力 → 开 Shell 命令工具 → 保存 | 日志显示网关重启；工具面板（活动/工具）出现 shell 工具、权限 granted |
| 6 | 聊天里要求"用 shell 执行 echo hello-from-clickthrough" | 出现**审批提示**（模型提议被冻结），回复不执行 |
| 7 | 在审批面板点"批准" | 工具真执行，模型回报结果（含 hello-from-clickthrough） |
| 8 | 关掉主窗（隐藏托盘）再点托盘图标 | 窗口恢复，会话历史仍在 |
| 9 | 完全退出应用（托盘退出） | `apeireth.exe` 侧车进程消失（supervisor kill_on_drop 回收） |
| 10 | 再启动 | 向导不再出现；设置里 url/模型已记住，key 需要重新输入（设计如此：key 不落盘） |

## 失败时怎么办

- 任一步与预期不符 → 记录步骤号 + `apeireth-backend.log` 尾 50 行 + 前端 console，
  写进台账挂账 #2 并开 issue；不要默默重跑（重跑不产生证据）。
- 流式"整段蹦出"= 后端回退路径被触发（provider 未声明 Streaming）或前端 fetch 非流式
  分支——查 `/v1/models` 是否列出 deepseek-v4-flash，再查网关版本（`apeireth.exe --version`）。
