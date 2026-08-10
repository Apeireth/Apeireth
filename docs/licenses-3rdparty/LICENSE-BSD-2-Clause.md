# BSD 2-Clause License (Standard Text, deny.toml 增强)

> **类别**: BSD-2-Clause (Simplified BSD / FreeBSD License)
> **来源**: https://opensource.org/licenses/BSD-2-Clause
> **使用 crate 数**: ~15 (transitive 关键: libc / bitflags / libsqlite3-sys, deny.toml 增强)
> **最后更新**: 2026-08-06

---

## BSD 2-Clause License (Standard Text)

```
BSD 2-Clause License

Copyright (c) <YEAR>, <OWNER>
All rights reserved.

Redistribution and use in source and binary forms, with or without
modification, are permitted provided that the following conditions are met:

1. Redistributions of source code must retain the above copyright notice,
   this list of conditions and the following disclaimer.

2. Redistributions in binary form must reproduce the above copyright notice,
   this list of conditions and the following disclaimer in the documentation
   and/or other materials provided with the distribution.

THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE
LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR
CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF
SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS
INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN
CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE)
ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE
POSSIBILITY OF SUCH DAMAGE.
```

---

## Apeireth workspace 实际 BSD-2-Clause 依赖 (transitive 关键)

| Crate | Version | 关键作用 |
|-------|---------|---------|
| **libc** | 0.2 | libc FFI (几乎所有 native crate 用) |
| **bitflags** | 2.6 | 类型化 bit flag (transitive 广) |
| **libsqlite3-sys** | 0.28 | SQLite FFI (transitive, 替代 BSD-3) |

> 完整 BSD-2-Clause crate attribution 见 `THIRD-PARTY-NOTICES.md` §A (按 BSD-2-Clause 分组).

---

**Last-Modified**: 2026-08-06
**owner**: 整合 #3 R21 续补 (D-1)
