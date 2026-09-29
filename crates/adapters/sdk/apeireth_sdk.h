// apeireth-sdk C-ABI header (R122-8 auto-generated, 0 改 24 LOCKED)
// O-5 实质: 0 假装 100% multi-lang, 仅 5 fn demo 桥接.
// 0 改 workspace.version 1.1.0, 0 触碰 11 agent 公共 API 签名.
// Skeleton 桥接 1:1 c.rs 5 fn (count_tokens_c / hash_request_c /
// version_c / compile_info_c / free_string_c).
// 编译指令: cargo build -p apeireth-sdk --features c


#ifndef APEIRETH_SDK_H
#define APEIRETH_SDK_H

#pragma once

#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

#define SDK_SUBMODULE_COUNT 4

/**
 * 分类数量 (闭合词表规模, 测试钉死).
 */
#define ErrorCategory_COUNT 9

/**
 * 单帧帧体长度上界 (1 MiB): 覆盖最大信令帧 / 语音分块帧, 超限拒收.
 */
#define MAX_FRAME_BYTES (1024 * 1024)

/**
 * 长度前缀字节数 (u32 大端).
 */
#define LENGTH_PREFIX_BYTES 4

/**
 * K-1 强校验: `SDK_TOOL_WHITELIST` 长度 == 8 (6 工具 + 2 通用).
 */
#define SDK_TOOL_WHITELIST_COUNT 8

/**
 * **STUB MODE 守门标志** (K-1 强校验 #4): 编译期 hardcode = `true`.
 *
 * R21 真接 `apeireth-api` HTTP/WS 时, **必须经 8 哲学锚 (S-1/S-2/S-3 质量工程化 NEW/O-1 安全优先 NEW/O-2/O-3/O-4/O-5, baseline 2026-08-19)
 * + 主人审才能改 `false`**.
 */
#define STUB_MODE true

/**
 * API key 最小长度 (16, 防过短 key 误匹配).
 */
#define API_KEY_MIN_LENGTH 16

/**
 * API key 最大长度 (4 KB, 跟 `apeireth-keyring::TOKEN_MAX_LENGTH` 一致).
 */
#define API_KEY_MAX_LENGTH 4096

/**
 * 客户端 token bucket 容量 (P0 端点 1000 req/s, 普通 100 req/s, per D-04).
 */
#define CLIENT_BUCKET_CAPACITY 1000.0

/**
 * 客户端 token bucket 填充速率 (1000 token/s, 即 1000 req/s).
 */
#define CLIENT_BUCKET_REFILL_PER_SEC 1000.0

/**
 * 白名单长度守门 (8)。
 */
#define LARK_TOOL_WHITELIST_COUNT 8

/**
 * 8 核心 API 数守门。
 */
#define CORE_API_COUNT 8

/**
 * 6 核心 API 数量常量.
 */
#define CORE_API_COUNT 6

/**
 * 6 核心 API 数量常量.
 */
#define CORE_API_COUNT 6

/**
 * 6 消息类型守门常量。
 */
#define MESSAGE_TYPE_COUNT 6

/**
 * 5 鉴权要素守门常量 (App ID / App Secret / tenant token / user token / webhook token)。
 */
#define AUTH_METHOD_COUNT 5

/**
 * 4 实体守门常量 (Message / CalendarEvent / User / Document)。
 */
#define ENTITY_COUNT 4

/**
 * 6 K-1 强校验守门常量。
 */
#define K1_STRONG_VALIDATION_COUNT 6

/**
 * 4 K-1 强校验数量常量.
 */
#define K1_STRONG_VALIDATION_COUNT 4

/**
 * 6 K-1 强校验数量常量.
 */
#define K1_STRONG_VALIDATION_COUNT 6

/**
 * 单消息最大文本字节数 (防单消息爆炸)。
 */
#define MAX_MESSAGE_TEXT_BYTES 4096

/**
 * 单次 list_calendar_events 单页最大返回数。
 */
#define MAX_CALENDAR_EVENTS_PER_PAGE 1000

