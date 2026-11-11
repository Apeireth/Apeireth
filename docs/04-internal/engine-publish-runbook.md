# 引擎 crate 发布到 crates.io —— 执行手册（owner 侧 + 实施线）

> **决策真源**：[docs/adr/0017-engine-product-repo-split.md](../adr/0017-engine-product-repo-split.md)（Accepted 2026-10-31）
> **执行器**：[scripts/publish-engine-crates.sh](../../scripts/publish-engine-crates.sh)
> **适用范围**：引擎仓公开 crate 的**首次发布**（占名 + 建独立版本轴）
> **本文不含**：产品仓私有 crate（adapters，不发布）、crates-ui 的 npm 产物轨（另出 ADR）

---

## 1. 这一步到底在做什么（3 句话）

1. crates.io 是 Rust 生态的官方包仓库（对标 npm / PyPI），托管**源码包**。
2. 发上去之后，别人在自己的 `Cargo.toml` 写 `apeireth-core = "2.0.0-rc.5"`，`cargo build` 就自动下载编译 —— **不需要你本机有这个文件夹**。
3. 这是**分仓的前置**：产品仓若还用 `path` 依赖，就必须把引擎源码整个塞进产品仓（等于没分仓）；只有走 crates.io 版本依赖（或 git tag），产品仓才能真正"只拿版本号、不拿源码"。

**附带价值**：① **占名** —— `apeireth-*` 这个名字族发布后即归本项目（实测 25 个名字在 crates.io 全部 404 = 未被占用，零成本窗口）；② 公开引擎可被任何人发现与使用。

---

## 2. owner 侧准备（**1 件事，不可代劳**）

发布 crates.io 需要**账号 + token**，这两样只能由账号持有人创建。

