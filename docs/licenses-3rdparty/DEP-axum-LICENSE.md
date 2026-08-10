# axum 0.7 LICENSE (workspace 实际)

> **dep**: `axum`
> **version**: 0.7 (transitive, 主 HTTP server 框架)
> **license**: MIT
> **关键作用**: HTTP server (apeireth-api / apeireth-web 主体)
> **完整 LICENSE 文本**: 见 [LICENSE-MIT.md](LICENSE-MIT.md)
> **最后更新**: 2026-08-06

---

## 版权

```
Copyright (c) 2019-2024 Tokio Contributors
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

axum 是 **transitive dep** (经 tokio + tower 间接引入), 不在 [workspace.dependencies] 显式声明, 但实际 6.1k crates 引用, 是 0.7 版本关键 HTTP server.

**为何 apeireth-* 选用 axum**:
- 🟢 跟 tokio 同生态 (per APEIRETH-CONVENTIONS §11 "tier 1 生态")
- 🟢 类型安全 (handler 函数签名 = 路由 schema)
- 🟢 middleware 生态丰富 (tower)
- 🟢 WebSocket 支持 (跟 apeireth-protocol 配合)

**为何 0.8 不升**:
- 0.7 是 R20 阶段 6 时稳定版本
- 0.8 在 R21+ 续 (per 整合 #3 拍板 "semver 严守, 0 跳 major version")

---

## 致谢

Apeireth workspace 在以下 crate 用 axum:
- `apeireth-api` (HTTP server 主入口, 6 tools + 3 observability + auth + rate-limit)
- `apeireth-web` (管理面板)
- `apeireth-protocol` (WebSocket /v1/ws 端点)

---

## 仓库 / 文档

- 仓库: https://github.com/tokio-rs/axum
- 文档: https://docs.rs/axum/0.7.0/axum/

---

**Last-Modified**: 2026-08-06
**owner**: 整合 #3 R21 续补 (D-1)
