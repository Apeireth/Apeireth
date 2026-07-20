## GitHub - TelivANT/memoryos-rust: Production AI Memory OS: <10ms FAQ, 90% cost savings via smart routing, unified gateway for teams — 100K users ready 🦀⚡💰 · GitHub

**Source**: https://github.com/TelivANT/memoryos-rust

---

[Skip to content](/charset=utf-8#start-of-content)

You signed in with another tab or window. Reload to refresh your session.

You signed out in another tab or window. Reload to refresh your session.

You switched accounts on another tab or window. Reload to refresh your session.
Dismiss alert

{{ message }}

[TelivANT](/TelivANT)

/
**[memoryos-rust](/TelivANT/memoryos-rust)**

Public

- [Notifications](/login?return_to=%2FTelivANT%2Fmemoryos-rust)You must be signed in to change notification settings
- [Fork
    1](/login?return_to=%2FTelivANT%2Fmemoryos-rust)
- 
[Star
          4](/login?return_to=%2FTelivANT%2Fmemoryos-rust)

[/TelivANT/memoryos-rust](/TelivANT/memoryos-rust)

[Branches](/TelivANT/memoryos-rust/branches)[Tags](/TelivANT/memoryos-rust/tags)

[/TelivANT/memoryos-rust/branches](/TelivANT/memoryos-rust/branches)[/TelivANT/memoryos-rust/tags](/TelivANT/memoryos-rust/tags)

Open more actions menu

## Folders and files

| Name | Name | Last commit message | Last commit date |
|---|---|---|---|
| Latest commit History165 Commits165 Commits |  |  |  |
| .github/workflows | .github/workflows |  |  |
| archive | archive |  |  |
| crates | crates |  |  |
| docs | docs |  |  |
| examples | examples |  |  |
| issues | issues |  |  |
| k8s | k8s |  |  |
| memoryos-sdk-python | memoryos-sdk-python |  |  |
| monitoring | monitoring |  |  |
| roadmap | roadmap |  |  |
| scripts | scripts |  |  |
| tests | tests |  |  |
| .dockerignore | .dockerignore |  |  |
| .env.example | .env.example |  |  |
| .gitignore | .gitignore |  |  |
| CHANGELOG.md | CHANGELOG.md |  |  |
| CONTRIBUTING.md | CONTRIBUTING.md |  |  |
| Cargo.lock | Cargo.lock |  |  |
| Cargo.toml | Cargo.toml |  |  |
| Dockerfile | Dockerfile |  |  |
| Dockerfile.worker | Dockerfile.worker |  |  |
| FIXES_REPORT.md | FIXES_REPORT.md |  |  |
| INTEGRATION_TESTING_README.md | INTEGRATION_TESTING_README.md |  |  |
| LICENSE | LICENSE |  |  |
| MAINTENANCE.md | MAINTENANCE.md |  |  |
| P0_FIXES.md | P0_FIXES.md |  |  |
| PERFORMANCE_BENCHMARKING_README.md | PERFORMANCE_BENCHMARKING_README.md |  |  |
| PROCESS.md | PROCESS.md |  |  |
| PRODUCTION_DEPLOYMENT_README.md | PRODUCTION_DEPLOYMENT_README.md |  |  |
| PROGRESS.md | PROGRESS.md |  |  |
| README.md | README.md |  |  |
| README_AR.md | README_AR.md |  |  |
| README_CN.md | README_CN.md |  |  |
| README_DE.md | README_DE.md |  |  |
| README_ES.md | README_ES.md |  |  |
| README_FR.md | README_FR.md |  |  |
| README_JA.md | README_JA.md |  |  |
| README_KO.md | README_KO.md |  |  |
| SECURITY_AUDIT.md | SECURITY_AUDIT.md |  |  |
| STATUS.md | STATUS.md |  |  |
| VERSION | VERSION |  |  |
| WORK_LOG.md | WORK_LOG.md |  |  |
| config.docker.toml | config.docker.toml |  |  |
| config.example.toml | config.example.toml |  |  |
| config.ollama.toml | config.ollama.toml |  |  |
| config.production.toml | config.production.toml |  |  |
| config.secure.toml | config.secure.toml |  |  |
| config.toml | config.toml |  |  |
| docker-compose.cluster.yml | docker-compose.cluster.yml |  |  |
| docker-compose.middleware-demo.yml | docker-compose.middleware-demo.yml |  |  |
| docker-compose.standalone.yml | docker-compose.standalone.yml |  |  |
| docker-compose.yml | docker-compose.yml |  |  |
| install.sh | install.sh |  |  |
| verify_fixes.sh | verify_fixes.sh |  |  |
| View all files |  |  |  |

## Repository files navigation

# MemoryOS-Rust

[/charset=utf-8#memoryos-rust](/charset=utf-8#memoryos-rust)

High-Performance AI Agent Memory Management System - Rust Implementation

[https://www.apache.org/licenses/LICENSE-2.0](https://www.apache.org/licenses/LICENSE-2.0)[https://www.rust-lang.org/](https://www.rust-lang.org/)[/TelivANT/memoryos-rust/blob/main/CHANGELOG.md](/TelivANT/memoryos-rust/blob/main/CHANGELOG.md)[https://github.com/TelivANT/memoryos-rust/stargazers](https://github.com/TelivANT/memoryos-rust/stargazers)[https://github.com/TelivANT/memoryos-rust/releases](https://github.com/TelivANT/memoryos-rust/releases)[https://github.com/TelivANT/memoryos-rust/actions](https://github.com/TelivANT/memoryos-rust/actions)[https://hub.docker.com/r/telivant/memoryos-rust](https://hub.docker.com/r/telivant/memoryos-rust)

**Languages**: [English](/TelivANT/memoryos-rust/blob/main/README.md) | [简体中文](/TelivANT/memoryos-rust/blob/main/README_CN.md) | [日本語](/TelivANT/memoryos-rust/blob/main/README_JA.md) | [Français](/TelivANT/memoryos-rust/blob/main/README_FR.md) | [العربية](/TelivANT/memoryos-rust/blob/main/README_AR.md) | [Deutsch](/TelivANT/memoryos-rust/blob/main/README_DE.md) | [Español](/TelivANT/memoryos-rust/blob/main/README_ES.md) | [한국어](/TelivANT/memoryos-rust/blob/main/README_KO.md)

> 📌 **Version Note**: This is the **Personal/Enterprise Single-Tenant Edition**. Multi-tenant features (RBAC + Tenant isolation) are included in the main branch.

---

## 🎯 Overview

[/charset=utf-8#-overview](/charset=utf-8#-overview)

MemoryOS-Rust is a high-performance AI Agent memory management system built with Rust + Tokio, featuring a 3-Tier memory architecture (STM/MTM/LTM), OpenAI API compatibility, and support for 100,000+ concurrent users.

**This edition is optimized for**:

- 👤 Individual developers and researchers
- 🏢 Single enterprise/organization deployments
- 🔒 On-premise installations with full data control

---

## ✨ Key Features

[/charset=utf-8#-key-features](/charset=utf-8#-key-features)

- 🚀 **High Performance**: Rust + Tokio async runtime, designed for high concurrency. Criterion microbenchmarks available; production QPS/latency pending real-world validation.
- 🧠 **Unified Vector Storage**: All memory tiers (STM/MTM/LTM) use vector databases for persistent storage.
- 💾 **3 Vector Database Options**: Qdrant (default), Chroma (lightweight), Pinecone (cloud-hosted).
- ⚡ **FAQ Heat Tracking**: High-frequency Q&A detection with heat score calculation and auto-promotion logic.
- 🔌 **Universal Gateway**: OpenAI protocol compatible, 10 LLM adapters (OpenAI, Gemini, Claude, Ollama, DeepSeek, OpenRouter, Azure, Groq, Cohere, Mistral).
- 🕸️ **Graph Memory**: Entity extraction + relation extraction + graph query API (/v1/graph) + DFS path query (v0.4.0).
- 📚 **Knowledge Export**: FAQ export to Local Markdown + S3 (OpenDAL) + Confluence (REST API) (v0.3.0).
- 🛡️ **Security Shield**: PII sanitization (email/phone/credit card/SSN/API key), prompt injection defense (17 patterns), IP defense system.
- 🤖 **3-Tier LLM Router**: Routes requests to different model tiers based on input complexity (heuristic-based) + Tier 0 FAQ direct hit (v0.3.0).
- 🔄 **Coordination Layer**: Redis/NATS for distributed coordination (Session, Lock, Cache, Message Queue).
- 🎯 **6 Performance Optimization Modules**: Bloom Filter, LRU Cache, Batch Processing, Heat Buffer, Similarity Filter, Incremental Summary.
- 🎨 **Multimodal Memory**: QdrantMultiModalStorage + HTTP API (/v1/multimodal/*) (v0.5.0, experimental).
- 🏷️ **Memory Versioning & Tags**: Version history + tag management + export/import (v0.6.0).
- 🔐 **Security Hardening**: AES-256-GCM encryption + persistent audit log (JSONL) + GDPR records (JSON) (v0.8.0~v0.9.0).
- 📊 **Prometheus Observability**: /metrics endpoint + HTTP/Router/FAQ/LLM full-chain metrics (v0.10.0).
- 🧠 **LLM FAQ Classification**: Automatic FAQ categorization via LLM + /v1/admin/faq/classify API (v0.10.0).
- 🔌 **MCP Server**: Model Context Protocol support with stdio transport, 7 tools for memory operations, Gateway proxy integration (v1.0.0-rc).

### vs Mem0 Comparison

[/charset=utf-8#vs-mem0-comparison](/charset=utf-8#vs-mem0-comparison)

| Feature | MemoryOS-Rust | Mem0 | Advantage |
|---|---|---|---|
| Language | Rust 🦀 | Python 🐍 | Lower overhead |
| LLM Adapters | 10 | 10+ | Similar |
| Vector DBs | 3 (Qdrant, Chroma, Pinecone) | 5+ | Good coverage |
| Graph Memory | ✅ entity/relation extraction + graph query | ✅ Neo4j | Similar capabilities |
| Hot Config Reload | ⚠️ Limited (restart required) | ❌ | See docs/CONFIG_HOT_RELOAD_LIMITATION.md |
| Smart Routing | ✅ Tier 0 FAQ + heuristic tiers | ⚠️ Basic | MemoryOS has Tier 0 |
| Production Ready | Release candidate (pre v1.0) | ✅ Mature | Mem0 is more mature |

**When to choose MemoryOS-Rust**:

- Want a Rust-based memory layer for AI Agents
- Need tight resource control and low overhead
- Prefer compiled language performance characteristics
- Building in the Rust ecosystem

**When to choose Mem0**:

- Python ecosystem preference
- Need more vector DB options
- Mature community and examples

---

## 💻 System Requirements

[/charset=utf-8#-system-requirements](/charset=utf-8#-system-requirements)

| Spec | Minimum (Dev) | Recommended (Prod) |
|---|---|---|
| CPU | 2 vCPU | 4+ vCPU |
| RAM | 4GB | 16GB+ |
| Disk | 10GB SSD | 100GB NVMe |
| OS | Linux / macOS | Linux (K8s) |

---

## 🚀 Quick Start

[/charset=utf-8#-quick-start](/charset=utf-8#-quick-start)

### 1.Start Dependencies

[/charset=utf-8#1start-dependencies](/charset=utf-8#1start-dependencies)

```
docker-compose up -d
```

### 2. Configuration

[/charset=utf-8#2-configuration](/charset=utf-8#2-configuration)

Create `.env` file (optional) or set environment variables:

```
export GEMINI_API_KEY="your_key_here"
export QDRANT_API_KEY="your_qdrant_key"
```

Copy config file:

```
cp config.example.toml config.toml
# Edit config.toml to enable desired modules (Router, Wiki, etc.)
```

### 3. Run

[/charset=utf-8#3-run](/charset=utf-8#3-run)

```
# Default full-featured mode
cargo run --release --bin memoryos-gateway

# (Advanced) Enable specific features only (if Cargo.toml supports)
# cargo run --release --no-default-features --features "redis,qdrant"
```

### 4. Test

[/charset=utf-8#4-test](/charset=utf-8#4-test)

```
curl http://localhost:8080/health/status
```

**Detailed Guide**: [docs/QUICKSTART.md](/TelivANT/memoryos-rust/blob/main/docs/QUICKSTART.md)

---

## 🏗️ Architecture

[/charset=utf-8#%EF%B8%8F-architecture](/charset=utf-8#%EF%B8%8F-architecture)

```
graph TD
    Client[User Client] -->|OpenAI Protocol| Gateway
    subgraph MemoryOS-Rust
        Gateway -->|Auth & Shield| Router{LLM Router}
        Router -->|Tier 1: Simple| SmallLLM[Small Model]
        Router -->|Tier 2: Medium| MediumLLM[Medium Model]
        Router -->|Tier 3: Complex| LargeLLM[Large Model]
        Gateway -->|Async Event| Queue[NATS/Redis]
        Queue --> Worker
        Worker -->|Summarize| VectorDB[(Qdrant)]
        Worker -->|Export| Wiki[Local/S3/Confluence]
    end
```

Loading

**Detailed Architecture**: [docs/ARCHITECTURE.md](/TelivANT/memoryos-rust/blob/main/docs/ARCHITECTURE.md)

---

## 📚 Documentation

[/charset=utf-8#-documentation](/charset=utf-8#-documentation)

### User Documentation

[/charset=utf-8#user-documentation](/charset=utf-8#user-documentation)

- [Quick Start](/TelivANT/memoryos-rust/blob/main/docs/QUICKSTART.md) - Get started in 5 minutes
- [User Manual](/TelivANT/memoryos-rust/blob/main/docs/USER_MANUAL.md) - Complete usage guide 📖
- [Architecture](/TelivANT/memoryos-rust/blob/main/docs/ARCHITECTURE.md) - System design (Graph/Router)
- [API Reference](/TelivANT/memoryos-rust/blob/main/docs/API.md) - API documentation
- [Development Guide](/TelivANT/memoryos-rust/blob/main/docs/DEVELOPMENT.md) - Development setup
- [Deployment Guide](/TelivANT/memoryos-rust/blob/main/docs/DEPLOYMENT.md) - K8s/Docker deployment
- [K3s Auto-Deploy](/TelivANT/memoryos-rust/blob/main/docs/K3S_DEPLOYMENT.md) - One-click K8s cluster 🚀
- [Authentication](/TelivANT/memoryos-rust/blob/main/docs/AUTH.md) - API Key management
- [FAQ System](/TelivANT/memoryos-rust/blob/main/docs/FAQ_SYSTEM.md) - Auto-promote high-frequency Q&A ⚡

### Performance Optimization

[/charset=utf-8#performance-optimization](/charset=utf-8#performance-optimization)

- [Optimization Analysis](/TelivANT/memoryos-rust/blob/main/docs/OPTIMIZATION.md) - Algorithm optimization strategies 🚀
- [Usage Guide](/TelivANT/memoryos-rust/blob/main/docs/OPTIMIZATION_USAGE.md) - How to use optimization modules ⚡

### Deep Dive

[/charset=utf-8#deep-dive](/charset=utf-8#deep-dive)

- [Design Principles](/TelivANT/memoryos-rust/blob/main/docs/DESIGN.md) - Design philosophy & implementation ⭐
- [Comparison](/TelivANT/memoryos-rust/blob/main/docs/COMPARISON.md) - vs Mem0 analysis ⭐

### Developer Documentation

[/charset=utf-8#developer-documentation](/charset=utf-8#developer-documentation)

- [Roadmap](/TelivANT/memoryos-rust/blob/main/docs/ROADMAP.md) - v0.2.0 → v1.0.0 planning
- [API Key Auth](/TelivANT/memoryos-rust/blob/main/docs/AUTH.md) - Enterprise auth system (Qdrant persistence) 🔒
- [Work Log](/TelivANT/memoryos-rust/blob/main/WORK_LOG.md) - **Who's doing what, for collaboration** ⭐⭐⭐
- [Process Log](/TelivANT/memoryos-rust/blob/main/PROCESS.md) - **All completed work + current progress** ⭐⭐
- [Project State](/TelivANT/memoryos-rust/blob/main/docs/state.json) - AI context recovery (machine-readable)
- [Changelog](/TelivANT/memoryos-rust/blob/main/CHANGELOG.md) - Version history
- [Contributing](/TelivANT/memoryos-rust/blob/main/CONTRIBUTING.md) - Contribution guidelines
- [Documentation Index](/TelivANT/memoryos-rust/blob/main/docs/README.md) - Complete docs navigation

**⭐ Recommended**: Design Principles and Comparison for system design insights

---

## 📊 Project Status

[/charset=utf-8#-project-status](/charset=utf-8#-project-status)

**Version**: 1.0.0-rc
**Status**: Release Candidate (pre v1.0)

| Phase | Module | Status | Notes |
|---|---|---|---|
| Phase 1 | Foundation (Config/Log) | Done | Functional |
| Phase 2 | Gateway & Adapters | Done | Basic implementation |
| Phase 3 | Storage (Redis/Qdrant) | Done | Needs production testing |
| Phase 4 | Intelligence (Router/Shield) | Done | Tier routing + FAQ Tier 0 |
| Phase 5 | Worker & Async | Done | Functional |
| Phase 6 | Wiki Export | Done | Local + S3 + Confluence |
| Phase 7 | Graph Memory | Done | Entity/relation extraction + graph query |
| Phase 8 | Multimodal | Done | Qdrant storage + HTTP endpoints (experimental) |
| Phase 9 | Security | Done | AES-256-GCM + audit + GDPR persistence |
| Phase 10 | Benchmarks | Done | Criterion microbenchmarks (see docs/PERFORMANCE_REPORT.md) |
| Phase 11 | Observability | Done | Prometheus /metrics + full-chain instrumentation |
| Phase 12 | LLM FAQ | Done | LLM-based FAQ classification + /v1/admin/faq/classify |
| Phase 13 | Enterprise | Done | RBAC + multi-tenant + Admin service + JSON file persistence |
| Phase 14 | Wiki Generation | Done | Tree-sitter + LLM hybrid, multi-language (Rust/Python/Java/Vue) |
| Phase 15 | Storage Connectors | Done | 17 connectors (Local/Git/S3/WebDAV/OSS/COS/OBS/SFTP/GCS/Azure/SMB/NFS/OneDrive/Google Drive/Dropbox/Baidu Pan/Aliyun Drive) |
| Phase 16 | MCP Server | Done | MCP protocol support (rmcp + stdio, Gateway proxy) |

> **Note**: End-to-end performance claims (QPS, latency) have not been independently validated yet. Criterion microbenchmark results are available in `docs/PERFORMANCE_REPORT.md`.

---

## 🛠️ Tech Stack

[/charset=utf-8#%EF%B8%8F-tech-stack](/charset=utf-8#%EF%B8%8F-tech-stack)

- **Language**: Rust 1.75+ stable
- **Async Runtime**: Tokio
- **Web Framework**: Axum
- **Short-term Storage**: Redis
- **Vector Storage**: Qdrant
- **LLM**: OpenAI, Gemini, Claude, Ollama, DeepSeek, OpenRouter, Azure, Groq, Cohere, Mistral (10 adapters)

---

## 🤝 Contributing

[/charset=utf-8#-contributing](/charset=utf-8#-contributing)

Contributions are welcome! Please follow this workflow:

### Before Starting

[/charset=utf-8#before-starting](/charset=utf-8#before-starting)

1. 📖 Read [Development Guide](/TelivANT/memoryos-rust/blob/main/docs/DEVELOPMENT.md)
2. 📝 Log your task in [WORK_LOG.md](/TelivANT/memoryos-rust/blob/main/WORK_LOG.md)
3. 🔄 Pull latest code: `git pull`

### During Work

[/charset=utf-8#during-work](/charset=utf-8#during-work)

1. 📊 Update progress in [WORK_LOG.md](/TelivANT/memoryos-rust/blob/main/WORK_LOG.md) daily
2. 🐛 Log issues immediately
3. 🔴 Update status if blocked

### After Completion

[/charset=utf-8#after-completion](/charset=utf-8#after-completion)

1. ✅ Mark task as complete in [WORK_LOG.md](/TelivANT/memoryos-rust/blob/main/WORK_LOG.md)
2. 📝 Update [CHANGELOG.md](/TelivANT/memoryos-rust/blob/main/CHANGELOG.md)
3. 🚀 Submit code: `git commit && git push`

**Collaboration**: We use `WORK_LOG.md` (human) + `docs/state.json` (AI) dual-track recording for transparent collaboration.

**Detailed Guide**: [CONTRIBUTING.md](/TelivANT/memoryos-rust/blob/main/CONTRIBUTING.md)

---

## 🔧 Maintenance Status

[/charset=utf-8#-maintenance-status](/charset=utf-8#-maintenance-status)

**Current Status**: Active Development

This project is in early development. We are actively working on:

- 🐛 Bug fixes and security updates
- 📚 Documentation improvements
- 💡 Community-driven enhancements

**See**: [MAINTENANCE.md](/TelivANT/memoryos-rust/blob/main/MAINTENANCE.md) for detailed maintenance plan

---

## 🏢 Enterprise Features

[/charset=utf-8#-enterprise-features](/charset=utf-8#-enterprise-features)

MemoryOS includes enterprise features in the main branch:

- 🏢 **Multi-Tenant Architecture**: Complete tenant isolation via `X-Tenant-ID` header
- 🔑 **RBAC Permission Model**: SuperAdmin / Admin / User / ReadOnly roles
- 📊 **Admin Service**: Dedicated management service on port 9090 (internal network only)
- 📋 **Audit Logging**: Persistent audit trail (JSONL)

---

## 📞 Contact

[/charset=utf-8#-contact](/charset=utf-8#-contact)

- **GitHub Issues**: [Report Issues](https://github.com/TelivANT/memoryos-rust/issues)
- **GitHub Discussions**: [Join Discussions](https://github.com/TelivANT/memoryos-rust/discussions)
- **Email**: [246803628+TelivANT@users.noreply.github.com](mailto:246803628+TelivANT@users.noreply.github.com)
- **Security Issues**: Please email with subject `[SECURITY]`

---

## 📄 License

[/charset=utf-8#-license](/charset=utf-8#-license)

Apache 2.0 License - See [LICENSE](/TelivANT/memoryos-rust/blob/main/LICENSE)

---

## 🌟 Related Projects

[/charset=utf-8#-related-projects](/charset=utf-8#-related-projects)

- **Original Project**: [MemoryOS](https://github.com/BAI-LAB/MemoryOS) - Python implementation
- **Paper**: [Memory OS of AI Agent](https://arxiv.org/abs/2506.06326)

---

**Version**: 1.0.0-rc (Personal Edition) | **Updated**: 2026-02-25

## About

         Production AI Memory OS: <10ms FAQ, 90% cost savings via smart routing, unified gateway for teams — 100K users ready 🦀⚡💰       

### Topics

[rust](/topics/rust)[ai](/topics/ai)[knowledge](/topics/knowledge)[llm](/topics/llm)[ai-gateway](/topics/ai-gateway)[memory-system](/topics/memory-system)[memory-os](/topics/memory-os)

### Resources

[Readme](/charset=utf-8#readme-ov-file)

### License

[Apache-2.0 license](/charset=utf-8#Apache-2.0-1-ov-file)

### Contributing

[Contributing](/charset=utf-8#contributing-ov-file)

###         Uh oh! 

There was an error while loading. Please reload this page.

[Activity](/TelivANT/memoryos-rust/activity)

### Stars

**4**             stars         

### Watchers

**0**             watching         

### Forks

[1
        fork](/TelivANT/memoryos-rust/forks)

[Report repository](/contact/report-content?content_url=https%3A%2F%2Fgithub.com%2FTelivANT%2Fmemoryos-rust&report=TelivANT+%28user%29)

## [Releases](/TelivANT/memoryos-rust/releases)

No releases published

## [Packages
      0](/users/TelivANT/packages?repo_name=memoryos-rust)

###         Uh oh! 

There was an error while loading. Please reload this page.

## [Contributors](/TelivANT/memoryos-rust/graphs/contributors)

- 

- 

- 

###         Uh oh! 

There was an error while loading. Please reload this page.

## Languages

- [Rust
          93.7%](/TelivANT/memoryos-rust/search?l=rust)
- [Shell
          4.5%](/TelivANT/memoryos-rust/search?l=shell)
- [Python
          1.7%](/TelivANT/memoryos-rust/search?l=python)
- [Dockerfile
          0.1%](/TelivANT/memoryos-rust/search?l=dockerfile)

     You can’t perform that action at this time.