/**
 * 单 webhook 回调体字节上限 (防超大回调体)。
 */
#define MAX_WEBHOOK_CHUNK_BYTES (16 * 1024)



/**
 * 5 状态 hardcode 常量。
 */
#define InstanceStatus_COUNT 5

/**
 * 3 状态 hardcode 常量。
 */
#define TaskStatus_COUNT 3

/**
 * 默认 tenant_access_token TTL (2h = 7200s)。
 */
#define DEFAULT_TENANT_TOKEN_TTL_SECONDS 7200

/**
 * 默认 user_access_token TTL (2h = 7200s)。
 */
#define DEFAULT_USER_TOKEN_TTL_SECONDS 7200

/**
 * token 最大 TTL (24h, 防长占)。
 */
#define MAX_TOKEN_TTL_SECONDS 86400

/**
 * Token 最大 TTL (24h, per livekit-server 上限).
 */
#define MAX_TOKEN_TTL_SECONDS 86400

/**
 * Token 最大 TTL (24h, per Anthropic API 上限, 防长占).
 */
#define MAX_TOKEN_TTL_SECONDS 86400

/**
 * token 刷新提前量 (秒): 剩余 TTL 低于此值即视为需刷新。
 */
#define TOKEN_REFRESH_SKEW_SECS 60

/**
 * App ID 最小长度 (`cli_` + 8 字符 = 12)。
 */
#define MIN_APP_ID_LENGTH 12

/**
 * App Secret 最小长度 (16 字符)。
 */
#define MIN_APP_SECRET_LENGTH 16

/**
 * App Secret 典型长度 (32 字符)。
 */
#define TYPICAL_APP_SECRET_LENGTH 32

/**
 * 单次 list 调用最多跟随的页数 (防 page_token 循环)。
 */
#define MAX_EVENT_PAGES 10

/**
 * 5 状态 hardcode 常量。
 */
#define EventStatus_COUNT 5

/**
 * 文档标题字节上限。
 */
#define MAX_TITLE_BYTES 1024

/**
 * 3 variant hardcode 常量。
 */
#define DocumentType_COUNT 3

/**
 * 平台限流业务码 (命中即 [`ErrorClass::Retryable`], 退避后可重试)。
 */
#define PLATFORM_CODE_RATE_LIMITED 99991400

/**
 * 平台 access token 无效业务码 (命中即 [`ErrorClass::AuthFailed`])。
 */
#define PLATFORM_CODE_TOKEN_INVALID 99991663

/**
 * 平台 access token 过期业务码 (命中即 [`ErrorClass::AuthFailed`])。
 */
#define PLATFORM_CODE_TOKEN_EXPIRED 99991668

/**
 * 编译期守门: `LarkError` variant 数 (14)。新增 variant 必须同步改本常量。
 */
#define LARK_ERROR_VARIANT_COUNT 14

/**
 * 闭合词表大小 (3)。
 */
#define ErrorClass_COUNT 3

/**
 * 6 类型 hardcode 常量。
 */
#define MessageType_COUNT 6

/**
 * 事件时间戳允许的最大偏差 (秒, 防重放)。
 */
#define WEBHOOK_TIMESTAMP_SKEW_SECS 300

/**
 * 加密明文的随机前缀长度 (字节, 解密后丢弃)。
 */
#define EVENT_PAYLOAD_PREFIX_BYTES 16

/**
 * 4 variant hardcode 常量。
 */
#define EventType_COUNT 4

/**
 * 5 RoomState 数量常量.
 */
#define ROOM_STATE_COUNT 5

/**
 * 8 RoomEvent 数量常量.
 */
#define ROOM_EVENT_COUNT 8

/**
 * 事件广播 channel 容量 (100 条).
 */
#define EVENT_CHANNEL_CAPACITY 100

/**
 * 连接 / 握手默认超时 (毫秒).
 */
#define DEFAULT_CONNECT_TIMEOUT_MS 10000

/**
 * 连接 / 握手超时上限 (毫秒, 走 `apeireth_core::deadline::clamp_timeout` 过闸).
 */
