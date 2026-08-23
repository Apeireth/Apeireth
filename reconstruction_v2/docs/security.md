# Apeireth 2.0 安全与治理体系规范

> **安全总纲**：安全底线永不外包 · 0 虚招，真防御 · 三重洋葱 · S4 出站默认拒绝 (Default-Deny)

---

## 一、 治理层级与防线

Apeireth 2.0 将分散的 6 个安全相关 Crate 收敛为统一的 `apeireth-governance`，构筑了五层纵深防御体系：

```
┌────────────────────────────────────────────────────────────────────────┐
│                        Apeireth 2.0 纵深安全防线                       │
└────────────────────────────────────────────────────────────────────────┘
  1. 哲学守门 (V1): 13 哲学键常量硬编码约束 (NotClone, NotSafe, NotUndo 等)
  2. 权限洋葱 (V2): 基于属性的权限访问控制 (ABAC) 与 PermissionPack 凭证
  3. 人类授权 (V3): 物理隔离的 L0 人类批准 (不可由 AI 自动绕过, Q20 百年章节)
  4. 出站拦截 (S4): Gateway 层物理 Socket/Reqwest 级域名与 IP 白名单拦截
  5. 进程沙箱 (SB): Windows JobObject (CPU/Memory) + RestrictedToken 最小特权
```

---

## 二、 三重洋葱架构 (Triple Onion)

1. **原则洋葱 (Principle Onion)**：核心 L0 到 L5 的分层权限包，高危操作强制要求 L0 人类授权。
2. **权限洋葱 (Permission Onion)**：工具级别的动态权限授权（单次、小时、永久）。
3. **DSL 洋葱 (DSL Guardrails)**：声明式行为规则校验，拦截命令注入与越权行为。

---

## 三、 S4 出站 Default-Deny 实装（破除 1.0 占位暗伤）

在 `apeireth-gateway/src/egress.rs` 中，实现了真正的出站请求物理过滤：
- **默认拒绝（Default-Deny）**：所有对公网的 HTTP/Socket 请求默认全部阻断。
- **动态白名单**：仅允许预配置或经授权的合法服务域名与 IP（如已配置的 LLM 端点）。
- **SHA-256 审计链**：所有外发请求在发包前自动记录至不可变审计哈希链，保证每一条外网流量均有据可查。

---

## 四、 防篡改审计链 (Tamper-Evident Hash Chain)

在 `apeireth-governance/src/audit.rs` 中：
- 采用 SHA-256 前向链接算法：`Hash(n) = SHA256(Hash(n-1) || Timestamp || ActionPayload)`。
- 每 N 条记录生成一次锚定签名（Anchor Checkpoint），任何历史记录的修改或删除都会导致哈希链断裂，立即可被校验算法检测。
