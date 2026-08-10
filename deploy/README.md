# deploy（计划目录）

```
[Document-Meta]
Document: deploy/README.md
Version: Manual-Rev-G + R14 (占位目录)
R-Cycle: R14
Last-Modified: 2026-07-31
Status: 🟡 占位（阶段 7+ 真正施工时再创建 Dockerfile + compose + k8s）
```

> **状态**: 🟡 **占位** — 阶段 7+ 真正施工时再创建 17 crate 部署配置。
>
> **为什么现在不存在内容**：
> - 阶段 7 才是"前端设计 + 部署配置"（另起团队负责，不在本施工团队范围）
> - 当前 = 顶层 README 占位，**没有真正部署** 配置
> - 真正施工时由施工团队按 `stage5-construction-document.md §7 §8` 创建
>
> **未来部署配置**（设计层 LOCKED）：
> - **17 crate 各 1 个 Dockerfile**（apeireth-core / perception / cognition / ... / cli）
> - **1 个 master Dockerfile**（orchestrator / supervisor）
> - **docker-compose.yml**（开发环境）
> - **k8s deployment yaml**（生产环境）
> - **CI/CD pipeline**（GitHub Actions 扩展：nightly + coverage + benchmark）
>
> **详见**：
> - `docs/stage5/stage5-construction-document.md §7 §8`
> - `APEIRETH-FINAL-CHECK-2026-07-31.md`

---

_本目录为占位（owner: 施工团队，阶段 7+ 真正施工时再创建 17 + 1 Dockerfile + compose + k8s）._