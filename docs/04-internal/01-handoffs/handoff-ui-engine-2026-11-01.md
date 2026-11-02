# 引擎线交接（2026-11-02 更新；初版 2026-11-01 收尾）

> **状态**：本批**已收尾**、工作树干净、成果全部推到远端。
> **下次从这里继续**：§3「下一次开工的四件事」按顺序做，每件都已是**可执行**的（不是愿望）。
> 谁该看：接手引擎线的人 + 决定优先级的人。

---

## §1 现状（一手事实，可复核）

| 项 | 值 |
|---|---|
| 分支 | `ui-engine-p0-2026-10-24` |
| 本地 HEAD | `d83d6179`（= 远端 HEAD） |
| 工作树 | **干净**（`git status` 无改动） |
| 主 worktree | `/home/bai/Projects/Apeireth-rust`（**另一条线在用**，有 68 个未提交改动 ⇒ **不要动**） |
| 本线 worktree | `/home/bai/Projects/apeireth-ui-wt2`（分支 `ui-engine-w0`，与远端分支**同内容**） |
| 台账 | `docs/04-internal/ui-engine-defect-ledger.md`（E-30…**E-153**） |

### 本批已闭的口子

- **C2 闭合**：真实 Windows + WebView2 引擎上五段能力探针**全过**
  （GitHub run **41266497534**，2026-11-01）⇒ 详见台账 **E-144**。
- **合并进主线**：本线 278 提交合进主线（`65fd7261`）⇒ 详见 **E-146**。
- **arch_guard 复核闭合**：合并引入的 5 个 W1 = 本线历史「API 重建快照」把别人的线
  带了进来 ⇒ 还原主仓路径 ⇒ **arch_guard 0 violations** ⇒ 详见 **E-147**。
- **红邮件洪水修法进了 main**（PR #110）⇒ 详见 **E-148 / E-150 / E-152**。
- **性能归因排掉 6 个候选**（全部负结果入库）：读 DOM / 重复遍历（1.12 倍）/
  子树克隆（≈8%）/ 属性 `String` 分配 / class token 分配 / **wasm⇄JS 边界**
  ⇒ 详见 E-141…E-151、**E-153**。

---

## §2 本批最重要的两个「认知更新」（别踩回去）

1. ⭐ **瓶颈不在 wasm⇄JS 边界**。浏览器 84 ms / native **143~150 ms**（同一棵树）
   ⇒ ⇒ **native 反而更慢** ⇒ ⇒ 成本在 **Rust 侧 / 后端实现**。
   ⇒ ⇒ 别再按「减少跨界」的思路优化。
   （探针：`cargo run --release -p apeireth-ui-dom --example patch_cost_probe`）

2. ⭐ **跨构建路径 / 跨时段的计时不可比**。曾差点把「vite 路径 82 ms vs 早先 95 ms」
   当成 **-14% 收益**，同口径重测后基线也是 84 ms ⇒ ⇒ **收益不存在**。
   ⇒ ⇒ 判定纪律：**同路径 + 同时段 + 多次取中位数 + 丢首次**（抖动可达 10 倍）。

---

## §3 下一次开工的四件事（按顺序，都已是可执行状态）

### ✅ 已完成（2026-11-02）：段级计时插桩 —— E-154

已交付：`SegmentTimings`（7 段）+ `Renderer::set_segment_clock()` + `seg!` 宏
（可插拔时钟，**默认关闭 ⇒ 零开销**）。

**结论**：`patch_children` **自身逻辑占 75.4%**，直接段合计仅 24.6%
（⚠️ `children` 段含递归 ⇒ 84.1% **不可直接读**，正确读法见探针输出与 E-154）。

### ① ⭐ 定位 `patch_children` 自身体内的**具体语句**（下一批最该做）

**为什么**：E-154 已把大头缩到 `patch_children` 自身（75.4%），但**还没定位到语句**。
**已缩小到 4 个嫌疑**（按可能性排序）：
1. ⭐⭐ **`first_dom_node(&mounted)` 每 child 调一次** —— 若它**向下走子树**
   找第一个 DOM 节点 ⇒ **O(子树) × 每 child ⇒ 二次方**（**最强嫌疑**）；