#define MAX_CONNECT_TIMEOUT_MS 120000

/**
 * 协商帧缓冲上限 (溢出丢最旧, 防无消费者撑爆内存).
 */
#define MAX_NEGOTIATION_BUFFER 1024

/**
 * 白名单工具数.
 */
#define TOOL_WHITELIST_COUNT 7

/**
 * 白名单工具数.
 */
#define TOOL_WHITELIST_COUNT 7

/**
 * 默认 access token TTL (1h, per livekit-server 默认).
 */
#define DEFAULT_TOKEN_TTL_SECONDS 3600

/**
 * 默认 access token TTL (1h = 3600s, per Anthropic API 文档).
 */
#define DEFAULT_TOKEN_TTL_SECONDS 3600

/**
 * 8 事件 hardcode 常量.
 */
#define RoomEvent_COUNT 8

/**
 * 4 等级 + 1 unknown = 5 variant (per LiveKit 协议 实际 5 variant).
 */
#define ConnectionQuality_COUNT 5

/**
 * 5 权限 hardcode.
 */
#define Permission_COUNT 5

/**
 * 5 状态机 hardcode 常量.
 */
#define RoomState_COUNT 5

/**
 * 信令协议版本 (握手版本协商守门).
 */
#define PROTOCOL_VERSION 1

/**
 * 服务端建议心跳间隔的默认值 (毫秒; `Welcome` 可覆盖).
 */
#define DEFAULT_HEARTBEAT_INTERVAL_MS 5000

/**
 * 心跳超时默认值 (毫秒): 连续 [`MAX_HEARTBEAT_MISSED`] 次心跳无响应判失联.
 */
#define DEFAULT_HEARTBEAT_TIMEOUT_MS 15000

/**
 * 心跳最大连续丢失次数 (超过即进入重连状态机).
 */
#define MAX_HEARTBEAT_MISSED 3

/**
 * 单条数据消息分块体上界 (字节; 超限必须先分块).
 */
#define MAX_DATA_CHUNK_BYTES (16 * 1024)

/**
 * 单条数据消息最大分块数 (防超大消息拖垮重组缓冲).
 */
#define MAX_DATA_CHUNKS 256

/**
 * 帧变体总数 (闭合词表规模).
 */
#define SignalFrame_COUNT 26

/**
 * 2 类型 hardcode.
 */
#define TrackKind_COUNT 2

/**
 * 5 variant (4 known + 1 unknown, per LiveKit 协议).
 */
#define TrackSource_COUNT 5

/**
 * 编译期守门: 白名单长度 == 6。
 */
#define SANDBOX_TOOL_WHITELIST_COUNT 6

/**
 * 单沙箱最大存活时间 (秒, 防长占资源; 状态巡检按此回收)。
 */
#define SANDBOX_MAX_LIFETIME_SECONDS 3600

/**
 * 单次 stream_logs 最大 chunk 数 (防流爆炸)。
 */
#define SANDBOX_MAX_LOG_CHUNKS 10000

/**
 * 单 chunk 字节上限 (防单行爆炸)。
 */
#define SANDBOX_MAX_LOG_CHUNK_BYTES 4096

/**
 * 默认请求超时 (毫秒, deadline 缺省)。
 */
#define DEFAULT_REQUEST_TIMEOUT_MS 5000

/**
 * 请求超时上限 (毫秒, deadline 硬顶)。
 */
#define MAX_REQUEST_TIMEOUT_MS 60000

/**
 * 默认 wait 超时 (毫秒)。
 */
#define DEFAULT_WAIT_TIMEOUT_MS 30000

/**
 * wait 超时上限 (毫秒 = 单沙箱最大存活时间)。
 */
#define MAX_WAIT_TIMEOUT_MS (SANDBOX_MAX_LIFETIME_SECONDS * 1000)

/**
 * stream_logs 单次拉取 chunk 批大小。
 */
#define LOG_CHUNK_BATCH 16

/**
 * 编译期守门: 分类闭合词表长度 17。
 */
