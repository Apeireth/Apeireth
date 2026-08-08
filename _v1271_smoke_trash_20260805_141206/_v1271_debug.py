from apeireth import v1271_asi_stream_rate_limit_integration as v1271
from apeireth.v1270_asi_stream_rate_limiter import V1270RateLimitConfig, V1270RateLimiter
import time

cfg = V1270RateLimitConfig(requests_per_minute=60, tokens_per_minute=4000, max_concurrent=4)
lim = V1270RateLimiter(cfg)
for i in range(5):
    d = lim.acquire(tokens=10, now=time.time())
    snap = lim.snapshot()
    print(f"[{i}] allowed={d.allowed} reason={d.reason} snap={snap}")
    lim.release(tokens=10, now=time.time())