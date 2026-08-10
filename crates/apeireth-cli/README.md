# apeireth-cli

> **职责**: CLI 入口 + TUI + slash commands
> **状态**: R11 占位实现
> **对应文档**: 阶段 2 §3 入口层

---

## 设计意图

`apeireth-cli` 是 Apeireth 的"命令行入口"crate, 包含:

1. **CLI 入口** — `apeireth` 可执行文件
2. **TUI** — 终端 UI (类似 OpenClaw CLI)
3. **slash commands** — `/model`, `/mcp`, `/memory` 等
4. **Admin RPC** — Unix domain socket (本地管理)

## 命令列表 (v1)

```
apeireth                     # 交互式 REPL (默认)
apeireth chat                # 单次查询
apeireth serve               # 启动 API server + gateway + cron
apeireth gateway             # 仅 Gateway (无 API)
apeireth config              # 配置管理
apeireth model               # 模型/提供者管理
apeireth auth                # 认证 (登录/登出/状态)
apeireth tools               # 工具管理
apeireth mcp                 # MCP 服务器管理
apeireth skills              # 技能管理
apeireth memory              # 记忆提供者管理
apeireth doctor              # 健康检查
apeireth status              # 运行状态
```

---

_主哲学 anchor: 主 00:56 任何人都能接手 (CLI 友好)._