#define SANDBOX_ERROR_CODE_COUNT 17

/**
 * 编译期守门: 17 variant 守门 (新增 variant 必须同步改本 const)。
 */
#define SANDBOX_ERROR_VARIANT_COUNT 17

/**
 * 单沙箱最大 env 变量数 (按既有实现估算 64, 防 env 爆炸).
 */
#define MAX_ENV_VARS 64

/**
 * 单沙箱最大卷挂载数 (按既有实现估算 32).
 */
#define MAX_VOLUME_MOUNTS 32

/**
 * 单沙箱最大端口映射数 (按既有实现估算 16).
 */
#define MAX_PORT_MAPPINGS 16

/**
 * 最小 CPU 核数 (按既有实现估算 0.1, 防止过度限制导致进程无法启动).
 */
#define MIN_CPU_CORES 0.1

/**
 * 最大 CPU 核数 (按既有实现估算 64, 防止独占宿主机).
 */
#define MAX_CPU_CORES 64.0

/**
 * 最小内存 (16 MiB, 按既有实现估算, 防止进程无法启动).
 */
#define MIN_MEMORY_BYTES ((16 * 1024) * 1024)

/**
 * 最大内存 (256 GiB, 按既有实现估算, 防止 OOM 宿主机).
 */
#define MAX_MEMORY_BYTES (((256 * 1024) * 1024) * 1024)

/**
 * 最小 IO 带宽 (1 MiB/s, 按既有实现估算).
 */
#define MIN_IO_BANDWIDTH_BPS (1024 * 1024)

/**
 * 最大 IO 带宽 (10 GiB/s, 按既有实现估算).
 */
#define MAX_IO_BANDWIDTH_BPS (((10 * 1024) * 1024) * 1024)

/**
 * 最小网络带宽 (1 MiB/s, 按既有实现估算).
 */
#define MIN_NET_BANDWIDTH_BPS (1024 * 1024)

/**
 * 最大网络带宽 (10 GiB/s, 按既有实现估算).
 */
#define MAX_NET_BANDWIDTH_BPS (((10 * 1024) * 1024) * 1024)

/**
 * 最小临时目录大小 (1 MiB, 按既有实现估算).
 */
#define MIN_TMP_BYTES (1024 * 1024)

/**
 * 最大临时目录大小 (100 GiB, 按既有实现估算).
 */
#define MAX_TMP_BYTES (((100 * 1024) * 1024) * 1024)

/**
 * 编译期守门: 5 SandboxStatus 守门 (对齐既有实现状态机).
 */
#define SANDBOX_STATUS_COUNT 6

/**
 * 4 STT 模型数量常量.
 */
#define STT_MODEL_COUNT 4

/**
 * 4 TTS 模型数量常量.
 */
#define TTS_MODEL_COUNT 4

/**
 * 4 唤醒词类别数量常量.
 */
#define WAKE_WORD_CATEGORY_COUNT 4

/**
 * 3 VAD 算法数量常量.
 */
#define VAD_ALGORITHM_COUNT 3

/**
 * 采集会话默认队列容量 (帧).
 */
#define SESSION_CHANNEL_CAPACITY 100

/**
 * 一元 RPC 默认超时 (毫秒, 走 `apeireth_core::deadline::clamp_timeout` 过闸).
 */
#define DEFAULT_OP_TIMEOUT_MS 30000

/**
 * 一元 RPC 超时上限 (毫秒).
 */
#define MAX_OP_TIMEOUT_MS 120000

/**
 * API Key 最小长度 (per K-1 #1 强校验, 16 char).
 */
#define MIN_API_KEY_LENGTH 16

/**
 * API Key 典型长度 (32 char, per Anthropic voice 规范).
 */
#define TYPICAL_API_KEY_LENGTH 32

/**
 * 默认帧队列容量 (帧数): 覆盖约 2s @ 20ms 帧.
 */
#define DEFAULT_CAPTURE_QUEUE_FRAMES 100

/**
 * 采集帧时长上限 (毫秒).
 */
