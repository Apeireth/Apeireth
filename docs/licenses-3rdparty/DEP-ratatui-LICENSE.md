# ratatui 0.28 LICENSE (workspace 实际)

> **dep**: `ratatui`
> **version**: 0.28 (transitive, TUI 框架)
> **license**: MIT
> **关键作用**: TUI 框架 (apeireth-tui 主体, 5 nav + 9 器官)
> **完整 LICENSE 文本**: 见 [LICENSE-MIT.md](LICENSE-MIT.md)
> **最后更新**: 2026-08-06

---

## 版权

```
Copyright (c) 2016-2024 Ratatui Developers
```

## License

```
MIT License

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

---

## workspace 引用

ratatui 是 **transitive dep** (经 apeireth-tui 引入), 不在 [workspace.dependencies] 显式声明.

**为何 apeireth-tui 选用 ratatui**:
- 🟢 TUI 业界主流 (前身 tui-rs, 2022 重启 ratatui)
- 🟢 TestBackend 支持 (per 整合 #3 G-1 "ratatui TestBackend 测 TUI 设计契约")
- 🟢 StatefulWidget 模式 (per 整合 #3 C-6 "SharedState<T> 1:1 镜像")
- 🟢 0 GUI 依赖 (per 主人 2026-08-04 拍板 "0 引 pyo3/qt/GDI")

**为何 0.29 不升**:
- 0.28 是 R20 阶段 6 时稳定版本
- 0.29 在 R21+ 续

---

## 致谢

Apeireth workspace 在以下 crate 用 ratatui:
- `apeireth-tui` (TUI 主入口, 5 nav + 9 器官 + 54 command, per 整合 #3 C-1)
- `apeireth-tui-e2e` (TUI 端到端测试, ratatui TestBackend 测 25+ cases, per 整合 #3 H-2)

---

## 仓库 / 文档

- 仓库: https://github.com/ratatui/ratatui
- 文档: https://docs.rs/ratatui/0.28.0/ratatui/
- 配套: crossterm (跨平台 terminal) — MIT
- 配套: tui-input (输入处理) — MIT
- 配套: tui-textarea (多行编辑) — MIT

---

**Last-Modified**: 2026-08-06
**owner**: 整合 #3 R21 续补 (D-1)
