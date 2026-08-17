# Creative Commons Zero 1.0 Universal (Standard Text, deny.toml 增强)

> **类别**: CC0-1.0 (Public Domain Dedication)
> **来源**: https://creativecommons.org/publicdomain/zero/1.0/
> **使用 crate 数**: ~5 (transitive 关键: serde_derive_internals / toml_edit, deny.toml 增强)
> **最后更新**: 2026-08-06

---

## CC0 1.0 Universal (Standard Text, 摘要)

完整文本见 https://creativecommons.org/publicdomain/zero/1.0/legalcode (5 段)

**核心条款** (CC0 1.0 全文):

### §1: 定义
> "Work" means the material you receive under this License...
> "Licensor" means the individual or entity offering the Work under this License...

### §2: 弃权 (Waiver)
> To the fullest extent permitted by applicable law, **the Licensor hereby waives all copyright and related or neighboring legal rights** in the Work...

### §3: 公共领域 fallback
> If the waiver in §2 is ineffective for any reason, then the Licensor grants You a **worldwide, royalty-free, non-exclusive, perpetual, irrevocable license** to exercise the rights in the Work...

### §4: 限制 (Limitations)
> Nothing in this License is intended to reduce, limit, restrict, or impose conditions on any use of the Work...

### §5: 担保 + 责任
> THE WORK IS PROVIDED "AS IS", AND THE LICENSOR DISCLAIMS ALL WARRANTIES...

---

## Apeireth workspace 实际 CC0-1.0 依赖 (transitive 关键)

| Crate | Version | 关键作用 |
|-------|---------|---------|
| **serde_derive_internals** | 0.29 | serde 内部 (transitive) |
| **toml_edit** | 0.22 | TOML 解析 (测试 fixture) |

> 完整 CC0-1.0 crate attribution 见 `THIRD-PARTY-NOTICES.md` §A.
>
> CC0-1.0 是**最宽松** license (等价 public domain), 0 限制, 0 attribution.

---

**Last-Modified**: 2026-08-06
**owner**: 整合 #3 R21 续补 (D-1)
