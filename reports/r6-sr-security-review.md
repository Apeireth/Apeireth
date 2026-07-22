# R6-SR-01 安全审查

静态审查 4 模块；未改源码、未跑攻击测试。未见 `eval/exec/os.system` 或危险 YAML Loader。三个 P0 模块均为契约壳，不能证明 R7 已具备安全或恢复能力。

## 模块审查

**self_reproduction.py**：`target_path` 仅验非空；无允许根、规范化、链接、覆盖或配额约束，R7 实现后可越界写。snapshot/verify/restore 仅签名，无完整性、原子恢复保证。

**self_mod_safety.py**：无危险执行 API，但 checkpoint/rollback 未实现，当前不能恢复状态。ID/scope 无签名、授权或主体绑定；布尔 verify 缺证据、策略版本及 fail-closed 语义。

**formal_verify.py**：`CONTRACT_ONLY=True`，未调用证明器，当前无注入面也无真实证明。claim/code 缺大小、语法、后端白名单；布尔结果未绑定模型、假设、产物与工具版本。

**v1000_yaml_serializer.py**：使用 `safe_load`/`safe_load_all` 和 `SafeDumper`，阻断对象 RCE。仍可任意路径读写覆盖；无大小/别名/深度/文档数限制，且 load_all、dump_stream 会全量驻留内存。

## 风险

### High
- R7 沿用任意路径可借绝对路径、`..`、junction/symlink 逃逸并覆盖代码或密钥。
- 将契约的 rollback 布尔值当恢复证据，会在部分写入或身份/审计污染后继续自改。
- YAML 文件 API 若开放给不可信调用方，可覆盖宿主文件并间接升级为代码执行。

### Medium
- YAML 别名、深层/大文件、多文档可导致 CPU/内存/磁盘 DoS；“streaming”名实不符。
- R7 证明器若拼接 claim/spec 到 shell，将产生注入；契约未要求隔离、配额或参数数组。
- checkpoint ID、scope 和 verify 证据缺授权、防重放及不可抵赖性，易 fail-open。

### Low
- 自由 metadata/mutation/counterexample 与 YAML 异常可能把敏感信息带入日志或证明产物。

## R7 建议

1. 固定 workspace 根；resolve 后校验 `is_relative_to`，拒绝绝对路径、`..`、链接/reparse point；默认新建，原子 rename，最小权限和磁盘配额。
2. 自改放入无网络、只读源码、系统调用/资源受限的隔离 worker；上线前独立授权，模型不得扩大自身权限。
3. checkpoint 覆盖代码、配置、锁文件、身份和审计游标；内容寻址并签名、事务落盘。跨进程演练恢复并重验哈希/不变量，失败关闭。
4. 证明器固定版本，禁 shell，限制时空/文件并保存 spec、假设、模型哈希和证明产物；证明不替代沙箱。YAML 边界限制目录、大小、深度、别名、文档数，原子写且日志脱敏。
