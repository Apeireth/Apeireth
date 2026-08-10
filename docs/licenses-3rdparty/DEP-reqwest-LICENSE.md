# reqwest 0.12 LICENSE (workspace 实际)

> **dep**: `reqwest`
> **version**: 0.12
> **license**: MIT OR Apache-2.0 (双许可)
> **关键作用**: HTTP client (SSE 流式, REST 调用)
> **完整 LICENSE 文本**: 见 [LICENSE-MIT.md](LICENSE-MIT.md) + [LICENSE-Apache-2.0.md](LICENSE-Apache-2.0.md)
> **最后更新**: 2026-08-06

---

## 版权

```
Copyright (c) Sean McArthur
```

## License (双许可)

Apeireth 选 **MIT** 作为 reqwest 的许可 (跟 workspace 一致).

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
reqwest = { version = "0.12", default-features = false, features = ["json", "rustls-tls", "stream"] }
```

**features 说明**:
- `json` — JSON 序列化
- `rustls-tls` — TLS (rustls, 纯 Rust, 0 OpenSSL 依赖)
- `stream` — SSE / chunked stream (TUI 流式响应依赖)
- `default-features = false` — 关掉 native-tls (OpenSSL), 改用 rustls

---

## 致谢

Apeireth workspace 在以下 crate 用 reqwest:
- `apeireth-api` (SSE 流式)
- `apeireth-sdk` (HTTP 客户端 SDK)
- `apeireth-credentials` (5 Provider 鉴权 HTTP)
- `apeireth-update` (autoupdate endpoint)

---

## 仓库 / 文档

- 仓库: https://github.com/seanmonstar/reqwest
- 文档: https://docs.rs/reqwest/0.12.0/reqwest/

---

**Last-Modified**: 2026-08-06
**owner**: 整合 #3 R21 续补 (D-1)
