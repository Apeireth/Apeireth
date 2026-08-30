# Apeireth Docker 包 (Apeireth 2.0 RC1, Linux packaging)

## 文件清单

- `Dockerfile` (根目录) & `packaging/docker/Dockerfile` — multi-stage (builder + runtime slim)
- `docker-compose.yml` (根目录) — 3 服务 (apeireth + postgres + redis)

## 快速上手

```bash
# 1. 单镜像 build
docker build -t apeireth/apeireth:2.0.0-rc.1 -t apeireth/apeireth:latest .

# 2. 多架构 build (linux/amd64 + linux/arm64)
docker buildx create --use --name apeireth-builder
docker buildx build --platform linux/amd64,linux/arm64 \
    --tag apeireth/apeireth:2.0.0-rc.1 --tag apeireth/apeireth:latest \
    --tag ghcr.io/apeireth/apeireth:2.0.0-rc.1 --tag ghcr.io/apeireth/apeireth:latest \
    --push .

# 3. compose 启动 (3 服务)
docker-compose up -d
docker-compose ps  # 期望 3/3 healthy
curl http://localhost:8080/health

# 4. 推 GHCR
docker tag apeireth/apeireth:2.0.0-rc.1 ghcr.io/apeireth/apeireth:2.0.0-rc.1
docker push ghcr.io/apeireth/apeireth:2.0.0-rc.1
```

## 镜像分层

| 层 | 大小 | 内容 |
|---|---|---|
| builder | ~2GB | rust:1.97.1-bookworm + libdbus-1-dev + 源码编译 |
| runtime | ~150MB | debian:bookworm-slim + ca-certificates + curl + libdbus-1-3 + apeireth binary |
| **总 runtime** | **~150MB** | **生产镜像 (非 root 用户: apeireth:10001), 可直接 docker run** |

## 安全要点

- ✅ non-root user (`apeireth` uid 10001)
- ✅ API key 通过环境变量注入, 不写进 image
- ✅ 内部网络隔离 (apeireth-net bridge, postgres/redis 不暴露外部端口)
- ✅ HEALTHCHECK 内置 (每 30s 调 `curl http://127.0.0.1:8080/health`)
- ✅ `cargo audit` + `cargo deny` 0 advisory

