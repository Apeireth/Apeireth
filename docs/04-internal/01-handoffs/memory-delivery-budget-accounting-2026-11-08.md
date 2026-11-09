[Document-Meta]
Document: docs/04-internal/01-handoffs/memory-delivery-budget-accounting-2026-11-08.md
Class: EVIDENCE
Status: 🟢 活跃
Last-Modified: 2026-11-08
Author: deputy

# 送达预算**计费口径**修复（2026-11-08）—— 长记录不再被整条静默丢掉

## 〇、一句话结论

> ⭐⭐⭐ **一个静默的「全部丢失」缺陷已修**：
> 检索管线的字符预算原本按候选的**原始正文长度**计费
>；一条 7,400 字的整章记录在 4,000 字预算下**一条都装不下**
>；而当**所有**候选都这样时，**整批候选被静默丢弃**
>；不报错、不告警、模型一条也收不到。
>
>；**修法**：预算按**送达量**计（= `min(原始长度, 生产单条跨度 400)`）
>；+ **保底装一条**（任何时候都不返回空集）。
>
>；**两把尺子逐字不变**：内部卷基线 **Δ +0.0000%** · 真人卷闸门 **Δ +0.0000%**。

## 〇之二、四级口径声明

| 项 | 口径 | 依据 |
|---|---|---|
| 送达预算计费 | **PRODUCTION WIRED**（DEFAULT ENABLED）| 修的是**生产路径**（`diversity::select` + `coordinator` 召回循环）|
| 反证测试 | **IMPLEMENTED**（测试，非生产）| `crates/engine/memory/tests/delivery_budget_accounting.rs`（4 例）|
| 缺陷定位 | **EVIDENCE**（并行线实测）| `docs/04-internal/01-handoffs/memory-delivery-loss-decomposition-2026-11-08.md` |

> ⭐ **没有任何一项达到 `HARDWARE VALIDATED`**（本仓为软件本体，无硬件路径）。

## 一、缺陷（**并行线实测定位**，本线落地）

`crates/engine/memory/src/retrieval_pipeline/diversity.rs::select` 原本这样计费：

```rust
let content_chars = candidate.content.chars().count();   // 原始正文长度
if chars + content_chars > max_chars {
    continue;                                             // 整条跳过
}
```

而生产注入走**锚定选段**（`AnchoredInjectionConfig::PRODUCTION.span_chars = 400`）
； **实际只送达 400 字**；按 7,400 字计费就是**把一条只送 400 字的东西当 7,400 字算**。

； **两个后果**（并行线实测）：
1. **内部卷**：候选全是整章（7,400 字）； 一条都装不下；**平均入选 0.00 条、送达率 0.0000**；
2. **真人卷**：候选是段落级（几百字）； **不受影响**（这正是它长期没被发现的原因）。

### 1.1 为什么「静默」比「丢分」更严重
选不中只是召回差；**静默全失**是「模型一条也收不到，而且没有任何报错」
； 生产上表现为「记忆功能好像坏了」，但**日志/指标里什么都不显示**。

### 1.2 ⚠️ 同一个错误在**两处**
| 位置 | 修复前 | 修法 |
|---|---|---|
| `retrieval_pipeline/diversity.rs::select` | 按原始长度计费，**整条跳过** | 按送达量计费 + **保底装一条** |
| `coordinator.rs` 召回循环 | `total_chars + cand.content.len() > max_chars`，**break** | 同样按送达量计费 |

； **只修一处不够**：修好 `select` 后，协同器仍会按原始长度 `break`；仍然只送一条。
； **两处必须同时改**（这是本笔的实测结论）。

## 二、修法

### 2.1 计费口径：**按送达量**

```rust
/// 单条候选**实际送达**的字符数（= 预算计费口径）。
pub(super) fn delivered_chars(content: &str) -> usize {
    let span = crate::anchor_injection::AnchoredInjectionConfig::PRODUCTION.span_chars;
    content.chars().count().min(span)
}
```

； **单一真源**：跨度直接引用 `AnchoredInjectionConfig::PRODUCTION.span_chars`
； **不复制常量**（避免两处 400 各改各的）。

### 2.2 为什么取「锚定跨度」而不是「披露层跨度」
披露层（XML / 头截形态）每条上限是 `per_source_budget_chars(total)`（4,000 总预算下 **1,000**），
**大于**锚定跨度 400；取**较小**的 400 是**保守**的
； **两种形态下都不会超额送达**。

### 2.3 保底：**任何时候都不返回空集**

```rust
if chars + content_chars > max_chars {
    if !result.is_empty() {
        continue;
    }
    // 一条都没装下时，先装这一条（与下游「第一条总是装得下」语义一致）
}
```

； 与协同器原有的 `&& !final_items.is_empty()` 语义**逐字一致**
； 即便预算小到装不下任何一条，也**至少送一条**（披露层会再截断它）。

## 三、⭐ 反证（**这是本笔最重要的部分**）

`crates/engine/memory/tests/delivery_budget_accounting.rs`（4 例）：

| 用例 | 量的是什么 |
|---|---|
| `long_records_are_not_silently_dropped` | 全是长记录时**一条都不许丢**（修前：空集）|
| `delivered_total_stays_within_budget` | 按送达量计费，**总送达量不许超预算** |
| `a_tiny_budget_still_admits_one` | 预算只有 9 字时**保底装一条**（修前：空集）|
| `short_records_keep_their_previous_behaviour` | **短记录的既有行为逐字不变**（这是修复不能破坏的那一半）|

**反证实测（把计费改回「按原始长度」+ 去掉保底 = 修复前的原始逻辑）**：

