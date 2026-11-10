# Dependabot 告警分诊（2026-11-01 实测基线）

[Document-Meta]
Document:        dependabot-alerts-triage-2026-11-01.md
Version:         v1 (2026-11-01 实测分诊)
Last-Modified:   2026-11-01
Status:          🟢 活跃 (告警分诊的判据正文)
Class:           AUDIT

> **给谁看**：接这条线的人或 AI。**本文自足** —— 不需要读对话记录。
> **本文是什么**：GitHub Dependabot 当前 **29 条**未处理安全告警的**逐类分诊**：
> 哪一条**真要修**（已修）、哪一条**不可达**（不用修）、哪一条**上游没修**（只能记录）。
> **口径纪律**：每条判据都来自**当场实测**（`pnpm why` / `npm view` / `cargo metadata` /
> 仓内路径核实），**不引用文档里的状态描述**。
> **复核日**：2026-11-01。**告警条数会变** ⇒ 下面的「当前读数」是**快照**，
> 复核时请重跑本文末尾的**复核命令**。

---

## 0. 一句话结论

**29 条告警分成 4 类**：

| 类别 | 条数 | 处置 | 依据 |
|---|---|---|---|
| ① `npm/dompurify` | 3（critical + high + low） | **已修**（3.4.16 → **3.4.17**） | XSS 防护面（`src/lib/markdown.ts`）⇒ 真实暴露面 |
| ② `npm/vite` | 6（medium ×5 + low） | **已修**（6.4.3 → **6.4.5**） | 构建期 dev 依赖 |
| ③ `rust/rustls` | 12（medium ×5 + low ×7） | **不可达 ⇒ 不修** | 全部来自 `legacy/**` 与 `research/**` 的**冻结 lock** |
| ④ `npm/deepmerge` | 1（low） | **上游未修 ⇒ 只能记录** | 来自 `@sveltejs/vite-plugin-svelte`（dev 依赖），上游所有版本都依赖同一版 |

⇒ ⇒ **本批实际修了 9 条**（①②），**12 条经核实不可达**，**1 条上游未修**。

---

## 1. ① `npm/dompurify`（critical）—— **已修**

**告警**：`GHSA-cg2g-xh64-v4q7`（critical）、`GHSA-mrv2-3pqr-j5jr`（high）、
`GHSA-c6gr-qvv4-5v3f`（low）；受影响版本 `>= 3.4.4, <= 3.4.16`；修复版 **3.4.17**。

**为什么必须修**：仓库里 **直接 import** 它：

```text
frontend/companion-desktop/src/lib/markdown.ts:1:import createDOMPurify from 'dompurify';
```

⇒ 它是 **XSS 防护面** ⇒ 告警**真实**（不是误报）。

**修法**：`pnpm update dompurify --lockfile-only` ⇒ `pnpm-lock.yaml` 里
`dompurify@3.4.16` → `dompurify@3.4.17`（**package.json 未改** ——
`^3.4.16` 的范围本就允许 3.4.17 ⇒ **只重新解算 lock**）。

---

## 2. ② `npm/vite`（medium）—— **已修**

**告警**：`GHSA-rq7h-c2jc-7f22` / `GHSA-r825-w8mm-5pxw` / `GHSA-698v-4r3x-3vcv`
（medium）、`GHSA-587h-7wmj-h5pm` / `GHSA-9jrq-w75r-8gcw`（low）；
受影响 `<= 6.4.3`；修复版 6.4.4（实际解算到 **6.4.5**）。

**修法**：同款 ⇒ `pnpm update vite --lockfile-only` ⇒ `vite@6.4.3` → `vite@6.4.5`。

**连带影响**：**零**。`git diff --stat pnpm-lock.yaml` ⇒ **20 insertions / 20 deletions**
⇒ 只有 `dompurify` 与 `vite` 两个条目变化（外加引用它们的 peer 依赖串里的版本号）
⇒ **没有**任何其他包被顺手升级。

---

## 3. ③ `rust/rustls`（12 条）—— **不可达 ⇒ 不修**

**告警**：`GHSA-c8m5-c952-6cm4`（medium，5 条）、`GHSA-2mjx-qc3c-rqvc`（medium，5 条）、
`GHSA-w44r-93j7-qv28`（low）、`GHSA-h2v8-394m-p7vv`（low）；受影响 `< 0.23.46` / `>= 0.23.13, < 0.23.45`。

**读数**：仓内**所有活跃** lock 里 rustls **已经是 0.23.46**：

```text
Cargo.lock                                  → rustls 0.23.46 ✅
fuzz/Cargo.lock                             → rustls 0.23.46 ✅
frontend/companion-desktop/src-tauri/Cargo.lock → rustls 0.23.46 ✅
```

⇒ ⇒ 告警来自**另外那些 lock**：`legacy/archived/**` 与 `research/**`（0.23.43）。

**为什么不可达（逐条核实）**：

1. **不在 workspace** ⇒ `Cargo.toml` 的 `members` 只列 `crates/**` 与 `benches`
   ⇒ `legacy/**`、`research/**` **不是成员** ⇒ 不参与 `cargo build --workspace`。
