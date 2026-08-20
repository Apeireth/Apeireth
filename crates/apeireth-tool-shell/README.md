# apeireth-tool-shell

> Apeireth R138: shell extension (real sandbox seccomp/JobObject + russh SSH + persistent tasks + streaming + multi-sig + calculator), extending apeireth-tools/code_exec and long_task. TP4/N22 ShellPreset 预设命令模板 (白名单 + argv 模板展开 + 参数独立 quote 防注入, §10 官方包最后一件) + N17/TP2 register (ToolRegistry 装配统一注册件). src 模块 11 个 (lib + calculator + compat + enhanced + organ_kani_proofs + persist + preset + register + sandbox + ssh + streaming). 测试数 (#[test]): 40 in-src + 4 集成. 注: russh 借脑留 stub trait 口 (大编译时长 deferred), ssh module 真接 rusqlite-based 凭证查.

apeireth-tool-shell 是 Apeireth 1.0 (AGI 操作系统) 工作区 crate 之一。完整架构见 [docs/](../../docs/README.md)。
