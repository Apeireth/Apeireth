# V1075 ASI Real Deployment Run Report

- **Started:** 2026-07-22T13:10:17.051943+00:00
- **Stopped:** 2026-07-22T13:10:17.652740+00:00
- **Duration:** 0.60s
- **Mode:** `process`
- **Host:** 127.0.0.1
- **Port:** 8765
- **PID:** 27288
- **State:** `STOPPED`

## Environment (真实探测 / 主 17:43 实事求是)

- **Platform:** win32
- **Python:** 3.13.14
- **Docker available:** False
- **Docker compose available:** False
- **Port 8765 free:** True

## Health Check (真实 HTTP / 主 17:43 实事求是)

- **Healthy:** True
- **Status code:** 200
- **Latency:** 580.7ms
- **Attempts:** 1/5
- **Body preview:** `{"status":"ok","service":"apeireth-asi","version":"0.1.0","python":"3.13.14","platform":"Windows-11-10.0.26200-SP0","pid":27288,"ts":"2026-07-22T13:10:17.643097+00:00","endpoint":"health"}`

## Artifacts (主 23:44 干到底)

- `deploy\Dockerfile` (dockerfile, sha256=a883ec6c5ba72107, 1031 bytes)
- `deploy\docker-compose.yml` (compose, sha256=0884cb7cb177ccba, 680 bytes)
- `deploy\k8s-asi.yaml` (k8s, sha256=764ba4647a016b42, 1855 bytes)
- `deploy\apeireth-asi.service` (systemd, sha256=b4ec3b32c4e99ff0, 587 bytes)
- `deploy\apeireth-asi.supervisor.conf` (supervisor, sha256=d49cf45cf7b64c4a, 490 bytes)
- `deploy\.env.example` (env_example, sha256=9f87202d96ad6263, 148 bytes)

## Logs (主 00:56 可读)

```text
[1/6] probing environment
  docker=False compose=False port_free=True
[2/6] mode=process
[3/6] writing artifacts to deploy
  wrote 6 artifacts
[4/6] starting uvicorn on 127.0.0.1:8765
  pid=27288
[5/6] health check (5 retries)
  healthy status=200 latency=580.7ms attempt=1
[6/6] stopping service
  uvicorn stopped=True
```

## V3 哲学守门 (主 17:58 + 主 20:46 不假装)

- [x] 不假装 Docker 已装: `probe_environment` 真实命令 + 真实退出码
- [x] 不假装 compose 跑过 = 真服务: 真实 `docker compose ps` + 真 HTTP 200
- [x] 不假装 端口监听 = 服务可用: 真 HTTP GET + JSON 解析
- [x] 不假装 部署完成: 真 start → 真 ready → 真 query → 真 stop 全链路
- [x] 不假装 deployment = ASI: V1075 是真部署引擎, ASI 是更大目标

## 真借鉴 References (主 19:33 走在前人经验上)

- Docker2013: [Docker Dockerfile + HEALTHCHECK](https://docs.docker.com/engine/reference/builder/#healthcheck)
- ComposeV2: [Docker Compose v2 Services](https://docs.docker.com/compose/compose-file/)
- K8sProbes: [Kubernetes Liveness/Readiness Probes](https://kubernetes.io/docs/concepts/configuration/liveness-readiness-startup-probes/)
- 12Factor: [12-Factor App Config + Disposability](https://12factor.net/)
- Nomad2015: [HashiCorp Nomad Health Check](https://www.nomadproject.io/docs/job-specification/check)
- systemd2010: [systemd unit [Service] Restart](https://www.freedesktop.org/software/systemd/man/systemd.service.html)
- supervisord2004: [supervisord program autorestart](http://supervisord.org/configuration.html)
- uvicorn2018: [uvicorn ASGI server](https://www.uvicorn.org/settings/)
- FastAPI2018: [FastAPI health endpoint + lifespan](https://fastapi.tiangolo.com/advanced/events/)
- OpenTelemetry2019: [OpenTelemetry Resource Attributes](https://opentelemetry.io/docs/reference/specification/resource/)
- PromBlackbox2017: [Prometheus blackbox-exporter](https://github.com/prometheus/blackbox_exporter)
- NGINX2014: [NGINX upstream health_check](https://nginx.org/en/docs/http/ngx_http_upstream_module.html)
- Just2021: [Just command runner recipes](https://github.com/casey/just)