> **依据**：[The Cargo Book — Publishing on crates.io](https://doc.rust-lang.org/cargo/reference/publishing.html) + [Registry Authentication](https://doc.rust-lang.org/cargo/reference/registry-authentication.html)（2026-10-31 取）。

### 2.1 注册（3 步，约 5 分钟）

| 步 | 动作 | 关键点 |
|---|---|---|
| 1 | 打开 <https://crates.io/> → **Log in with GitHub** | crates.io **没有独立注册**，用 GitHub 账号登录；**也不需要"注册组织"**（crates.io 只有个人账号 + 按 crate 分配的 owner，见 §2.3） |
| 2 | 登录后**必须验证邮箱**：<https://crates.io/settings/profile> 填邮箱 → 收信点确认 | Cargo Book 明确要求：未验证邮箱不能发布 |
| 3 | 创建 token：<https://crates.io/settings/tokens> → New Token | 名字建议 `apeireth-engine-first-publish`；scopes 至少 `publish-new` + `publish-update`（要能撤回旧版再加 `yank`）；⚠️ **token 只显示一次**，离开页面再也看不到 |

### 2.2 本机配置（两种方式，任选其一）

```bash
# 方式 A：cargo login（写入 ~/.cargo/credentials.toml）
cargo login --registry crates-io
# 提示 "please paste the API Token ..." 时粘贴 token，回车

# 方式 B：环境变量（不想落盘时用；cargo:token provider 读的就是这个）
export CARGO_REGISTRIES_CRATES_IO_TOKEN='<token>'
```

⚠️ **本机注意**：`~/.cargo/config.toml` 用 rsproxy 镜像替换了 `crates-io`。若方式 A 报错或把 token 写到镜像名下，改用方式 B，并确保发布命令始终带 `--registry crates-io`（执行器已固定带）。
⚠️ **诚实标注**：以上两种方式**在本机均未实测**（当前无 token）；首个 `cargo publish --dry-run --registry crates-io` 成功即代表 token 生效。

**撤销**：<https://crates.io/settings/tokens> 随时 revoke。建议**首发完成后立即 revoke 一次性 token**，后续改用 CI 的 trusted publishing（见 §6）。

**开工前自查**：
```bash
cargo search apeireth --limit 5     # 期望 0 命中（名字还没被占）
```

### 2.3 团队归属的正确形态（crates.io 没有"组织账号"）

- 先用**个人账号**发布第一个 crate；
- 再把 GitHub 团队加成 owner：`cargo owner --add github:Apeireth:<team名> apeireth-core`；
- **team owner 权限受限**：能发布 / yank，但**不能加删 owner**（这是 crates.io 有意的安全设计）；`github-handle` 形式的 named owner 才有加删 owner 的权限；
- 前提：crates.io 需要 GitHub 的 `read:org` 授权，且 GitHub 组织未屏蔽第三方 OAuth app，否则会报 `could not find the github team org/repo`。

### 2.4 三条硬事实（Cargo Book 原文口径）

| 事实 | 含义 |
|---|---|
| 发布**永久** | 版本不能覆盖、代码不能删除；唯一补救是 `cargo yank`（yank 也**不删代码**，只是让新依赖解析不到该版本） |
| **`.crate` 上限 10 MB** | 本仓最大的包是 `apeireth-tools-canonical` 3.6 MiB（未压缩，静态审计值）⇒ 安全 |
| 名字**先到先得** | 一旦被占不能给别人用 ⇒ 按拓扑序边发边占 |

---

## 3. 跑法（三条命令，按顺序）

```bash
# ① 看拓扑序（离线, 不需要 token）—— 确认无成环
./scripts/publish-engine-crates.sh --print-order

# ② 离线门禁自检（不需要 token）：每个 crate 走 G2 cargo package --no-verify
./scripts/publish-engine-crates.sh --check

# ③ 真发（需要 token）：逐个走 G2 → G3 dry-run → 真发
./scripts/publish-engine-crates.sh
```

**续跑**（中途失败后从某个 crate 继续）：`./scripts/publish-engine-crates.sh --from apeireth-core`

---

## 4. 拓扑序与三段门禁

**顺序由脚本现算**（读 `cargo metadata`，含 dev/build 依赖约束），不手抄。原因：**dev-dependency 也是发布顺序约束**（实测，见 ADR-0017 附录 C.2 B3）——`orchestration` 曾因 2 条 dev-dep 与 `plugin`/`provider` 成环，**导致 18 个 crate 排不出顺序**；该环已在 S2a 拆除。

**每个 crate 三段门禁**：

| 段 | 命令 | 拦什么 | 网络 |
|---|---|---|---|
| G1 | `cargo package -p <c> --list` | 打包内容清单 | 离线 |
| **G2** | `cargo package -p <c> --no-verify` | **path 依赖缺 version / 依赖未上架 / dev-dep 成环** | 离线 |
| **G3** | `cargo publish -p <c> --dry-run --registry crates-io` | 含 build verify | 联网 |

⚠️ **不能把 31 个 dry-run 全跑完再统一发**：G2/G3 会向 crates.io 解析内部依赖 ⇒ 必须按拓扑序**逐个发一个再发下一个**。

---

## 5. 出问题怎么办

| 现象 | 成因 | 处理 |
|---|---|---|
| `crates-io is replaced with non-remote-registry source registry` | 本机 `~/.cargo/config.toml` 用镜像替换了 crates-io | 脚本已固定带 `--registry crates-io`；**这个错长得像网络错** |
| `all dependencies must have a version requirement specified when packaging` | path 依赖缺 version（B1） | 根 `[workspace.dependencies]` 声明 `{ path, version }`，crate 用 `{ workspace = true }` |
| `dependency X not found on registry` / 找不到版本 | 该依赖还没发布（发布顺序问题） | 按 `--print-order` 的顺序推进；**不要**改 `publish = false` 掩盖 |
| `cyclic package dependency` | dev-dep 成环（B3） | 拆 dev-dep；本仓已处理（`council_live` 迁到产品侧 CLI crate） |
| 工作树不干净 | 有未提交改动 | 默认**拒绝发布**（发布内容会与 git 状态不一致）。先提交，或 `--allow-dirty` 并知情 |
| 发布被拒（审核） | crates.io 新 crate 人工审核 | 等审核（通常数小时到数天）；**已发出的版本不可删除**，只能 yank |

**已发布版本的唯一补救 = yank**（标记弃用）：crates.io 上没有"删除版本"这个动作。

---

## 6. 诚实边界（必读）

- 🚪 **单向门**：crates.io 上已发布的版本**不可删除**，只能 yank。⇒ 必须按序逐个发，每个先 dry-run。
- 🔍 **人工审核**：新 crate 首次发布有人工审核环节，可能延迟数小时到数天。
- 📦 **只收源码**：crates.io 不收编译产物；crates-ui 的 wasm 产物走 npm（`@apeireth/ui`），**不在本批**。
- 🔐 **首发用 token**：实测 cargo 1.97.1 内建 provider 只有 `cargo:token` 与 `cargo:paseto`，**无 `cargo:oidc`**；crates.io 的 trusted publishing 是否 GA **未查证**（官方文档页为 JS 渲染，未取到正文）⇒ 首发用 token，**1.0 后转 CI trusted publishing，开工前先核对官方文档**。
- 📌 **预发布版本**：workspace.version 现值 `2.0.0-rc.5` 是预发布号，crates.io 接受；依赖方写 `=2.0.0-rc.5` 精确锁（RC 期不用 `^`）。
- 🧹 **工作树当前不干净**（本仓存在其它施工线的未提交改动）⇒ 真发前须先确认要发布的源码状态。

---

## 7. 发布完成后

1. `cargo search apeireth --limit 40` 确认名字族已可见。
2. 用一个空目录做**下游消费实测**（真下载 + 编译），证明"别人能真的用起来"：
   ```bash
   cargo new /tmp/apeireth-smoke && cd /tmp/apeireth-smoke
   cargo add apeireth-core@2.0.0-rc.5 && cargo build
   ```
3. **revoke 一次性 token**。
4. 记入 ADR-0017 §9 实施记录，并把 crates.io 链接补进公开仓 README（owner 面版）。

---

_本文由仓库治理线（总工：lead）撰写于 2026-10-31，与 ADR-0017 同步。_
