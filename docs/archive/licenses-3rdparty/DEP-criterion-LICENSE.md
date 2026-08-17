# criterion 0.5 LICENSE (workspace 实际)

> **dep**: `criterion`
> **version**: 0.5 (1.0 release 必做 #7 perf)
> **license**: MIT OR Apache-2.0 (双许可)
> **关键作用**: 性能 bench 框架 (1.0 release 12 项 checklist #7 必做)
> **完整 LICENSE 文本**: 见 [LICENSE-MIT.md](LICENSE-MIT.md) + [LICENSE-Apache-2.0.md](LICENSE-Apache-2.0.md)
> **最后更新**: 2026-08-06

---

## 版权

```
Copyright (c) 2014-2024 Jorge Aparicio
```

## License (双许可)

Apeireth 选 **MIT** 作为 criterion 的许可 (跟 workspace 一致).

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

```toml
# Cargo.toml [workspace.dependencies]
criterion = { version = "0.5", features = ["html_reports"] }
```

**features 说明**:
- `html_reports` — 生成 HTML 报告 (1.0 release 必做, per 整合 #3 D-4 决策 #7 perf 100%)

---

## 致谢

Apeireth workspace 在以下 crate 用 criterion (1.0 release 估补):
- `apeireth-bench` (主 bench 框架)
- `apeireth-tui` (9 器官 54 command bench, per 整合 #3 C-1)
- `apeireth-vector` (tantivy 索引 bench)
- `apeireth-cache` (5 EvictionPolicy × 4 BackendKind bench)
- `apeireth-sandbox` (3 RuntimeKind × 6 API bench)
- 17 bench 文件 / 1,275 行 (per 整合 #3 D-4 决策 #7 perf 100%)

---

## 仓库 / 文档

- 仓库: https://github.com/bheisler/criterion.rs
- 文档: https://docs.rs/criterion/0.5.0/criterion/
- 1.0 release 必做: HTML 报告 + baseline 模式

---

**Last-Modified**: 2026-08-06
**owner**: 整合 #3 R21 续补 (D-1)
