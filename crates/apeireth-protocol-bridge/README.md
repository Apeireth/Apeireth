# apeireth-protocol-bridge

**R141** — VCP 协议兼容桥

## 职责

不在模仿 VCP, 但在兼容平台里兼容 VCP 协议的 50 命令. 这是 必要的兼容性, 不是 模仿.

## 核心能力

- VCP `<<<[TOOL_REQUEST]>>>...<<<[END_TOOL_REQUEST]>>>` 协议解析
- VCP placeholder (`<<<VCPToolInfo:*>>>...<<<EndVCPToolInfo>>>`) 渲染
- VCP 数字后缀批量调用约定 (`command1_1` / `command1_2` / `command2_1`)
- 桥接到 `apeireth-tool-*` 实战工具

## 命名说明

crate 名含 "vcp" 是因为这件事的本质是 "兼容 VCP 协议". 内部实现是 100% Rust + 实战工具, 0 借 VCP 源码.

## 0 假装

✅ 27 单元测试 | ✅ 50 命令路由真接
