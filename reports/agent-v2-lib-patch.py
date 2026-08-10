import re
with open(r".openclaw\workspace\promethean\Apeireth-rust\crates\apeireth-cache\src\lib.rs", "r", encoding="utf-8") as f:
    content = f.read()

# Find Redis backend match arm, replace
# Pattern: from "BackendKind::Redis => {" up to "}\n}" of build_cache fn
start_marker = "        BackendKind::Redis => {"
end_marker = "        BackendKind::Memcached => Err(CacheError::BackendNotImplemented(\n            \"MEMCACHED\".to_string(),\n        )),\n    }\n}"

si = content.find(start_marker)
ei = content.find(end_marker, si)
if si == -1 or ei == -1:
    print(f"NOT FOUND: si={si} ei={ei}")
else:
    block_to_replace = content[si:ei + len(end_marker)]
    print(f"Found block: {repr(block_to_replace[:200])}...")

    new_block = """        BackendKind::Redis => {
            // R121 (V2-4): Redis backend not in build_cache<K, V> (cross-K, V cast issue)
            // Use build_cache_redis() explicitly
            Err(CacheError::DiskIoError(
                "Redis backend please use build_cache_redis()".into(),
            ))
        }
        BackendKind::Disk => Err(CacheError::BackendNotImplemented("DISK".to_string())),
        BackendKind::Memcached => Err(CacheError::BackendNotImplemented(
            "MEMCACHED".to_string(),
        )),
    }
}

/// R121 (V2-4): Redis cache backend explicit constructor
/// K = String, V = Vec<u8> (Redis protocol limit)
/// Real connection test uses #[ignore] (no redis-server locally)
pub async fn build_cache_redis() -> CacheResult<std::sync::Arc<dyn Cache<String, Vec<u8>>>> {
    let url = std::env::var("APEIRETH_REDIS_URL")
        .unwrap_or_else(|_| "redis://127.0.0.1:6379/0".to_string());
    let cache = redis_backend::RedisCache::new(url).await?;
    Ok(std::sync::Arc::new(cache))
}"""

    new_content = content[:si] + new_block + content[ei + len(end_marker):]
    with open(r".openclaw\workspace\promethean\Apeireth-rust\crates\apeireth-cache\src\lib.rs", "w", encoding="utf-8") as f:
        f.write(new_content)
    print("REPLACED")
