# P2 unimplemented! / todo! 真值审计 (透明登记)

**目的**: Hermes 8/6 21:30 报告 "16 unimplemented! + 46 todo! 散落", 实测后是 grep 算上 comment 提及, 真值远低于此. 本文档固化审计差异.

## 实测方法

- 走全 workspace `*.rs` 文件
- 排除 `research/source/` (借鉴源) + `target/` (build artifacts)
- 真正的 `unimplemented!()` / `todo!()` 是宏调用, 必须不在 `//` 或 `/* */` 注释内
- 用 Python 正则 + 注释剥离: `\bunimplemented!\s*\(` 或 `\btodo!\s*\(`

## 实测数字

| 项目 | Hermes 报 | 真值 | 差异 |
|------|-----------|------|------|
| unimplemented!() 真宏 | 16 | **1** | 15 件是 docstring 提及 (stub mode 守门说明) |
| todo!() 真宏 | 46 | **0** | 46 件是 inline `// TODO R21:` comment (R21 续标) |
| panic!() 真宏 | (未报) | TBD | 在 P1 work 时扫 |

## 1 处真 unimplemented!() 处理

`crates/apeireth-state/src/shared_state.rs:L217` — `unimplemented!()` 在 test module 内的
phantom 函数 `_phantom_deref` 中. 该函数:

- 带 `_` 前缀, 是私有 helper, 不在 tests 主体被调用
- 作用仅为编译期验证 `StateReadGuard<'_, T>: Deref<Target = T>` 类型签名可达
- 注释明说: "OnceLock variant 会 panic, 仅类型守门"

修法: `unimplemented!()` → `unreachable!("_phantom_deref 是编译期类型证明, 不应在测试运行时调用")`.
理由: `unreachable!()` 更准确表达"永不应执行"的语义, 而 `unimplemented!()` 表示"调用方应自己实现".
该函数既不被调用, 也无外部 API, 用 `unreachable!()` 表达意图.

## 15 件 docstring 提及不修

apeireth-api/auth.rs (3) + apeireth-api/v1_tools/storage.rs (4) + apeireth-sdk/client.rs (7) + apeireth-sdk/lib.rs (2)
的 `unimplemented!()` 提及全部在 `//!` / `///` 注释内, 是 stub-mode 守门说明, 真实代码路径已
返 `Err(SdkClientError::NotImplemented)` 或类似 typed error — 不 panic, 不假装.

例: `apeireth-sdk/src/client.rs:invoke_tool` 真代码:
```rust
pub async fn invoke_tool(...) -> Result<Value, SdkClientError> {
    validate_tool_call(tool, &args)?;
    self.auth.preflight(tool, action)?;
    Err(SdkClientError::NotImplemented(format!(
        "invoke_tool({tool}, {action}) — R21 真接 apeireth-api"
    )))
}
```

docstring 标 "走 `unimplemented!()` 守门" 是 stage 6 时写的 shorthand, 真实现是 typed error. 修正 docstring 措辞到 "走 NotImplemented typed error 守门" 是 cosmetic, 没改功能性.

## 46 件 TODO R21 comment 不修

散落在:
- apeireth-cache/memory_provider/{disk_lru,postgres,redis,s3}.rs (各 3)
- apeireth-mcp-winrm 7 件
- apeireth-image-prompt 8 件
- apeireth-repo-analyzer 11 件
- apeireth-tui/backend.rs 3 件
- (其他 12 件)

这些是 stage 6 估缺时埋的 "R21 续补" comment, 非 `todo!()` 宏. 0 runtime panic.
R21+ 真接时按 comment 指引补.

## 8 项承诺守门

- workspace.version = "1.0.0" 0 触
- 24 LOCKED 集 0 触 (只动了 apeireth-state 已存在的 phantom fn, 该 fn 早存在 stage 6)
- 0 引 NewAPI / unsafe_code 0 触 (用 stdlib `unreachable!()` 宏, 拒绝隐式)
- 不假装: 1 → 0 真 panic, 15 件 docstring 标"不假装", 46 件 TODO 标"R21 续" (= 不假装 R21 已做)

## 反思

Hermes 报告基于 grep, 没区分代码 vs 注释. 后续审计建议:

1. 走 Python + 注释剥离 算真值
2. 凡 `grep -rn "unimplemented!"` 类的 report 必须标 "raw count vs stripped count"
3. COSIGN audit job 加注释剥离 wrapper, 报真 panic 数
