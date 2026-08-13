# apeireth-tool-browser

**R139** — 浏览器自动化工具

## 职责

CDP (Chrome DevTools Protocol) 封装, agent 可操作网页: open / click / type / scroll / query / screenshot.

## 核心能力

- 35 个 VCP ChromeBridge 命令 1:1 映射
- managed Chrome (独立 profile + tab 上限)
- 页面信息类型分组 ID 协议
- headless / headful 双模式

## 借鉴

VCP v1.1 `ChromeBridge V3`.

## 上升

- Rust 异步 trait + chromiumoxide (纯 Rust 内部)
- 区别于 selenium/playwright 等 polyglot 方案

## 0 假装

✅ 48 单元测试 | ⚠️ 真 browser 启动需 feature-gated chromiumoxide

## R162 lint cleanup

86 -> 0 warnings (per O-5: internal helpers allow(missing_docs); public API in lib.rs).