2. 每元素 **3~4 次全扫 children**（`key_ref` / `has_vnode` 计数 / `key` / `has_key`）；
3. `patch_positional_children` 的 `Vec` 构建；
4. ⚠️ `new_element.clone()` —— **已排除**（`rebuild` 单独计时 = 10.5%）。
**怎么做**：先给 `first_dom_node` 加调用计数（或临时段计时）⇒ 确认是否二次方。
**验收**：`compared_nodes` / `written_nodes` **不得变差**；patch 时间按同口径 A/B 下降。

### ② native 侧 `Fragment` 节点计数口径

**现象**：native 数出 37,458 节点，浏览器 `querySelectorAll('*')` 也 ≈ 37,477
（**已基本一致**）⇒ ⇒ 但 E-140 记的「native 8×4 = 70,226」与浏览器差 ~2 倍。
**怎么做**：确认 `InMemoryDom::children` 是否对 `Fragment` 展平后**重复计入**
⇒ 若是 ⇒ 修计数函数并更新 E-140 的口径注记。

### ③ E-54 data 面 0.0626% 的定向修

**现状**：已排除特异性 / 层叠顺序 / 分数相位 / 抽取失真，收敛到
**渐变·阴影插值 1~5 LSB**（判据按**有界像素预算** ≤0.1% 判定为**通过**，criterion A）。
**怎么做**：只需在台账把「已按 criterion A 通过 + 残留成因」写清；
若要追到 0 ⇒ 需改插值实现（**属于 L3 排除项之外的边界判断，先问 owner**）。

### ④ C2 的另一半：Tauri 内嵌 WebView2 的**宿主集成**

**现状**：C2 的「引擎兼容性」这半**已闭**（E-144）；
「Tauri 应用内嵌 WebView2 跑起来」这半 **未验**。
**怎么做**：需要**真实 Tauri 应用**（不是 headless Edge）⇒ ⇒ 属新装置，
⇒ ⇒ 开工前先确认 owner 是否要现在投入。

---

## §4 环境与命令（照抄即可跑）

```bash
export PATH=/home/bai/.local/node22/node_modules/node-linux-x64/bin:/root/.cargo/bin:$PATH
export CARGO_TARGET_DIR=/home/bai/Projects/apeireth-ui-target
export RUSTC_WRAPPER=            # sccache 坏了，必须清空

cd /home/bai/Projects/apeireth-ui-wt2/crates-ui
cargo test --workspace --locked --offline                    # 期望 1136 passed / 0 failed
cargo clippy --workspace --all-targets --locked --offline -- -D warnings   # 期望 0
cargo clippy --workspace --all-targets --target wasm32-unknown-unknown \
  --locked --offline -- -D warnings                          # 期望 0（CI 门禁）
cargo run --release -p apeireth-ui-dom --example patch_cost_probe          # 性能探针
```

**提交**：`pwsh scripts/commit.ps1 -MessageFile /home/bai/mNNN.txt`
（正文必带 0 装真账 + O-6 三阶审查，否则 hook 直接拒）。

**推送**：`/home/bai/Projects/apeireth-ops/push.sh HEAD refs/heads/ui-engine-p0-2026-10-24`
- ⚠️ 网络间歇不通 ⇒ 会重试 5 轮；仍失败就**过几分钟再来**。
- ⚠️ 若报 **非快进** ⇒ 说明别的线推了东西 ⇒
  `git rebase origin/ui-engine-p0-2026-10-24`（对象通常已在共享 `.git` 里，
  不必先 fetch）⇒ 再推。

---

## §5 纪律提醒（照旧）

- **0 触碰 LOCKED 5 项**（9 哲学锚本体 / 键集 14 键 / 3 不可变脊柱 / workspace.version / R11 baseline）。
- **crates-ui 保持 0 外部依赖**（wasm-only 依赖放 `[target.'cfg(target_arch = "wasm32")']`）。
- **能力缺口与缺陷分账**；**负结果同样入台账**；**任何结论必须可复跑**。
- **不在主 worktree 里动手**（那是另一条线）。
