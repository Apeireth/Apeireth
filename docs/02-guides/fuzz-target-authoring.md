# fuzz target 编写规范（写之前先读这篇）

> `Status: ✅ 已交付（2026-10-24，输入面 fuzz 批）`
> 目的：把「写 fuzz target 时真踩过的坑」固化成规范，避免重复踩。
> 适用范围：`fuzz/fuzz_targets/*.rs`（核心输入面）与 `crates/engine/image/codecs/fuzz/fuzz_targets/*.rs`（图像面）。
> **本文每条都来自一次真实返工**，不是理论建议。

## 0. 一句话

**fuzz 的价值不在"跑起来了"，在"判据是对的"。**
一个只断言"不 panic"的 target，抓不到「放行了不该放行的值」——而后者才是安全问题。

---

## 1. 选面：先问三个问题

不是"随便找个函数 fuzz"。按顺序筛：

| # | 问题 | 为什么 |
|---|---|---|
| 1 | **它吃不受控输入吗？** | 吃网络回程、第三方进程、用户文件、LLM 生成的参数 → 值得；纯内部函数 → 风险低 |
| 2 | **它有没有"强承诺"？** | `fail-closed`、权限判定、路径限域、脱敏、剥标签 —— 这些承诺一旦被绕过就是安全事件 |
| 3 | **它接线了吗？** | **没接线也值得 fuzz** —— 在接线**之前**把边界验干净，比上线后补救便宜得多（见 `plugin_bounds`） |

实测踩过的反例：我一开始只想 fuzz "解析器"，漏掉了 `validate_path_safety` / `is_loopback_bind`
这类**判据函数** —— 它们被绕过比解析 panic 严重得多。

---

## 2. 判据：优先契约断言，其次不 panic

### 2.1 契约断言必须取自**函数自己的文档承诺**

```rust
// ✅ 好：断言取自该函数注释里写明的语义
// OutputSchema::validate 的语义 = required 字段必须存在且类型匹配
if schema.validate(&value).is_ok() {
    let obj = value.as_object().expect("Ok 时必须是结构化对象");
    let got = obj.get("f").expect("Ok 时 required 字段必须存在");
    assert!(kind.matches(got), "validate 放行了类型不匹配的字段");
}
```

```rust
// ❌ 坏：断言来自"我以为它应该这样"
// strip_tags 的语义是"移除 `<...>` 序列"，不是"移除所有尖括号"
assert!(!out.contains('<') && !out.contains('>'), "...");
// → 被 fuzz 用单字符 `>` 当场打破：strip_tags(">") == ">" 是**正确**行为
```

**教训**：自造期望 = 自造假阳性。写断言前，先把该函数的注释/文档读一遍，
断言必须能从那里推出来。

### 2.2 不要用「幂等性」当判据

单次处理语义的函数**本来就不幂等**：

```rust
strip_tags("&amp;lt;") == "&lt;"   // 反转义一次
strip_tags("&lt;")     == ""       // 再剥一次才变空
// ⇒ strip_tags(strip_tags(x)) != strip_tags(x) 是**正确**的
```

### 2.3 三类判据的价值排序

| 判据 | 抓什么 | 价值 |
|---|---|---|
| **契约断言** | 放行了不该放行的值（越权、绕过、fail-open） | 最高 |
| **不 panic** | 解析崩溃（DoS、稳定性） | 中 |
| **性能/复杂度上界** | 算法退化（ReDoS 类） | 视面而定 |

---

## 3. 硬坑清单（每条都真栽过）

### 3.1 必须写 `#![no_main]`

```rust
#![no_main]          // ← 漏了它，编译报 `main function not found`，
                      //   而 fuzz_target! 宏是**静默不展开**的，
                      //   报错信息看起来毫不相关，极难定位。
//! target 说明...
use libfuzzer_sys::fuzz_target;
fuzz_target!(|data: &[u8]| { ... });
```

**新增 target 后必查这一行。**

### 3.2 CI 必须写 `cargo +nightly fuzz`，不能只写 `cargo fuzz`

