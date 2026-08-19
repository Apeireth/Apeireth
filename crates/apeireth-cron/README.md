# apeireth-cron

> R23 cron 子模块: 计划任务声明 + 时间窗校验 + 5-field cron 解析 + @-shorthand + 月/星期别名 + 跨年闰年时间推算 (Sakamoto's algorithm)

apeireth-cron 是 Apeireth 1.0 (AGI 操作系统) 工作区 crate 之一。完整架构见 [docs/](../../docs/README.md)。

## 模块 (5)

| 模块 | 职责 | 公开 API |
|---|---|---|
| `Schedule` | 简单周期任务 (name + interval_secs) | `Schedule::new() / validate()` |
| `CronExpr` | 5-field cron 表达式 (minute hour dom month dow) | `CronExpr::parse() / matches() / is_reboot()` |
| `Field` | 单字段位图 (0..=59 / 0..=23 / ...) | `Field::parse() / parse_alias() / matches()` |
| `next_after` | 算下次触发时间 (Sakamoto's algorithm, 1 年 O(N)) | `next_after(expr, year, m, h, dom, mon, dow)` |
| `describe` | human-readable 描述 | `describe(expr)` |

## 用法

```rust
use apeireth_cron::{CronExpr, Schedule, validate_expr, describe};

// 标准 5-field cron
let e = CronExpr::parse("30 9 * * MON-FRI").unwrap();
assert!(e.matches(30, 9, 1, 6, 1));  // 周一 9:30
assert!(!e.matches(30, 9, 1, 6, 0)); // 周日

// @-shorthand (Vixie cron convention)
let daily = CronExpr::parse("@daily").unwrap();
let reboot = CronExpr::parse("@reboot").unwrap();
assert!(reboot.is_reboot());

// 月/星期别名 (case-insensitive, 3-letter prefix)
let jan = CronExpr::parse("0 0 1 JAN *").unwrap();
let tue = CronExpr::parse("0 0 * * TUE").unwrap();

// 简单周期任务
let s = Schedule::new("heartbeat", 60);
assert!(s.validate().is_ok());
```

## @-shorthand 列表 (per Vixie cron)

| Shorthand | 等价于 | 说明 |
|---|---|---|
| `@hourly` | `0 * * * *` | 整点 |
| `@daily` / `@midnight` | `0 0 * * *` | 每天 0 点 |
| `@weekly` | `0 0 * * 0` | 每周日 0 点 |
| `@monthly` | `0 0 1 * *` | 每月 1 日 0 点 |
| `@yearly` / `@annually` | `0 0 1 1 *` | 每年 1 月 1 日 0 点 |
| `@reboot` | (特殊) | 启动时一次, 不走时间表, `is_reboot()` 标识 |

## 月/星期别名 (case-insensitive)

- 月: `JAN FEB MAR APR MAY JUN JUL AUG SEP OCT NOV DEC` → 1..=12
- 星期: `SUN MON TUE WED THU FRI SAT` → 0..=6 (Sunday=0)
- 范围: `MON-FRI`, `JAN-MAR` 也支持

## 错误

```rust
pub enum CronError {
    NonPositiveInterval(i64),
    ParseError(String, String),
    FieldCountMismatch(usize),
    UnknownShorthand(String),
}
```

## 测试

- 25 unit tests (5-field cron + aliases + shorthand + backward compat) in `src/lib.rs mod tests`
- 5 scheduler tests (`#[cfg(test)] mod scheduler`)
- 25 integration tests in `tests/integration_cron.rs` (端到端, 跨字段组合 / 业务场景 / 跨年闰年)
- 总计: `cargo test -p apeireth-cron` → **68 passed** (43 unit + 25 integration)

### Integration tests 覆盖

- **@-shorthand 等价** (5): @hourly/@daily/@midnight/@weekly/@monthly/@yearly/@annually 跟 5-field 一致
- **别名 vs 数字** (4): JAN/1, JAN-MAR/1-3, MON-FRI/1-5, SUN/0 (semantic 比 `bits`)
- **真实业务场景** (5): business hours 9-17, nightly backup 2:30, heartbeat 5min, quarterly, last-Friday-of-month
- **next_after 时间推算** (7): 跨日 / 跨周 / 跨月 / 跨年 / 闰年 (2028-02-29)
- **错误恢复** (3): invalid exprs, unknown shorthand, schedule validation

## Honest (per O-5 不假装)

- Token counting 用 chars/4 近似 (无 tiktoken 依赖)
- Summary strategy 需 user-supplied callback (无内嵌 LLM)
- Marker replace 保留原始 bytes (lossless unfold)
- Semantic fold 默认 char truncation (summarizer 可注入, 无内嵌 LLM)
- `@reboot` 不是真 scheduler (需 runtime 配套)