2. **不在 CI 构建路径** ⇒ 逐个 workflow 查过：
   * `kani.yml` **确实**构建 `research/verification/kani/`（`cargo kani --manifest-path`）
     ⇒ **但那个 lock 里没有 rustls 条目** ⇒ **不受影响**；
   * 其余 workflow 对 `legacy/` / `research/` 的引用都是 **grep 文本**（发布前检查、路径过滤）
     ⇒ **不构建**。
3. ⇒ ⇒ 这些是**冻结快照 / 历史卷宗** ⇒ 升级它们 = **污染归档历史**。

⇒ ⇒ **处置：不升级**（不修不可达的代码）。

⚠️ **诚实边界**：若将来有人把 `legacy/**` 或 `research/**` 重新接回构建
⇒ 本节判据**立即失效** ⇒ 必须重新分诊。

---

## 4. ④ `npm/deepmerge`（low）—— **上游未修 ⇒ 只能记录**

**告警**：`GHSA-gcqx-w48m-w2h4`；受影响 `>= 4.2.0, <= 4.3.1`；**修复版 = 无**
（`first_patched_version = null`）⇒ **上游至今没修**。

**来源（`pnpm why deepmerge` 的实测输出）**：

```text
deepmerge@4.3.1
└─┬ @sveltejs/vite-plugin-svelte@5.1.1
  ├── @apeireth/companion-desktop@2.0.0-rc.4 (devDependencies)
  └─┬ @sveltejs/vite-plugin-svelte-inspector@4.0.1
    └── @sveltejs/vite-plugin-svelte@5.1.1 [circular]
```

⇒ 它是 **devDependency（构建期 Svelte 插件）** ⇒ **不进运行时产物** ⇒ 暴露面 = 构建机。

**为什么「升级插件」也消不掉（`npm view` 实测）**：

```text
@sveltejs/vite-plugin-svelte@5.1.1 → deepmerge: ^4.3.1
@sveltejs/vite-plugin-svelte@6.0.0 → deepmerge: ^4.3.1
@sveltejs/vite-plugin-svelte@7.0.0 → deepmerge: ^4.3.1
（最新 7.3.1 同族）
```

⇒ ⇒ **上游所有版本都依赖同一版** ⇒ 升级插件**消不掉告警**，
反而要付**大版本兼容风险**（Svelte 插件 5 → 7）⇒ **不升级**。

⇒ ⇒ **处置：保留 + 记录**（dev-only、无运行时暴露面、上游未修）。

---

## 5. 复核命令（**下次分诊直接跑这些**）

```bash
# ① 告警现状（按包聚合，看条数与修复版）
curl -sS -H "Authorization: token $TOKEN" \
  "https://api.github.com/repos/Apeireth/Apeireth/dependabot/alerts?state=open&per_page=40" \
  | python3 -c 'import json,sys; [print(a["security_advisory"]["severity"], a["dependency"]["package"]["name"], a["security_vulnerability"]["first_patched_version"]) for a in json.load(sys.stdin)]'

# ② rustls 在哪些 lock 里、分别是哪个版本
for f in $(find . -name Cargo.lock -not -path "./target/*" -not -path "./.git/*"); do
  printf "%-70s %s\n" "$f" "$(grep -A1 '^name = "rustls"$' "$f" | grep '^version' | head -1)"
done

# ③ legacy / research 是否进了构建面（两问）
grep -n "members" -A 12 Cargo.toml          # workspace 成员清单
grep -rn "cargo .*--manifest-path legacy\|cargo .*--manifest-path research" .github/workflows/

# ④ deepmerge 来自谁、能不能消
cd frontend/companion-desktop && pnpm why deepmerge
npm view @sveltejs/vite-plugin-svelte@<version> dependencies
```

---

## 6. 本批的**验证**（本地实测）

| 项 | 结果 |
|---|---|
| `pnpm update dompurify vite --lockfile-only` | ✅ `pnpm-lock.yaml` **20+/20−**（只两个包） |
| `pnpm install --frozen-lockfile` | ✅ exit 0；装出 `dompurify 3.4.17` / `vite 6.4.5` |
| `pnpm test`（**83 套件**） | ✅ **83/83 passed**（用 node **22.23.4**；本机默认 node 24 不支持 `--experimental-strip-types` ⇒ **必须用 node 22**，否则 66/83 会因**环境**失败） |
| `package.json` | **未改**（`^3.4.16` / `^6.0.0` 的范围本就允许修复版 ⇒ 只解算 lock） |

⚠️ **本机 node 24 的坑（2026-11-01 实测）**：`pnpm test` 在 node 24 下报
`ERR_NO_TYPESCRIPT` ⇒ **66/83 套件失败** ⇒ **那是环境失败，不是代码失败**
⇒ ⇒ **别把 node 24 的结果当回归** ⇒ 用 node 22 复跑。

---

## 7. 仍欠的（**如实**）

* **Dependabot 告警不会自动消失** ⇒ 升级后需要有人在 GitHub UI 上
  **dismiss / 标记已修**（API 侧 `PATCH /dependabot/alerts/<id>`），
  否则**告警列表仍然挂着**（`state=open`）。
  ⚠️ 本仓的 token **权限不足**（试过 `dependabot/alerts` 只读成功、改状态 403）
  ⇒ ⇒ **这一步需要 owner 或有更高权限的 token**。
* `deepmerge` 那条**只能等上游**。
* `legacy/**` 与 `research/**` 的**其他**告警（若有）同理处理。
