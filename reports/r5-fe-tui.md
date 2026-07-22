# R5-FE Apeireth TUI 验收报告

## 产出 (774 LOC)

| 文件 | LOC | 角色 |
|---|---|---|
| apeireth/tui.py | 391 | 5 区 App + ArtifactReader + 路由/命令 |
| apeireth/tui_sessions.py | 122 | SessionStore 原子 JSON (.tmp rename) |
| apeireth/tui_widgets.py | 132 | TopBar/Metrics/ChatView/Palette/Bottom |
| tests/test_r5_tui_smoke.py | 125 | 7 烟测 (in-process pilot.run_test) |
| bin/apeireth-tui | 4 | sh 启动器, 注入 PYTHONPATH |

## 5 区结构

```
+-#topbar h3  MODEL | failover | cost | p50 --------+
+--left 20%--+--- center 60% ---+--right 20%--+
| Metrics    | #chat 滚动       | Metrics       |
| asi/trend  | #command-palette | guard/shells  |
| life/mem   | (Ctrl+P 切换)    | honesty/phi   |
+--#bottom h8  session xxx | mem N ----------+
| > Type a task or /help  Ctrl+P/Q/L         |
+--------------------------------------------+
```

| 区 | ID | 规格 | 来源 |
|---|---|---|---|
| 顶 | #topbar | h3 | TopBar.set_route (V1083 决策) |
| 左 | #left-panel | 20% | MetricsPanel ASI/trend/life |
| 中 | #chat + #command-palette | 60% | ChatView + CommandPalette |
| 右 | #right-panel | 20% | MetricsPanel guard/shells |
| 底 | #bottom-bar | h8 | BottomBar status+Input+键位 |

## 命令面板 (COMMANDS)

```
/model <name>   切换路由模型 (∈ SUPPORTED_MODELS)
/clear          清空 ChatView (保留 session)
/sessions       列最近 10 条
/switch <id>    加载 sess_<12hex>
/new            新 session, 重置 mem
/help           列 COMMANDS
/quit /exit     退出
```

键位: Ctrl+P 命令面板 / Ctrl+Q 退出 / Ctrl+L 清空。

## 持久化

- 路径: memory/sessions/sess_<12hex>.json
- 目录: APEIRETH_SESSIONS_DIR -> <repo>/memory/sessions
- 原子写: .tmp -> replace() (tui_sessions.py:117)
- id 正则: ^sess_[0-9a-f]{12}$
- 样例: sess_b111aa540cc8.json (8 行)

## 7 烟测

1. starts_and_quits - 5 selector + Ctrl+Q - PASS
2. send_message_no_crash - 无 key 真 llm_kernel + 2 条 - PASS
3. command_palette - Ctrl+P + /model/switch/quit - PASS
4. sessions_persist - 双 App 共享 session - PASS
5. switch_session - /new + /switch 跨 session - PASS
6. no_asi_leak_in_input - 占位无 "asi", 提交清空 - PASS
7. model_command - /model template 同步 - PASS

## pytest

```
tests/test_r5_tui_smoke.py ............... [100%]
========================= 7 passed in 1.18s =========================
```

## 主哲学不外显

TUI 默认不显 ASI 趋势/guard 明文/V1074 probe/V1082 空壳数/HQB verdict; 面板仅标签值, /sessions 只列 ID+消息数。task-input 占位符 > Type a task or /help 无 "asi" 字样。

## 启动

```sh
bin/apeireth-tui                       # 方式 1
PYTHONPATH=src python -m apeireth.tui  # 方式 2
```

无 key 走 llm_kernel 无 key 分支; 模型切换需已登记名。

## 下一步

- /search <text> 关键字检索当前 session
- V1084+/HQB 对话级 verdict 提示 (仅 --debug)
- 复用 R4-CLI 同一 task/model 契约至 apeireth-py SDK
