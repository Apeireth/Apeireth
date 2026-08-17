# rusqlite 0.32 LICENSE (workspace 实际)

> **dep**: `rusqlite`
> **version**: 0.32 (workspace 硬锁, 0 重复造轮子)
> **license**: MIT
> **关键作用**: SQLite 客户端 (memory / vector / api / mcp 4 crate 用)
> **完整 LICENSE 文本**: 见 [LICENSE-MIT.md](LICENSE-MIT.md)
> **最后更新**: 2026-08-06

---

## 版权

```
Copyright (c) 2014-2024 rusqlite authors
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

```toml
# Cargo.toml [workspace.dependencies]
rusqlite = { version = "0.32", features = ["bundled"] }
```

**features 说明**:
- `bundled` — 编译时打包 libsqlite3 源码, **0 系统 SQLite 依赖**

**workspace 硬锁原因**:
- 防止 V2 各 crate libsqlite3-sys 冲突 (不同版本 ABI 不兼容)
- per 整合 #3 H-3 决策, Cargo.lock 4 RUSTSEC fix 时不破坏 workspace version 1.0.0 (semver 兼容)

---

## 致谢

Apeireth workspace 在以下 crate 用 rusqlite:
- `apeireth-memory` (向量 + 全文 持久化)
- `apeireth-vector` (tantivy 索引 metadata)
- `apeireth-api` (session 持久化)
- `apeireth-mcp` (MCP 资源持久化)
- `apeireth-vector` (SQLite FTS5 全文检索)

---

## 仓库 / 文档

- 仓库: https://github.com/rusqlite/rusqlite
- 文档: https://docs.rs/rusqlite/0.32.0/rusqlite/
- SQLite 版本: 3.46+ (per `bundled` feature, 跟 rusqlite 0.32 同步)

---

**Last-Modified**: 2026-08-06
**owner**: 整合 #3 R21 续补 (D-1)
