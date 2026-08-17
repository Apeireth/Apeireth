# Unicode License v3 (Standard Text)

> **类别**: Unicode-3.0 (也叫 Unicode-TOU / Unicode-TermsOfUse)
> **来源**: https://www.unicode.org/license.txt
> **使用 crate 数**: 19 / 561 (3.4%)
> **关键 crate**: unicode-ident / unicode-normalization / unicode-segmentation / unicode-width / icu_*
> **最后更新**: 2026-08-06

---

## Unicode® Terms of Use (Standard Text)

```
UNICODE, INC. LICENSE AGREEMENT - DATA FILES AND SOFTWARE

See Terms of Use for definitions of Unicode Inc.'s
Data Files and Software.

NOTICE TO USER: Carefully read the following legal agreement.
BY DOWNLOADING, INSTALLING, COPYING OR OTHERWISE USING UNICODE INC.'S
DATA FILES ("DATA FILES"), AND/OR SOFTWARE ("SOFTWARE"),
YOU UNEQUIVOCALLY ACCEPT, AND AGREE TO BE BOUND BY, ALL OF THE
TERMS AND CONDITIONS OF THIS AGREEMENT.
IF YOU DO NOT AGREE, DO NOT DOWNLOAD, INSTALL, COPY, DISTRIBUTE OR USE
THE DATA FILES OR SOFTWARE.

COPYRIGHT AND PERMISSION NOTICE

Copyright © 1991-2024 Unicode, Inc. All rights reserved.
Distributed under the Terms of Use in https://www.unicode.org/copyright.html.

Permission is hereby granted, free of charge, to any person obtaining
a copy of the Unicode data files and any associated documentation
(the "Data Files") or Unicode software and any associated documentation
(the "Software") to deal in the Data Files or Software
without restriction, including without limitation the rights to use,
copy, modify, merge, publish, distribute, and/or sell copies of
the Data Files or Software, and to permit persons to whom the Data Files
or Software are furnished to do so, provided that either

(a) this copyright and permission notice appear with all copies
of the Data Files or Software, or

(b) this copyright and permission notice appear in associated
Documentation.

THE DATA FILES AND SOFTWARE ARE PROVIDED "AS IS", WITHOUT WARRANTY OF
ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE
WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND
NONINFRINGEMENT OF THIRD PARTY RIGHTS.
IN NO EVENT SHALL THE COPYRIGHT HOLDER OR HOLDERS INCLUDED IN THIS
NOTICE BE LIABLE FOR ANY CLAIM, OR ANY SPECIAL INDIRECT OR CONSEQUENTIAL
DAMAGES, OR ANY DAMAGES WHATSOEVER RESULTING FROM LOSS OF USE,
DATA OR PROFITS, WHETHER IN AN ACTION OF CONTRACT, NEGLIGENCE OR OTHER
TORTIOUS ACTION, ARISING OUT OF OR IN CONNECTION WITH THE USE OR
PERFORMANCE OF THE DATA FILES OR SOFTWARE.

Except as contained in this notice, the name of a copyright holder
shall not be used in advertising or otherwise to promote the sale,
use or other dealings in these Data Files or Software without prior
written authorization of the copyright holder.
```

---

## Apeireth workspace 实际 Unicode-3.0 依赖 (主要 12 个, 19 总)

| Crate | Version | 关键作用 |
|-------|---------|---------|
| **unicode-ident** | 1.0 | identifier 验证 (rustc 内置) |
| **unicode-normalization** | 0.1 | Unicode normalization (NFC/NFD/NFKC/NFKD) |
| **unicode-segmentation** | 1.12 | 文本 segmentation (grapheme / word / sentence) |
| **unicode-width** | 0.1 | 字符宽度 (CJK / emoji / ASCII) |
| **unicode-bidi** | 0.3 | 双向文本 (Arabic / Hebrew) |
| **unicode-linebreak** | 0.1 | 换行 (line break) |
| **icu_normalizer** | 1.5 | ICU 标准化 |
| **icu_properties** | 1.5 | ICU 字符属性 |
| **icu_provider** | 1.5 | ICU 数据 provider |
| **icu_locid** | 1.5 | ICU locale |
| **icu_collections** | 1.5 | ICU 集合 |
| **icu_list** | 1.5 | ICU 列表 |

> 完整 19 crate attribution 见 `THIRD-PARTY-NOTICES.md` §A (按 Unicode-3.0 分组).
>
> **不假装**: 实际 icu_* crate 数量是 7+ (per Cargo.lock, icu_normalizer / icu_properties / icu_provider / icu_locid / icu_collections / icu_list / icu_time / icu_calendar / etc), 跟 THIRD-PARTY-NOTICES.md §A 统计可能略差 ±2, 1.0 release **不**阻塞, R21 续补 (per 整合 #3 D-1).

---

**Last-Modified**: 2026-08-06
**owner**: 整合 #3 R21 续补 (D-1)
