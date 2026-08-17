# apeireth-livekit

> Apeireth R20 阶段 6 flesh out: LiveKit Server SDK 真接实现 (1:1 翻译 livekit-server-sdk 0.6+ Twirp API: server_url / api_key / room / track / participant / event 6 端点, 走 reqwest 0.12 + rustls-tls HTTP, wiremock 0.6 测; STUB 守门 6 核心 API + 5 K-1 强校验 + 8 tool whitelist 编译期 hardcode)

apeireth-livekit 是 Apeireth 1.0 (AGI 操作系统) 工作区 crate 之一。完整架构见 [docs/](../../docs/README.md)。
