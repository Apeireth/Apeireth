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

#define SDK_SUBMODULE_COUNT 0

/**
 * 分类数量 (闭合词表规模, 测试钉死).
 */
#define ErrorCategory_COUNT 10

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
 * **只守 WS 流层**（`invoke_stream`）：HTTP 工具传输自 W5（2026-10-10）已真接，
 * 6 个工具方法可真调；仍为桩的只有 `invoke_stream` 与 `check_quota`，
 * 两者都返**显式错误码**（不 panic、不假装支持）。
 * 要把 WS 也真接（R21）时须经 8 哲学锚 + 主人审才能改 `false`。
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
 * 归一化 wire 面的 schema 版本 (入站/出站信封契约的同步锚点)。
 */
#define IM_WIRE_SCHEMA_VERSION 1

/**
 * 渠道 kind 数守门 (3 个中性渠道 id)。
 */
#define IM_CHANNEL_KIND_COUNT 3

/**
 * kind 数守门 (3)。
 */
#define ImChannelKind_COUNT IM_CHANNEL_KIND_COUNT

/**
 * 分类变体数守门 (6)。
 */
#define ImErrorClass_COUNT 6

/**
 * 默认最大段数 (超长回复的段数预算)。
 */
#define DEFAULT_MAX_SEGMENTS 8

/**
 * 默认尝试次数 (1 次发送 + 2 次重连)。
 */
#define ImReconnectPolicy_DEFAULT_MAX_ATTEMPTS 3

/**
 * 默认退避基数 (毫秒)。
 */
#define ImReconnectPolicy_DEFAULT_BACKOFF_BASE_MS 200

/**
 * 默认退避上限 (毫秒)。
 */
#define ImReconnectPolicy_DEFAULT_BACKOFF_CAP_MS 5000

/**
 * 错误闭合词表 (9 类).
 */
typedef struct ErrorCategory ErrorCategory;

/**
 * 渠道 kind (闭合 3 variant, wire 值即中性渠道 id)。
 */
typedef struct ImChannelKind ImChannelKind;





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
