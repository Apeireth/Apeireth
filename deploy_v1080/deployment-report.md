# V1080 ASI Real Deployment Run Report

- **Started:** 2026-07-22T13:46:35.030794Z
- **Stopped:** 2026-07-22T13:46:36.067088Z
- **Duration:** 1.04s
- **Mode:** `process`
- **Port:** 8765
- **PID:** 50868
- **State:** `HEALTHY`

## Environment (真实探测 / 主 17:43 实事求是)

- **Platform:** win32
- **Python:** 3.13.14
- **Docker available:** False
- **Docker compose available:** False
- **Port 8765 free:** True
- **Workdir:** .openclaw\workspace\promethean

## Health Check (真实 HTTP / 主 17:43 实事求是)

- **Healthy:** True
- **Status code:** 200
- **Latency:** 1031.2ms
- **Attempts:** 1
- **Body preview:** `{"status":"ok","service":"apeireth-asi","version":"0.1.0","python":"3.13.14","platform":"win32","pid":50868,"ts":"2026-07-22T13:46:36.066476Z","endpoint":"health","module":"v1080_asi_real_subprocess_d`

## Artifacts (主 23:44 干到底)

- `Dockerfile`
- `docker-compose.yml`
- `requirements.txt`
- `apeireth-asi.service`

## V3 哲学守门 (主 17:58 + 主 20:46 不假装)

- [x] 不假装 Docker 已装: `environment_probe` 真实命令 + 真实退出码
- [x] 不假装 compose 跑过 = 真服务: 真子进程启动 + 真 HTTP GET /health
- [x] 不假装 端口监听 = 服务可用: 真 HTTP GET 状态码 200 + JSON 解析
- [x] 不假装 部署完成: 真 start -> 真 ready -> 真 query -> 真 stop 全链路
- [x] 不假装 deployment = ASI: V1080 是真部署 V1008+V1032 的引擎, ASI 是更大目标
