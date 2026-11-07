[Document-Meta]
Document: docs/04-internal/01-handoffs/memory-real-query-gate-proposal-2026-11-07.md
Class: DECISION
Status: 🟡 待裁决
Last-Modified: 2026-11-07
Author: deputy

# **真实查询闸门接入 CI 的方案**（待裁决）

> **选项**：**A** 新增独立 workflow（带缓存）· **B** 并进 `golden-baseline.yml`（分步报告）· **C** 只在本地跑
> **建议**：**A**（独立 workflow + 缓存 + 只在记忆 crate 改动时触发）
> **理由**：两个闸门**职责不同**（模板卷守「不许退步」、真人卷守「不许自欺」），
> **绑在一起会让失败原因难辨** ⇒ ⇒ 而闸门一旦失败原因难辨，就会被人绕过。
> **代价**：语料 **700MB**（对比小说语料 4MB ⇒ **175 倍**）⇒ ⇒ 必须带缓存。

## 一、**要裁决什么（一句话）**

**要不要为「真人查询闸门」新增一个 CI workflow**（它需要下载 **700MB** 语料）
⇒ ⇒ 这是 **CI 成本 vs 测量可信度** 的取舍，**属于治理决策**。

## 二、**已经落地的部分（不需要裁决，已完成）**

| 交付物 | 状态 |
|---|---|
| `scripts/check_real_query_baseline.py` | ✅ 方向闸门（相对劣化 > 1% ⇒ 判红）|
| `scripts/test_real_query_baseline.py` | ✅ 闸门自测（6 项，防闸门本身被改坏）|
| `scripts/build_real_query_exam.py` | ✅ 考卷构建脚本（**逐字可复现**，实测哈希一致）|
| `scripts/real-query-baseline.json` | ✅ 基线（recall@10 = 0.7246）|
| **本地 `gates.sh`** | ✅ 已接（每次提交前会跑）|
| `scripts/gate-manifest.json` | ⚠️ **由 workflow 生成** ⇒ ⇒ 我的闸门**不在清单里**（因为还没有 workflow）|

⇒ ⇒ ⇒ **本地每次提交前都会跑**（`gates.sh` 硬编码清单 + 我加的两步）
⇒ ⇒ ⇒ **CI 上还没跑**（缺 workflow）。

## 三、**闸门「真的有牙」的实测证据**

把基线改成 0.99 ⇒ ⇒ **相对变化 −26.80% ⇒ 退出码 1（拦下）**
⇒ ⇒ 恢复后 **+0.00% ⇒ 退出码 0** ⇒ ⇒ **不是装饰**。

## 四、**如果要在 CI 上跑，需要什么（照 `golden-baseline.yml` 的做法）**

⚠️ **语料在仓外**（MIRACL 中文 dev，约 **700MB**）⇒ ⇒ CI 上必须先下载。

```yaml
# 建议：新建 .github/workflows/real-query-ratchet.yml（结构照 golden-baseline.yml）
on:
  push:
    branches: [master, main]
    paths:
      - 'crates/engine/memory/**'
      - 'scripts/check_real_query_baseline.py'
      - 'scripts/real-query-baseline.json'
      - 'scripts/build_real_query_exam.py'
  pull_request:
    paths: [同上]
  workflow_dispatch:
jobs:
  real-query:
    runs-on: ubuntu-latest
    timeout-minutes: 30
    steps:
      - 下载语料（见 §五）
      - 构建考卷: python3 scripts/build_real_query_exam.py
      - 跑闸门: python3 scripts/check_real_query_baseline.py --check
```

⚠️ **两个必须遵守的既有惯例**（照 `golden-baseline.yml`）：
1. **下载失败 ⇒ 门禁如实失败**（不许静默跳过：那等于「没检查」）；
2. **CI 磁盘瘦身**：`CARGO_PROFILE_{DEV,TEST}_DEBUG=0` + `CARGO_PROFILE_{DEV,TEST}_STRIP=symbols`
   （实测：默认 101GB / 瘦身后 7.5GB ⇒ runner 只有 14GB ⇒ 不瘦身装不下）。

## 五、**成本（三条，要一起考虑）**

| 项 | 量级 | 说明 |
|---|---|---|
| 语料下载 | **~700MB** / 次 | 对比：小说语料只有 ~4MB ⇒ **175 倍** |
| 语料构建 | ~15s | 流式扫 10 个分片 ⇒ 单核可跑 |
| 跑批 | ~40s | 393 题 × 22862 段 |
| 磁盘 | ~10MB（考卷）+ 700MB（原始分片）| 构建后可删原始分片 |
| **缓存** | 可缓存 700MB | GitHub Actions cache ⇒ 第二次起近乎免费 |

⇒ ⇒ ⇒ **我的判断**：**值得，但必须带缓存**（否则每次 CI 都下 700MB）
⇒ ⇒ ⇒ ⇒ 且**只在改动记忆 crate 时触发**（照 `golden-baseline.yml` 的 `paths`）。

## 六、⭐ **不新增 workflow 的替代方案**

**方案 B：并进已有的 `golden-baseline.yml`**（同一个 job 里多一步）
- ✅ 不用新增 workflow（清单生成器会自动认，因为它从 workflow 抽）
- ✅ 语料下载可复用同一个 job 的缓存目录
- ⚠️ **缺点**：两个闸门绑在一起 ⇒ ⇒ 真人卷的失败会挡住模板卷的通过（或反之）
  ⇒ ⇒ 需要**分步 + 各自报告**（step 级别）

**方案 C：先只在本地跑**（现状）⇒ ⇒ 代价：**CI 上不会拦住「真实查询方向错了」**

## 七、**成本-收益对比（我的建议）**

| | 收益 | 成本 |
|---|---|---|
| **A 新增 workflow** | 方向错在 CI 就红 | 700MB 下载（需缓存）+ 治理新增一个 workflow |
| **B 并进 golden-baseline.yml** | 同上，且不新增 workflow | 两闸门耦合；需分步报告 |
| **C 只在本地** | 零 | CI 拦不住方向错 |

**我建议 A**：理由是**两个闸门的职责不同**，**绑在一起会让失败原因难辨**
⇒ ⇒ 而闸门一旦失败原因难辨，就会被人绕过。

## 八、**我这次没有做什么**
- **没有**新增/修改任何 `.github/workflows/*.yml`（**那是治理决策，等你拍板**）；
- **没有**改 `first_pos` 的默认值（**仍是 0.0 = 关**）
  ⇒ ⇒ **闸门进 CI 之前，那个 +12% 的开关就该保持关着**（否则等于没尺子就放大招）；
- **没有**把 700MB 语料打包进仓库（**数据不入库**，与小说语料同一惯例）。

## 九、**复现命令**

```sh
# 跑真人查询闸门（需要语料已构建）
APEIRETH_DATA_DIR=/home/bai/.apeireth python3 scripts/check_real_query_baseline.py --check

# 闸门自测
python3 scripts/test_real_query_baseline.py

# 从原始数据重建考卷（逐字可复现）
python3 scripts/build_real_query_exam.py --miracl-dir ~/miracl
```
