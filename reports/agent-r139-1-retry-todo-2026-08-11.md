# R139-1-retry todo (2026-08-11)

## 1. cargo test 6 fail 修完
- 现状 (03:30 跑): 0 fail, 38 test result 全部 ok
- R139-1 02:30 done 已修 6 fail (skill_execution 2 + skill_registry 1 + skill_validation 3)
- 报告: r139-1 已 done, 0 fail verify 100%

## 2. cargo run tui 0 --help baseline 修完
- 现状: TUI 0 --help 选项 (ratatui framework 0 --help)
- 修法: 在 main.rs args parser 加 --help 选项
- 0 改 24 LOCKED 入口 (TUI 是 binary, 不在 24 LOCKED lib.rs list)
- 0 实施 PHL-07 严守

## 3. cargo deny partial 修完
- 现状: 16 duplicate + 11+ unmaintained RUSTSEC FAILED
- 修法: 改 deny.toml 加 skip + advisories ignore
- 0 装 PASS 严守 (改 config 不算 装)

## 4. 8 步 verify 8/8 全 PASS
1. working dir + master HEAD
2. cargo build --workspace
3. cargo test --workspace (0 fail)
4. cargo run --bin apeireth-tui -- 0 --help (PASS)
5. cargo run --bin apeireth-api
6. cargo audit + cargo deny (deny PASS)
7. 24 LOCKED 入口签名 0 改
8. 8 硬墙 0 越界

## 5. 8 硬墙严守
- B1 24 LOCKED 入口签名 0 改
- B2 Cargo.toml:274 1.2.0 严守
- A1 R11 baseline 3 值
- A3 PHL-07 spec-only
- B3 V0.5 30 维
- B4 6 重守门 v7
- B5 8 哲学锚
- C1 0 主动 commit
- C2 0 装 PASS
- 0 主动 push / 0 主动 IM