#define MAX_FRAME_DURATION_MS 1000

/**
 * VoiceConfig 段数 (per task spec §1, 编译期 hardcode 5).
 */
#define VOICE_CONFIG_SECTION_COUNT 5

/**
 * 默认采样率 (16kHz, per Porcupine 官方 + 既有实现估).
 */
#define DEFAULT_AUDIO_SAMPLE_RATE 16000

/**
 * 默认位深 (16-bit, 按既有实现估算).
 */
#define DEFAULT_AUDIO_BIT_DEPTH 16

/**
 * 默认通道数 (单声道, 按既有实现估算).
 */
#define DEFAULT_AUDIO_CHANNELS 1

/**
 * 闭合变体表规模 (测试钉死).
 */
#define VOICE_ERROR_VARIANT_COUNT 19

/**
 * 单分块载荷上界 (字节).
 */
#define MAX_CHUNK_BYTES (64 * 1024)

/**
 * 默认发送窗口 (分块数).
 */
#define DEFAULT_WINDOW_CHUNKS 8

/**
 * 信用累计上限 (防接收方无限发放把发送方信用撑爆).
 */
#define MAX_CREDITS 1024

/**
 * 帧类型总数 (闭合词表规模).
 */
#define STREAM_FRAME_COUNT 8

/**
 * 4 模型 hardcode 常量.
 */
#define SttModel_COUNT 4

/**
 * 一元帧类型总数 (闭合词表规模).
 */
#define VOICE_FRAME_COUNT 5

/**
 * 模型名最大长度 (防超长标识进错误 / 日志).
 */
#define MAX_MODEL_NAME_BYTES 128

/**
 * 4 模型 hardcode 常量.
 */
#define TtsModel_COUNT 4

/**
 * 置信度 EMA 系数.
 */
#define CONFIDENCE_EMA_ALPHA 0.5

/**
 * 3 算法 hardcode 常量.
 */
#define VadAlgorithm_COUNT 3

/**
 * 自定义唤醒词最大长度 (按既有实现估算 64 char, 防恶意长串).
 */
#define MAX_CUSTOM_WAKE_WORD_LENGTH 64

/**
 * 唤醒词最小长度 (按既有实现估算 3 char, 防过短误触).
 */
#define MIN_WAKE_WORD_LENGTH 3

/**
 * 包络重采样点数 (相似度比较的固定维度).
 */
#define ENVELOPE_BINS 32

/**
 * 相似度计算的最小包络长度 (帧数): 过短输入不足以构成判定.
 */
#define MIN_ENVELOPE_FRAMES 4

/**
 * 登记模板所需的最少 PCM 采样点 (10ms @ 16kHz).
 */
#define MIN_ENROLL_SAMPLES 160

/**
 * 默认判定阈值 (与 `WakeWord.sensitivity` 默认 0.5 配套上调, 偏保守).
 */
#define DEFAULT_WAKE_THRESHOLD 0.75

/**
 * 4 类别 hardcode 常量.
 */
#define WakeWordCategory_COUNT 4

/**
 * 错误闭合词表 (9 类).
 */
typedef struct ErrorCategory ErrorCategory;

/**
 * 错误分类闭合词表 (3 类, 编译期 hardcode, 不可扩).
 *
 * 调用方的重试策略只允许依赖本词表, 不允许解析错误字符串。
 */
typedef struct ErrorClass ErrorClass;

/**
 * 沙箱隔离级别 (3 variant, 语义对齐 既有 Sandbox SDK).
 *
 * K-1 强校验 #3: 编译期 hardcode, 不允许运行时增删 variant.
 */
typedef struct IsolationLevel IsolationLevel;

/**
 * 沙箱运行时 (3 variant, 语义对齐 既有 Sandbox SDK).
 *
 * K-1 强校验 #2: 编译期 hardcode, 不允许运行时增删 variant.
 */
typedef struct RuntimeKind RuntimeKind;

/**
 * 审批任务状态 (3 variant 闭合枚举)。
 */