```
test long_records_are_not_silently_dropped ... FAILED   (空集)
test a_tiny_budget_still_admits_one ... FAILED           (空集)
test delivered_total_stays_within_budget ... ok
test short_records_keep_their_previous_behaviour ... ok
 2 failed / 2 passed
```

； ⭐ **两个「缺陷在不在」的用例变红，两个「不变式」的用例仍绿**
； **证明修复只改掉了缺陷，没动别的**。

## 三之二、⭐ 一处**我自己改错又被闸门抓住**的地方（留痕）

### 发生了什么
我加的那条「**保底：绝不返回空集**」把并行线的**反证**弄红了：

```
the_old_raw_length_billing_drops_every_long_record ... FAILED
  "旧口径（原始正文计费）下 7,400 字的记录在 4,000 字预算里一条都装不下
   —— 这就是静默全失的成因（此断言若红，说明旧口径已被改掉，反证需重做）"
```

### 根因
并行线的 `candidate_span_chars = 0` 是**文档承诺的「旧口径 / 逃生阀」**
（`0` 表示按原始正文计费，即改造前的行为）。
我的保底让「全丢」变成「送一条」，于是**逃生阀不再是旧口径**
，那条反证量到的就不是旧行为了。

### 改法（**保底限流到正常路径**）
```rust
if !result.is_empty() || ranking.candidate_span_chars == 0 {
    continue;
}
```
，`span == 0`（旧口径）**逐字保持原样**，含「全丢」这个病理；
，`span > 0`（生产默认 400）才走保底。

### 为什么这样分
| 路径 | 有没有「全丢」的洞 | 该不该保底 |
|---|---|---|
| `span == 0`（旧口径/逃生阀）| **有**（就是被修掉的那个缺陷）| **不该**，它必须与改造前逐字一致，否则反证失效 |
| `span > 0`（生产默认）| **有**（计费改成「选后长度」后**新出现**的洞：`max_chars < span` 时全丢）| **该**，这是本笔真正要堵的 |

，**反证失效比少一条保底更糟**（教训：不许为了让自己的改动显得有用而破坏别人的反证）。

### 结果
* `apeireth-memory --test candidate_budget_billing` => **8 passed / 0 failed**（含那条反证）
* `apeireth-memory --test delivery_budget_accounting` => **4 passed / 0 failed**

### 教训（新增）
* **(y)** **「逃生阀」的值必须逐字复现旧行为**，包括旧行为的**病理**；
  否则所有用逃生阀做的反证都会**静默失效**（它们会显示成「反证不成立」，而不是「口径被改了」）。
* **(z)** 加「保底 / 兜底」这类**防御性分支**时，先问「旧路径上有没有同样的分支」，
  旧路径上没有就说明它**不是旧行为的一部分**，要么加开关隔离，要么别加。

## 四、⭐ 两把尺子（**逐字不变**）

| 尺子 | 结果 |
|---|---|
| 内部卷基线闸门（`check_golden_baseline.py --check`）| **OK · Δ +0.0000%**（redchamber 0.5305 / watershed 0.7319）|
| 真人卷闸门（`check_real_query_baseline.py --check`）| **OK · Δ +0.0000%**（0.7246）|
| 送达率（观察项）| redchamber 0.3881 / watershed 0.7218 —— **Δ +0.0000%**（闸门量的是**头截**形态，本笔修的是**锚定**形态，两者是不同的形态，数字不动是**预期**的）|

； ⚠️ **口径提醒**：闸门量的「送达率」是**头截形态**；
本笔修的是**锚定形态**（生产 CLI 形态）； **两个数不是同一个数**
； **修好的收益要用锚定形态量**（并行线的 `delivery_loss_decomposition` 才有那个数）。

## 五、影响面

| 项 | 影响 |
|---|---|
| 生产行为 | **变好**：长记录不再被整条丢掉；内部卷的送达从 **0.0000** 恢复到量级 |
| 改动文件 | `retrieval_pipeline/diversity.rs`（**未登记棘轮**，可增）· `coordinator.rs`（棘轮 2346，**行数不变**）· `retrieval_pipeline.rs`（棘轮 1220，**行数不变**）|
| 棘轮 | 两个登记文件**行数逐字不变**（2346 / 1220）； `arch_guard --strict` 不受影响 |
| 短记录 | **行为逐字不变**（反证用例钉住）|
| 闸门 | 判定逻辑**零变更** |
| 基线文件 | **零变更** |
| 外部依赖 | **零新增** |

## 六、⭐ 边界 / 没测什么

1. ⭐ **没有测 CPU 成本**：长记录现在会进候选集；**打分的候选数变多**
  ；但 `select` 本来就把**全部候选**算了一遍分；**多出的成本在候选生成侧，不在选择侧**
  ；**未实测**，若要上线前定论需补测；
2. ⭐ **没有测多路融合后的送达**（本笔只改单路 `select` + 协同器计数）；
3. ⭐ **没有测「锚定找不到主体」的退化路径**（`find_subject_window` 找不到时保留原文
  ；此时送达量会是**原文长度**；保守方向，不会超额）；
4. ⭐ **真实增益的量级**以并行线的 `delivery_loss_decomposition` 为准
   （本笔的闸门数字量的是另一种形态，动不了）。

## 七、复现命令

```bash
export HOME=/home/bai APEIRETH_DATA_DIR=/home/bai/.apeireth TMPDIR=/home/bai/tmp-test RUSTC_WRAPPER=""

# 反证测试（4 例）
cargo test -p apeireth-memory --test delivery_budget_accounting

# 两把尺子
python3 scripts/check_golden_baseline.py --check
python3 scripts/check_real_query_baseline.py --check

# 送达形态分解（并行线；**锚定形态的收益在这里**）
cargo run -q -p apeireth-memory --example delivery_loss_decomposition
```
