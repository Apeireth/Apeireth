# orphan-scan 报告 (2026-08-17 01:44)

workspace 成员: 83 | 含 lib: 82 | 零 normal 消费者: 31 (孤儿 24 + dev-only 4 + bin 终点 3)

## 一、孤儿 lib crate (纯 lib 且零内部 normal 消费者, 待 Leader 处置决策)
apeireth-provider, apeireth-cron, apeireth-experience, apeireth-gateway, apeireth-environment, apeireth-config, apeireth-upgrade, apeireth-onion, apeireth-tool-shell, apeireth-tool-fetch, apeireth-tool-browser, apeireth-tool-codesearch, apeireth-tool-image-process, apeireth-context-fold, apeireth-credentials, apeireth-sdk, apeireth-team-lead, apeireth-repo-tools, apeireth-voice, apeireth-naming-v05, apeireth-state, apeireth-livekit, apeireth-blueprint-impl, apeireth-library-governance

## 二、dev/test 专用件 (零 normal 消费者, 但属 dev-only, 不算孤儿)
apeireth-bench, apeireth-test, apeireth-tui-e2e, apeireth-integration-e2e

## 二b、bin 终点件 (含 bin target, 天生 0 lib 消费者, 不算孤儿)
apeireth-cli, apeireth-web, apeireth-tui

## 三、dev-dep 自引用 (台账 #33② 模式)
apeireth-tool-fetch

## 四、dev-dep 双向回环
(无)

## 四b、dev↔normal 互指环 (台账 #33③ 模式)
apeireth-tool-runtime --dev--> apeireth-api --normal--> apeireth-tool-runtime; apeireth-tool-runtime --dev--> apeireth-tool-approval --normal--> apeireth-tool-runtime; apeireth-verify --dev--> apeireth-council --normal--> apeireth-verify; apeireth-verify --dev--> apeireth-sovereignty --normal--> apeireth-verify; apeireth-verify --dev--> apeireth-supervisor --normal--> apeireth-verify

## 五、dev-dep 长环 (DFS, 深度上限 8)
(无)

## 六、台账 #33 清单对账
- apeireth-provider : 仍为孤儿 (零 normal 消费者)
- apeireth-cron : 仍为孤儿 (零 normal 消费者)
- apeireth-experience : 仍为孤儿 (零 normal 消费者)
- apeireth-environment : 仍为孤儿 (零 normal 消费者)
- apeireth-config : 仍为孤儿 (零 normal 消费者)
- apeireth-state : 仍为孤儿 (零 normal 消费者)
- apeireth-naming-v05 : 仍为孤儿 (零 normal 消费者)
- apeireth-livekit : 仍为孤儿 (零 normal 消费者)
- apeireth-blueprint-impl : 仍为孤儿 (零 normal 消费者)
- apeireth-library-governance : 仍为孤儿 (零 normal 消费者)
- apeireth-voice : 仍为孤儿 (零 normal 消费者)
- apeireth-context-fold : 仍为孤儿 (零 normal 消费者)

## 七、全部成员消费计数 (normal)