typedef struct TaskStatus TaskStatus;



/**
 * 协议 schema 版本 (跟 [`LARK_SCHEMA_VERSION`] 同步锚点)。
 */
#define LARK_API_VERSION LARK_SCHEMA_VERSION

/**
 * 默认平台开放 API base URL (中性占位, 部署时覆盖)。
 */
#define DEFAULT_API_BASE DEFAULT_LARK_API_BASE









/**
 * 帧 schema 版本 (与 [`SANDBOX_SCHEMA_VERSION`] 同源)。
 */
#define WIRE_SCHEMA_VERSION SANDBOX_SCHEMA_VERSION

/**
 * Stub for the negotiation entry point — full negotiation in V2 D2.
 */
int32_t apeireth_sdk_init(void);

/**
 * Stub for error-message retrieval — last-error buffer wired in V2 D2.
 */
int32_t apeireth_sdk_last_error(uint8_t *_buf, uintptr_t _len);

/**
 * **C-ABI fn #1**: `apeireth_sdk_count_tokens(text: *const c_char) -> c_uint`.
 *
 * 安全性: caller 须保证 `text` 指向有效 UTF-8 + null-terminated C string.
 * Null / invalid ptr 返 0 (fail-soft, 与 abi.rs stub pattern 一致).
 */
unsigned int apeireth_sdk_count_tokens(const char *text);

/**
 * **C-ABI fn #2**: `apeireth_sdk_hash_request(method, url, body, body_len) -> *mut c_char`.
 *
 * **内存契约**: caller **必须**用 `apeireth_sdk_free_string` 释放返值, 0 用 C free().
 * Null ptr 返 null. invalid UTF-8 返 null.
 */
char *apeireth_sdk_hash_request(const char *method,
                                const char *url,
                                const unsigned int *body,
                                uintptr_t body_len);

/**
 * **C-ABI fn #3**: `apeireth_sdk_version() -> *const c_char`.
 *
 * **不漂移**: 复用 `apeireth_sdk::version::SDK_VERSION` 公共 API, 0 改 workspace.version 1.2.0 (双轴制: 产品轴 tag v1.0.0 + workspace 轴 1.2.0)。
 * 返 Rust `&'static CStr` 常驻指针, 生命周期 'static, **0 需要 free** (与 libc `getenv` pattern 一致)。
 *
 * **L 组修复**: 改 `std::sync::OnceLock` 只分配一次 — 修复前每次调用 `CString::into_raw`
 * 泄漏一个 CString 且头文件暗示免 free (同一 API 两套所有权契约)。统一契约:
 * version()/compile_info() 返**常驻指针, caller 0 应 free** (per R123 注记原意落地)。
 */
const char *apeireth_sdk_version(void);

/**
 * **C-ABI fn #4**: `apeireth_sdk_compile_info() -> *const c_char`.
 *
 * 返 "rustc X.Y.Z target triple, apeireth-sdk features: `[python,node,c,default]`" 字面量.
 * 0 假装实际 rustc version (编译期 hardcode "unknown" + "cfg(apeireth_sdk)" marker).
 *
 * **L 组修复**: 同 fn #3 — `OnceLock` 只分配一次, 返**常驻指针, caller 0 应 free**
 * (统一所有权契约, 消除每次调用泄漏一个 CString)。
 */
const char *apeireth_sdk_compile_info(void);

/**
 * **C-ABI fn #5**: `apeireth_sdk_free_string(ptr: *mut c_char)`.
 *
 * 释放 `apeireth_sdk_hash_request` 返的 C string (Rust 堆分配, caller **必须**释放).
 * **0 是 malloc 返值调 free() 行为未定义.**
 *
 * **L 组修复 (统一所有权契约)**: version()/compile_info() 返**常驻静态指针, 0 经本 fn
 * 释放** (对 `OnceLock` 的 `as_ptr()` 调 `from_raw` 是 UB)。本 fn 只服务 hash_request。
 */
void apeireth_sdk_free_string(char *ptr);

#endif  /* APEIRETH_SDK_H */
