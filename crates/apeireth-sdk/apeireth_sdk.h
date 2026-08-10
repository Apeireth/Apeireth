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

/**
 * K-1 强校验: `SDK_TOOL_WHITELIST` 长度 == 8 (6 工具 + 2 通用).
 */
#define SDK_TOOL_WHITELIST_COUNT 8

/**
 * **STUB MODE 守门标志** (K-1 强校验 #4): 编译期 hardcode = `true`.
 *
 * R21 真接 `apeireth-api` HTTP/WS 时, **必须经 6 哲学锚 (S-1/S-2/O-2/O-3/O-4/O-5)
 * + 主人审才能改 `false`**.
 */
#define STUB_MODE true

/**
 * API key 最小长度 (16, 防过短 key 误匹配).
 */
#define API_KEY_MIN_LENGTH 16

/**
 * API key 最大长度 (4 KB, 跟 `apeireth-keyring::TOKEN_MAX_LENGTH` 1:1).
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
 * Null / invalid ptr 返 0 (fail-soft, 1:1 abi.rs stub pattern).
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
 * **不漂移**: 复用 `apeireth_sdk::version::SDK_VERSION` 公共 API, 0 改 workspace.version 1.1.0.
 * 返 Rust static str, 生命周期 'static, 0 需要 free (1:1 libc `getenv` pattern).
 */
const char *apeireth_sdk_version(void);

/**
 * **C-ABI fn #4**: `apeireth_sdk_compile_info() -> *const c_char`.
 *
 * 返 "rustc X.Y.Z target triple, apeireth-sdk features: [python,node,c,default]" 字面量.
 * 0 假装实际 rustc version (编译期 hardcode "unknown" + "cfg(apeireth_sdk)" marker).
 */
const char *apeireth_sdk_compile_info(void);

/**
 * **C-ABI fn #5**: `apeireth_sdk_free_string(ptr: *mut c_char)`.
 *
 * 释放 `apeireth_sdk_hash_request` / `apeireth_sdk_version` / `apeireth_sdk_compile_info`
 * 返的 C string. 0 是 malloc 返值调 free() 行为未定义.
 */
void apeireth_sdk_free_string(char *ptr);

#endif /* APEIRETH_SDK_H */