仓根 `rust-toolchain.toml` 钉了 `channel = "1.97.1"`（stable），
而 **`rust-toolchain.toml` 的优先级高于 `dtolnay/rust-toolchain@nightly` 设的 rustup default**
→ cargo 实际拿到 stable → fuzz 的 sanitizer flag 要求 nightly → 构建直接失败：

```
error: the option `Z` is only accepted on the nightly compiler
Error: failed to build fuzz script: ... -Cllvm-args=-sanitizer-coverage
```

命令行 `+nightly` 优先级最高，能压过该 pin。**这是 2026-10-24 实际挂掉 6 个 CI job 的原因。**

### 3.3 新 target 必须**同步**加进 CI 矩阵

`fuzz/Cargo.toml` 加了 `[[bin]]` 只是本地能跑；不进 `.github/workflows/fuzz.yml`
的 `matrix.target` 就等于**没接进 CI**。

**前车之鉴**：`crates/engine/image/codecs/fuzz/` 里早有 6 个 target，
但**从未被任何 workflow 跑过** —— 闲置的检测工具比没有更误导人（它让人以为有覆盖）。

### 3.4 别把测试自身的假设写进判据

| 坑 | 表现 | 修法 |
|---|---|---|
| 字节切片切在多字节字符中间 | `s.split_at(s.len()/2)` 在多字节输入上 panic | 只在 **char 边界**切 |
| 断言依赖外部工具的输出语言 | `readelf`/`git` 的报错随 locale 本地化 | 调用时固定 `LC_ALL=C`，或断言与语言无关 |
| 往**共享**目录写固定文件名 | `/tmp/outside.pdf` 与上一轮遗留物撞车 → `PermissionDenied` | 用**本次运行独有**的路径 |

**这三条都是 harness 自己的错，不是库的缺陷。**

### 3.5 修法要落在**产生问题的抽象层**

同一处 `outside.pdf` 撞车，第一版我在**测试体**里加 5 行造独有目录 → 触发
`arch_guard` 的 `size-grow`（该文件基线 6150 行，**涨 1 行即违规，0 容差**）。
第二版改成让 `workspace()` 辅助函数本身返回「独有目录下的 `ws/`」→ 测试体一行不动、
文件恰好 6150 行。**落点从使用方移到抽象层，行数与语义同时解决。**

---

## 4. 交付前自检清单

```
[ ] 1. `#![no_main]` 在文件首行
[ ] 2. 每条断言都能从被 fuzz 函数的文档/注释推出来
[ ] 3. 没有用幂等性当判据（除非该函数确实幂等）
[ ] 4. 字节/字符边界处理正确（多字节、空串、极短输入）
[ ] 5. 已加进 `.github/workflows/fuzz.yml` 的 matrix.target
[ ] 6. CI 的 run 命令是 `cargo +nightly fuzz run ...`
[ ] 7. 本地实跑过 ≥60 秒并记录读数（执行次数 / 边覆盖 / crash 数）
[ ] 8. 降级/失败路径也验过（不是只验 happy path）
```

## 5. 怎么记录读数（数字要可复核）

```bash
cd fuzz
cargo +nightly fuzz run <target> -- -max_total_time=60 -print_final_stats=1
```

交付时在 commit 正文写清：**执行次数 / 边覆盖 / crash 数 / 跑了多久 / 机器**。
不许写"应该没问题"，只写实测数字。

**另一条纪律**：若 fuzz 报了 crash，**先问是不是 harness 自己的锅**。
实测本批共抓 2 次 crash，**两次都是 harness 的错**（漏 `#![no_main]`、断言过严），
库缺陷 0 次。把误报说成成果，比没发现更坏。

---

## 6. 相关

- 工作流：[`.github/workflows/fuzz.yml`](../../.github/workflows/fuzz.yml)
- 核心输入面 target：[`fuzz/`](../../fuzz/)
- 图像面 target（仓内原有）：[`crates/engine/image/codecs/fuzz/`](../../crates/engine/image/codecs/fuzz/)
- 四级口径定义（写结论时用）：[`docs/01-architecture/system-capabilities.md`](../01-architecture/system-capabilities.md)
