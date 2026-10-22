# 安全线重启批收口报告（S1→S3 · 2026-10-21/22）

```text
[Document-Meta]
Document:        docs/04-internal/backend-enhancement/security-restart-closeout-2026-10-22.md
Version:         1.0
Last-Modified:   2026-10-22
Status:          🟢 交付记录
Author:          安全线 lead
```

> 入口：重启简报 [restart-brief-security-2026-10-21.md](../../architecture/restart-brief-security-2026-10-21.md) + [ADR-0010](../../adr/0010-r8-governance-pipeline.md) + [r8-b-entry-convergence-design.md](../../architecture/r8-b-entry-convergence-design.md)。愿景锚 = [founding-design-v2.md](../../01-architecture/founding-design-v2.md)。

## 一、入库清单（6 commits，均主代理亲验）

| Commit | 批次 | 内容 |
|---|---|---|
| `4e380a69` | S1·R8-B2 | CLI 装配根切换单一入口（preset 入链 Gate；四态行为等价真账；helper 模块尺寸棘轮净 0） |
| `e812fda9` | S2·R8-B3 | FileChainAuditSink 持久审计（jsonl 哈希链/拒篡改/重启 verify/savepoint 回滚）+ ChainApprovalAudit（IM 审批对经链落账）+ production() 响亮降级；ADR-0010 §6/§9 + CHANGELOG 同步 |
| `d6b6420b` | S3·B5 | TLS 补测：协议矩阵 1.2/1.3 真握手、ECDSA P-256 真链、冷启动轮换+旧信任拒；生产证书 5 步清单 |
| `f72a6f13` | S3·B6 | 加密字段六档定级（content_hash 改判加密、vector_json 保明、Unrated fail-closed）+ KDF 210k→600k + sqlite_sink 列级接入示范 |
| `e4b09767` | S3·B2 | SSRF 收口二段：LocalEndpointRegistry（回环注册制）+ ProviderAllowlist（公网限已配置基址）+ 重定向逐跳钉址 + GatedResolver |

门禁：gateway 312/0 · storage 198/0 · 桌面 crate 159/0 · governance 214 基线不动；clippy 0 / fmt 0 / arch_guard VIOLATIONS 0 / docs_truth OK；0 新外部 dep（依赖边仅 B5 期 +4/锁内既有）。**未竟**：全量数字只认效率线 gate_full（本批不发布自测全量数）。

## 二、S3 拍板记录（owner「按你的建议来」，2026-10-21）

- **B6 P1 定级**：vector_json 保明（检索基元+产品宪法「摆在明面」）；聚合元数据保明；content_hash/dedup 指纹列改判加密（字典反推面）；KDF 600k。残余：相似度推理（明向量+密文长度）、盲索引（HMAC）待选型、Unrated 边界列待逐列核、600k 开销未实测。
- **B8 排期（本页登记即 S3 交付）**：P1 代码签名证书接入 = **owner 资源决策位**（采购/签发面；发布就绪状态见并行线 [release 收口页]，只等签名证书——两页同向不重复）；P2 可复现构建证明 = **登记排期**（构建体系改造，建议随发布线 T1 批）；P3 其他平台校验链实测 = **登记排期**（需非 Windows 环境）；P4 清单与安装包分通道 = **登记排期**（SHA256SUMS 已随发布流水线接线，分通道方案待产品拍板）。
- **B7 治理缺口 6 项**（编排接线/物理多签 v2.1/审计持久化面/反思常驻/审议计时/席位矩阵）= 与 D2 挂账合并排期，归治理线正题（[governance-chain-drill](../governance-chain-drill-2026-10-20.md) §5 为准）。

## 三、环境体检（详见 artifacts/env-diagnosis-2026-10-21.md）

- %TEMP% 拒写 = 沙箱按可执行文件位置收口（target\ 下二进制禁写用户 %TEMP%）→ 测试跑前 `TEMP` 指向 `target\_tmp`；**建议效率线固化进 verify_changed**。
- 页面文件峰值不足（建议固定 32-48GB 或错峰编译）；service dry-run 计划任务 cmdlet 受限子进程拒访问（登记）。

## 四、残余总账（owner 决策位/后续批）

1. B2 残余：向导基址注册时机归前端线；allowlist 用户自定义配置面待产品拍板。
2. B5 残余：生产证书真验（持真链时按清单）、热重载/ACME/mTLS 需求另立。
3. B6 残余：真读写路径逐列接线（归口存储 owner）、盲索引选型、KekAnchor 平台锚定。
4. B7 缺口 6 项（治理线）；B8 四项（上表）。
5. 快反馈门禁环境红（verify_changed nextest 段 OOM/%TEMP%）——环境修好前以各 crate scoped 实测为准。

_安全线重启批收工。子代理停启由 owner 管理；B2-REG 由 lead 接手收尾（子代理卡停后主代理自验接手，33/33 模块测同数复证）。_
