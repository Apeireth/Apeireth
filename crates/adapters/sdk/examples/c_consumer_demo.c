/*
 * apeireth-sdk C consumer demo (R122-8 Multi-Lang SDK skeleton)
 *
 * 编译: cc c_consumer_demo.c -I.. -L ../target/release -lapeireth_sdk -o c_consumer_demo
 *   或:  cbindgen 生成 apeireth_sdk.h 后, ccload .so 调 5 fn C 签名
 *
 * 演示: 跨语言客户 (C/C++/Go/cgo) 如何 ccall apeireth-sdk C-ABI
 * 0 假装 100% 多语言支持 (per O-5 实质), 仅 demo 5 fn
 *
 * 编译指令: cargo build -p apeireth-sdk --features c --release
 *           cc examples/c_consumer_demo.c -L ../target/release -lapeireth_sdk -o c_consumer_demo
 *           ./c_consumer_demo
 */

#include <stdio.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

/* cbindgen auto-generated header (per build.rs c feature gate) */
#include "apeireth_sdk.h"

int main(void) {
    /* ----- 1. version ----- */
    const char *ver = apeireth_sdk_version();
    printf("apeireth-sdk version: %s\n", ver);

    /* ----- 2. compile_info ----- */
    const char *info = apeireth_sdk_compile_info();
    printf("apeireth-sdk compile info: %s\n", info);

    /* ----- 3. count_tokens (R32-1 启发式) ----- */
    const char *text = "Hello, 世界!";
    uint32_t n = apeireth_sdk_count_tokens(text);
    printf("count_tokens(\"%s\") = %u\n", text, n);

    /* ----- 4. hash_request (SHA-256 hex) ----- */
    const char *method = "POST";
    const char *url = "/v1/tools/web_search/invoke";
    const uint8_t body[] = "{}";
    size_t body_len = sizeof(body) - 1; /* exclude \0 */
    char *hash = apeireth_sdk_hash_request(method, url, body, body_len);
    if (hash != NULL) {
        printf("hash_request(%s, %s, %.*s) = %s\n",
               method, url, (int)body_len, (const char *)body, hash);
        /* 0 用 C free() — 必须用 apeireth_sdk_free_string 释放 */
        apeireth_sdk_free_string(hash);
    } else {
        printf("hash_request 失败 (null ptr)\n");
    }

    /* ----- 5. free_string (already used in step 4) ----- */
    printf("5 fn C-ABI 演示完成. R122-8 skeleton 0 假装 100%.\n");
    return 0;
}
