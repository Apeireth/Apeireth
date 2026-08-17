# R241 -- failure path counter (cycle_failures_total)

## Problem
R238 \u52a0\u4e86 \untime_cycle_total\ (\u8ba1\u6b21) \u4e0e \untime_cycle_failures_total\ (\u8ba1\u9519),
\u4f46 \u540e\u8005\u4ece\u672a\u88ab inc. \u201c\u9519\u8bef\u8ba1\u6570\u5668\u4f46\u8ba1\u6570\u4e3a\u96f6\u201d = "R5 \u4e0d\u4f2a\u88c5" \u8fdd\u53cd.

\u539f run_one_cycle \u5728 dispatch_async_task / arbitration.append / group_chat \u4efb\u4e00\u5931\u8d25\u65f6\u8fd4 Err.
\u4f46\u73b0\u6709 try-catch \u4e0d\u4f1a\u8ba9\u8ba1\u6570\u5668\u53d8 1.

## Solution: split cycle body into run_one_cycle_inner()

\\\ust
pub async fn run_one_cycle(&self) -> RuntimeResult<CycleReport> {
    self.cycle_total.inc();
    match self.run_one_cycle_inner().await {
        Ok(report) => Ok(report),
        Err(e) => {
            self.cycle_failures_total.inc();
            Err(e)
        }
    }
}

async fn run_one_cycle_inner(&self) -> RuntimeResult<CycleReport> {
    // existing body (decay snapshot -> dispatch_async_task -> arbitration -> search -> group_chat -> emotion)
}
\\\

\u4e24\u5c42\u62c6\u5206\u540e:
- \u5916\u5c42\u8d1f\u8d23\u201c\u8ba1\u6570\u201d
- \u5185\u5c42\u8d1f\u8d23\u201c\u4e1a\u52a1\u903b\u8f91\u201d

## Tests (2 new tests pass)
- r241_01: \u9ed8\u8ba4 cycle \u6210\u529f\u8def\u5f84 \u2014\u2014 failures \u4ecd\u7136 0
- r241_02: metrics_text \u542b cycle_failures_total \u540d\u79f0 (\u8bc1\u660e\u8ba1\u6570\u5668\u5728\u6ce8\u518c\u8def\u5f84\u4e0a)

## Files
- \crates/apeireth-runtime/src/lib.rs\ (+1 helper fn \un_one_cycle_inner\, +2 tests)

cumulative: ~6323 tests pass.