| crate | normal 消费者数 | 消费者 |
|---|---|---|
| apeireth-acp | 1 | apeireth-provider |
| apeireth-action | 1 | apeireth-tui |
| apeireth-agent | 1 | apeireth-team-lead |
| apeireth-api | 6 | apeireth-agent, apeireth-asi, apeireth-cli, apeireth-council, apeireth-tui, apeireth-web |
| apeireth-arbitration | 2 | apeireth-runtime, apeireth-tui |
| apeireth-asi | 10 | apeireth-bench, apeireth-cli, apeireth-cognition, apeireth-companion, apeireth-graph, apeireth-pybridge, apeireth-tui, apeireth-value, apeireth-verify, apeireth-web |
| apeireth-bench | 0 | - |
| apeireth-blueprint-impl | 0 | - |
| apeireth-bus | 3 | apeireth-companion, apeireth-runtime, apeireth-team-lead |
| apeireth-central | 1 | apeireth-tui |
| apeireth-cli | 0 | - |
| apeireth-cognition | 3 | apeireth-graph, apeireth-tui, apeireth-web |
| apeireth-companion | 1 | apeireth-voice |
| apeireth-config | 0 | - |
| apeireth-consciousness | 9 | apeireth-cognition, apeireth-companion, apeireth-life-force, apeireth-motivation, apeireth-perception, apeireth-runtime, apeireth-tui, apeireth-voice, apeireth-web |
| apeireth-constraint | 1 | apeireth-upgrade |
| apeireth-context-fold | 0 | - |
| apeireth-core | 34 | apeireth-action, apeireth-api, apeireth-asi, apeireth-bench, apeireth-bus, apeireth-central, apeireth-cli, apeireth-cognition, apeireth-companion, apeireth-consciousness, apeireth-constraint, apeireth-context-fold, apeireth-council, apeireth-evolution, apeireth-graph, apeireth-graph-primitive, apeireth-life-force, apeireth-memory, apeireth-motivation, apeireth-onion, apeireth-perception, apeireth-pybridge, apeireth-sovereignty, apeireth-tool-browser, apeireth-tool-codesearch, apeireth-tool-filesystem, apeireth-tool-image-gen, apeireth-tool-image-process, apeireth-tools, apeireth-tool-shell, apeireth-tui, apeireth-upgrade, apeireth-value, apeireth-web |
| apeireth-council | 8 | apeireth-cli, apeireth-companion, apeireth-evolution, apeireth-runtime, apeireth-sovereignty, apeireth-team-lead, apeireth-tui, apeireth-upgrade |
| apeireth-credentials | 0 | - |
| apeireth-cron | 0 | - |
| apeireth-environment | 0 | - |
| apeireth-eval | 1 | apeireth-cli |
| apeireth-evolution | 1 | apeireth-companion |
| apeireth-experience | 0 | - |
| apeireth-extension | 1 | apeireth-bus |
| apeireth-gateway | 0 | - |
| apeireth-graph | 3 | apeireth-council, apeireth-team-lead, apeireth-tui |
| apeireth-graph-primitive | 2 | apeireth-companion, apeireth-tui |
| apeireth-guard | 2 | apeireth-companion, apeireth-gateway |
| apeireth-host | 2 | apeireth-api, apeireth-sdk |
| apeireth-http-client | 10 | apeireth-api, apeireth-eval, apeireth-pipeline, apeireth-provider, apeireth-runtime, apeireth-tool-browser, apeireth-tool-fetch, apeireth-tool-image-gen, apeireth-tools, apeireth-vector |
| apeireth-i18n | 1 | apeireth-tui |
| apeireth-integration-e2e | 0 | - |
| apeireth-lark | 1 | apeireth-companion |
| apeireth-library-governance | 0 | - |
| apeireth-life-force | 4 | apeireth-memory, apeireth-motivation, apeireth-tui, apeireth-web |
| apeireth-livekit | 0 | - |
| apeireth-llm-iface | 2 | apeireth-api, apeireth-memory |
| apeireth-mcp | 5 | apeireth-cli, apeireth-council, apeireth-eval, apeireth-graph, apeireth-skills |
| apeireth-memory | 10 | apeireth-agent, apeireth-api, apeireth-asi, apeireth-bench, apeireth-cli, apeireth-companion, apeireth-pybridge, apeireth-tool-runtime, apeireth-tui, apeireth-web |
| apeireth-memory-extensions | 1 | apeireth-memory |
| apeireth-motivation | 2 | apeireth-tui, apeireth-web |
| apeireth-naming-v05 | 0 | - |
| apeireth-onion | 0 | - |
| apeireth-perception | 2 | apeireth-tui, apeireth-web |
| apeireth-pipeline | 5 | apeireth-api, apeireth-eval, apeireth-team-lead, apeireth-tui, apeireth-web |
| apeireth-pipeline-g5 | 6 | apeireth-agent, apeireth-council, apeireth-memory, apeireth-pipeline, apeireth-runtime, apeireth-tool-runtime |
| apeireth-protocol | 7 | apeireth-api, apeireth-blueprint-impl, apeireth-eval, apeireth-pipeline, apeireth-sdk, apeireth-team-lead, apeireth-tool-runtime |
| apeireth-provider | 0 | - |
| apeireth-pybridge | 0 | - |
| apeireth-rate-limiter | 2 | apeireth-gateway, apeireth-tool-runtime |
| apeireth-repo-tools | 0 | - |
| apeireth-runtime | 1 | apeireth-tui |
| apeireth-sdk | 0 | - |
| apeireth-skills | 1 | apeireth-cli |
| apeireth-sovereignty | 11 | apeireth-companion, apeireth-context-fold, apeireth-skills, apeireth-tool-browser, apeireth-tool-codesearch, apeireth-tool-image-gen, apeireth-tool-image-process, apeireth-tool-shell, apeireth-tui, apeireth-upgrade, apeireth-web |
| apeireth-state | 0 | - |
| apeireth-supervisor | 4 | apeireth-graph, apeireth-runtime, apeireth-team-lead, apeireth-tui |
| apeireth-team-lead | 0 | - |
| apeireth-telemetry | 2 | apeireth-api, apeireth-tool-runtime |
| apeireth-test | 0 | - |
| apeireth-tool-approval | 8 | apeireth-agent, apeireth-companion, apeireth-tool-browser, apeireth-tool-codesearch, apeireth-tool-filesystem, apeireth-tool-image-gen, apeireth-tool-image-process, apeireth-tool-shell |
| apeireth-tool-browser | 0 | - |
| apeireth-tool-codesearch | 0 | - |
| apeireth-tool-fetch | 0 | - |
| apeireth-tool-filesystem | 1 | apeireth-tool-codesearch |
| apeireth-tool-image-gen | 1 | apeireth-tool-image-process |
| apeireth-tool-image-process | 0 | - |
| apeireth-tool-registry | 16 | apeireth-agent, apeireth-api, apeireth-companion, apeireth-mcp, apeireth-runtime, apeireth-team-lead, apeireth-tool-approval, apeireth-tool-browser, apeireth-tool-codesearch, apeireth-tool-filesystem, apeireth-tool-image-gen, apeireth-tool-image-process, apeireth-tool-runtime, apeireth-tools, apeireth-tool-shell, apeireth-tui |
| apeireth-tool-runtime | 11 | apeireth-agent, apeireth-api, apeireth-companion, apeireth-team-lead, apeireth-tool-approval, apeireth-tool-browser, apeireth-tool-codesearch, apeireth-tool-filesystem, apeireth-tool-image-gen, apeireth-tool-image-process, apeireth-tool-shell |
| apeireth-tools | 11 | apeireth-api, apeireth-companion, apeireth-context-fold, apeireth-eval, apeireth-mcp, apeireth-tool-browser, apeireth-tool-codesearch, apeireth-tool-filesystem, apeireth-tool-image-gen, apeireth-tool-image-process, apeireth-tool-shell |
| apeireth-tool-search | 2 | apeireth-runtime, apeireth-tui |
| apeireth-tool-shell | 0 | - |
| apeireth-tui | 0 | - |
| apeireth-tui-e2e | 0 | - |
| apeireth-upgrade | 0 | - |
| apeireth-value | 3 | apeireth-motivation, apeireth-tui, apeireth-web |
| apeireth-vector | 1 | apeireth-memory |
| apeireth-verify | 6 | apeireth-bench, apeireth-council, apeireth-evolution, apeireth-extension, apeireth-sovereignty, apeireth-supervisor |
| apeireth-voice | 0 | - |
| apeireth-web | 0 | - |
| apeireth-workflow | 1 | apeireth-runtime |
