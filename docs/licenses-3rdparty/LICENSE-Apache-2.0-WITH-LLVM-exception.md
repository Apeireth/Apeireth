# Apache License 2.0 with LLVM Exceptions (Standard Text, deny.toml 增强)

> **类别**: Apache-2.0 WITH LLVM-exception
> **来源**: https://foundation.llvm.org/relicensing/LICENSE.txt
> **使用 crate 数**: ~3 (关键: llvm-sys, deny.toml 增强)
> **最后更新**: 2026-08-06

---

## Apache-2.0 + LLVM Exception (Standard Text)

**主文本**: Apache-2.0 (见 [LICENSE-Apache-2.0.md](LICENSE-Apache-2.0.md))

**LLVM Exception** (附加):

```
--- LLVM Exceptions to the Apache 2.0 License ----

As an exception, if, as a result of your compiling your source code, portions
of this Software are embedded into an Object form of such source code, you
may redistribute such embedded portions in such Object form without complying
with the conditions of sections 4(a), 4(b) and 4(d) of the License.

In addition, if you combine or link compiled forms of this Software with
software that is licensed under the GPLv2 ("Combined Software") and if a
court of competent jurisdiction determines that the patent provision (Section
3), the indemnity provision (Section 9) or other Section of the License
conflicts with the conditions of the GPLv2, you may do either of the following:

 a) Accompany the Combined Software with the complete corresponding
    machine-readable source code for the Combined Software (the
    "Combined Software Source"), together with written offer, valid for at
    least three years, to give the Combined Software Source to any third
    party, for a charge no more than your cost of physically performing this
    distribution, or
 b) Use the Combined Software under the terms of the Apache License, Version
    2.0, together with the applicable terms of the GPLv2, as indicated by
    the license attached to the Combined Software.

The Combined Software and the Combined Software Source are provided to you
"AS IS" and without warranty of any kind.
```

---

## 例外意义

**Apache-2.0 + LLVM Exception** 是 LLVM 项目的特殊 license:
- 允许把 LLVM 代码嵌入到**任何**形式 (含商业闭源), **不**强制 §4(a/b/d)
- 允许跟 GPLv2 链接
- **不**影响 Apache-2.0 主体的其他条款 (§1-§3, §4(c), §4(e), §5-§13)

---

## Apeireth workspace 实际 Apache-2.0 + LLVM-exception 依赖 (关键)

| Crate | Version | 关键作用 |
|-------|---------|---------|
| **llvm-sys** | 2.0 | LLVM FFI (apeireth-formal 用) |

> 完整 crate attribution 见 `THIRD-PARTY-NOTICES.md` §A.

---

**Last-Modified**: 2026-08-06
**owner**: 整合 #3 R21 续补 (D-1)
