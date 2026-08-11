# apeireth-tui

> **Apeireth R19 TUI** — ratatui 终端版, 5 页面架构 (Cognition / 9 organ / Memory / Tools / Settings), 后端全接 (R128 minimax 4 协议 ready).
> **当前状态**: 254KB lib.rs + main.rs, 255KB 后端接 30+ crate.

---

## 启动

```bash
# 默认 (需要 minimax key)
$env:APEIRETH_API_KEY = "<minimax key from .openclaw\apikey.txt>"
cargo run --release -p apeireth-tui

# 启动后: 5 页面 Tab 切换
# - Cognition: AI 认知主区
# - Organs: 9 organ 监控
# - Memory: 历史 episode / note 流
# - Tools: 工具白名单 + 调用历史
# - Settings: 配置 / 模型 / 后端切换
```

## 5 页面架构 (per pages/*.rs)

```
src/pages/
├── dialogue.rs    # Cognition 主对话 (含 ratatui rendering + AI streaming)
├── organs.rs      # 9 organ dashboard (body / brain / ear / eye / hand / heart / memory / mind / voice)
├── memory.rs      # memory 流 (episode / note / session)
├── tools.rs       # 工具白名单 + 调用链
└── settings.rs    # 配置 + minimax / anthropic / openai 后端切换
```

## 后端依赖

- `apeireth-api` — 4 协议 minimax 真接
- `apeireth-memory` — SQLite 真持久化 + semantic_search
- `apeireth-council` — 7 advisor 决策
- `apeireth-sovereignty` — Self-Disable 判定
- `apeireth-tool-runtime` + `apeireth-tool-registry` + `apeireth-tool-approval` — 4 件套
- `apeireth-telemetry` — tracing + log
- `apeireth-bus` — internal bus
- `apeireth-protocol` — 4 协议 facade
- `apeireth-i18n` — i18n

## 依赖

- `ratatui` 0.30 + `crossterm` 0.28 (终端)
- `unicode-width` 0.2 (set_cursor alignment)
- `arboard` 3 (系统剪贴板 Ctrl+C 复制)
- `notify` 6 (R30 U15 tool-policy.json 热加载)

## 验证

```bash
cargo check -p apeireth-tui    # 0 errors, 4 历史 warnings (deprecated ratatui API + unused)
```

## See also

- [TUI 在 5 页面架构规范](src/pages/)
- [R30 U15 tool-policy 热加载 spec](../../docs/conventions/)