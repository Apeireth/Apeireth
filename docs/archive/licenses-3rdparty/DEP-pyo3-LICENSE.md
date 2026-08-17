# pyo3 0.29 LICENSE (workspace 实际)

> **dep**: `pyo3`
> **version**: 0.29 (R20 阶段 6 估补 0.22→0.29, 4 RUSTSEC fix 之一)
> **license**: Apache-2.0
> **关键作用**: Python 互操作 (仅 apeireth-pybridge)
> **完整 LICENSE 文本**: 见 [LICENSE-Apache-2.0.md](LICENSE-Apache-2.0.md)
> **最后更新**: 2026-08-06

---

## 版权

```
Copyright (c) 2017-present PyO3 Project Contributors
```

## License

```
Apache License
                           Version 2.0, January 2004
                        http://www.apache.org/licenses/

   ... (完整文本见 LICENSE-Apache-2.0.md) ...

   Copyright 2017-present PyO3 Project Contributors

   Licensed under the Apache License, Version 2.0 (the "License");
   you may not use this file except in compliance with the License.
   You may obtain a copy of the License at

       http://www.apache.org/licenses/LICENSE-2.0

   Unless required by applicable law or agreed to in writing, software
   distributed under the License is distributed on an "AS IS" BASIS,
   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
   See the License for the specific language governing permissions and
   limitations under the License.
```

---

## workspace 引用

```toml
# Cargo.toml [workspace.dependencies]
pyo3 = { version = "0.29", features = ["auto-initialize"] }
```

**features 说明**:
- `auto-initialize` — Python 解释器自动初始化 (per apeireth-pybridge 需求)

**升级历史**:
- 0.22 (R20 阶段 4 估补前) → 0.29 (R20 阶段 6 估补后)
- 升级原因: RUSTSEC-2024-0437 (pyo3 0.22 buffer overflow)
- per 整合 #3 H-3 决策, Cargo.lock 4 RUSTSEC fix 之一

---

## 致谢

Apeireth workspace 在以下 crate 用 pyo3:
- `apeireth-pybridge` (Python 互操作唯一)

其他 74 个 apeireth-* crate **不**用 pyo3 (per 主人 2026-08-04 拍板 "0 引 pyo3/qt/GDI").

---

## 仓库 / 文档

- 仓库: https://github.com/PyO3/pyo3
- 文档: https://docs.rs/pyo3/0.29.0/pyo3/
- 配套: maturin (build 工具) — 也 Apache-2.0

---

**Last-Modified**: 2026-08-06
**owner**: 整合 #3 R21 续补 (D-1)
