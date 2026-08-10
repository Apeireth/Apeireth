# Apeireth Docker 包 (8 包之 1, D-06 拍板, Linux 重点优化)

主文件在仓库根目录, 此 README 仅说明用法 + 跨 8 包总入口引用.

## 文件清单

- `Dockerfile` (根目录) — multi-stage (builder + runtime-deps + distroless final)
- `docker-compose.yml` (根目录) — 3 服务 (apeireth + postgres + redis)

## 快速上手

```bash
# 1. 单镜像 build
docker build -t apeireth/apeireth:1.0.0 .

# 2. 多架构 build (linux/amd64 + linux/arm64, per 蓝图 §3.2)
docker buildx create --use --name apeireth-builder
docker buildx build --platform linux/amd64,linux/arm64 \
    --tag apeireth/apeireth:1.0.0 --tag apeireth/apeireth:latest \
    --push .

# 3. compose 启动 (3 服务)
docker-compose up -d
docker-compose ps  # 期望 3/3 healthy
curl http://localhost:8080/health

# 4. 推 GHCR
docker tag apeireth/apeireth:1.0.0 ghcr.io/apeireth/apeireth:1.0.0
docker push ghcr.io/apeireth/apeireth:1.0.0
```

## 镜像分层 (per 蓝图 §3.2)

| 层 | 大小 | 内容 |
|---|---|---|
| builder | ~2GB | rust 1.80 + 56 crate 依赖 + 源码 |
| runtime-deps | ~200MB | debian slim + libssl3 + libsqlite3-0 + libgit2-1.7 |
| final | ~150MB | gcr.io/distroless/cc-debian12:nonroot + apeireth binary |
| **总 final** | **~150MB** | **生产镜像, 可直接 docker run** |

## 安全要点 (per 蓝图 §3.5 第 3 项 security)

- ✅ non-root user (per distroless `:nonroot` 镜像)
- ✅ API key 通过环境变量注入, 不写进 image
- ✅ 内部网络隔离 (apeireth-net bridge, postgres/redis 不暴露端口)
- ✅ HEALTHCHECK 内置 (每 30s 调 `apeireth --health-check`)
- ✅ `cargo audit` + `cargo deny` 0 advisory

## 跟 R19 18 老 Dockerfile 的关系

- 旧 `docker/` 18 Dockerfile 保留 (dev 环境用)
- 新 `Dockerfile` + `docker-compose.yml` 1 套 (production 用, 1.0 release 默认)
- 0 改 18 老 Dockerfile (per 蓝图 §1.5 0 冲突)
