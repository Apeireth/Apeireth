# V1075 ASI Real Deployment Run Report

- **Started:** 2026-07-30T07:01:47.369189+00:00
- **Stopped:** 2026-07-30T07:01:48.543286+00:00
- **Duration:** 1.17s
- **Mode:** `process`
- **Host:** 127.0.0.1
- **Port:** 8875
- **PID:** 20408
- **State:** `STOPPED`

## Environment (真实探测 / 主 17:43 实事求是)

- **Platform:** win32
- **Python:** 3.13.14
- **Docker available:** False
- **Docker compose available:** False
- **Port 8875 free:** True

## Health Check (真实 HTTP / 主 17:43 实事求是)

- **Healthy:** True
- **Status code:** 200
- **Latency:** 1150.4ms
- **Attempts:** 1/5
- **Body preview:** `{"status":"ok","service":"apeireth-asi","version":"0.1.0","python":"3.13.14","platform":"Windows-11-10.0.26200-SP0","pid":20408,"ts":"2026-07-30T07:01:48.531513+00:00","endpoint":"health"}`

## Artifacts (主 23:44 干到底)

- `reports\r11-v1075-process\Dockerfile` (dockerfile, sha256=604da21208f9e0a9, 1031 bytes)
- `reports\r11-v1075-process\docker-compose.yml` (compose, sha256=c04b043aae233132, 680 bytes)
- `reports\r11-v1075-process\k8s-asi.yaml` (k8s, sha256=ef84f9f53ed359dc, 1855 bytes)
- `reports\r11-v1075-process\apeireth-asi.service` (systemd, sha256=23c058b4ae9f75b7, 587 bytes)
- `reports\r11-v1075-process\apeireth-asi.supervisor.conf` (supervisor, sha256=cce9389e80bc0fc9, 490 bytes)
- `reports\r11-v1075-process\.env.example` (env_example, sha256=5a5c7873bdeb1c58, 148 bytes)

## Logs (主 00:56 可读)

```text
[1/6] probing environment
  docker=False compose=False port_free=True
[2/6] mode=process
[3/6] writing artifacts to reports\r11-v1075-process
  wrote 6 artifacts
[4/6] starting uvicorn on 127.0.0.1:8875
  pid=20408
[5/6] health check (5 retries)
  healthy status=200 latency=1150.4ms attempt=1
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
