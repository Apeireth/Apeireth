**Source**: https://raw.githubusercontent.com/TelivANT/memoryos-rust/main/README.md

---

# MemoryOS-Rust

High-Performance AI Agent Memory Management System - Rust Implementation

[![License](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](https://www.apache.org/licenses/LICENSE-2.0)
[![Rust](https://img.shields.io/badge/Rust-1.75+-orange.svg)](https://www.rust-lang.org/)
[![Status](https://img.shields.io/badge/Status-Release_Candidate-green.svg)](./CHANGELOG.md)
[![GitHub stars](https://img.shields.io/github/stars/TelivANT/memoryos-rust?style=social)](https://github.com/TelivANT/memoryos-rust/stargazers)
[![GitHub release](https://img.shields.io/github/v/release/TelivANT/memoryos-rust)](https://github.com/TelivANT/memoryos-rust/releases)
[![Build Status](https://img.shields.io/github/actions/workflow/status/TelivANT/memoryos-rust/ci.yml?branch=main)](https://github.com/TelivANT/memoryos-rust/actions)
[![Docker Pulls](https://img.shields.io/docker/pulls/telivant/memoryos-rust)](https://hub.docker.com/r/telivant/memoryos-rust)

**Languages**: [English](README.md) | [简体中文](README_CN.md) | [日本語](README_JA.md) | [Français](README_FR.md) | [العربية](README_AR.md) | [Deutsch](README_DE.md) | [Español](README_ES.md) | [한국어](README_KO.md)

> 📌 **Version Note**: This is the **Personal/Enterprise Single-Tenant Edition**. Multi-tenant features (RBAC + Tenant isolation) are included in the main branch.

---

## 🎯 Overview

MemoryOS-Rust is a high-performance AI Agent memory management system built with Rust + Tokio, featuring a 3-Tier memory architecture (STM/MTM/LTM), OpenAI API compatibility, and support for 100,000+ concurrent users.

**This edition is optimized for**:
- 👤 Individual developers and researchers
- 🏢 Single enterprise/organization deployments
- 🔒 On-premise installations with full data control

---

## ✨ Key Features

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

| Feature | MemoryOS-Rust | Mem0 | Advantage |
|---------|--------------|------|-----------|
| **Language** | Rust 🦀 | Python 🐍 | Lower overhead |
| **LLM Adapters** | 10 | 10+ | Similar |
| **Vector DBs** | 3 (Qdrant, Chroma, Pinecone) | 5+ | Good coverage |
| **Graph Memory** | ✅ entity/relation extraction + graph query | ✅ Neo4j | Similar capabilities |
| **Hot Config Reload** | ⚠️ Limited (restart required) | ❌ | See docs/CONFIG_HOT_RELOAD_LIMITATION.md |
| **Smart Routing** | ✅ Tier 0 FAQ + heuristic tiers | ⚠️ Basic | MemoryOS has Tier 0 |
| **Production Ready** | Release candidate (pre v1.0) | ✅ Mature | Mem0 is more mature |

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

| Spec | Minimum (Dev) | Recommended (Prod) |
| :--- | :--- | :--- |
| **CPU** | 2 vCPU | 4+ vCPU |
| **RAM** | 4GB | 16GB+ |
| **Disk** | 10GB SSD | 100GB NVMe |
| **OS** | Linux / macOS | Linux (K8s) |

---

## 🚀 Quick Start

### 1.Start Dependencies

```bash
docker-compose up -d
```

### 2. Configuration

Create `.env` file (optional) or set environment variables:
```bash
export GEMINI_API_KEY="your_key_here"
export QDRANT_API_KEY="your_qdrant_key"
```

Copy config file:
```bash
cp config.example.toml config.toml
# Edit config.toml to enable desired modules (Router, Wiki, etc.)
```

### 3. Run

```bash
# Default full-featured mode
cargo run --release --bin memoryos-gateway

# (Advanced) Enable specific features only (if Cargo.toml supports)
# cargo run --release --no-default-features --features "redis,qdrant"
```

### 4. Test

```bash
curl http://localhost:8080/health/status
```

**Detailed Guide**: [docs/QUICKSTART.md](./docs/QUICKSTART.md)

---

## 🏗️ Architecture

```mermaid
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

**Detailed Architecture**: [docs/ARCHITECTURE.md](./docs/ARCHITECTURE.md)

---

## 📚 Documentation

### User Documentation
- [Quick Start](./docs/QUICKSTART.md) - Get started in 5 minutes
- [User Manual](./docs/USER_MANUAL.md) - Complete usage guide 📖
- [Architecture](./docs/ARCHITECTURE.md) - System design (Graph/Router)
- [API Reference](./docs/API.md) - API documentation
- [Development Guide](./docs/DEVELOPMENT.md) - Development setup
- [Deployment Guide](./docs/DEPLOYMENT.md) - K8s/Docker deployment
- [K3s Auto-Deploy](./docs/K3S_DEPLOYMENT.md) - One-click K8s cluster 🚀
- [Authentication](./docs/AUTH.md) - API Key management
- [FAQ System](./docs/FAQ_SYSTEM.md) - Auto-promote high-frequency Q&A ⚡

### Performance Optimization
- [Optimization Analysis](./docs/OPTIMIZATION.md) - Algorithm optimization strategies 🚀
- [Usage Guide](./docs/OPTIMIZATION_USAGE.md) - How to use optimization modules ⚡

### Deep Dive
- [Design Principles](./docs/DESIGN.md) - Design philosophy & implementation ⭐
- [Comparison](./docs/COMPARISON.md) - vs Mem0 analysis ⭐

### Developer Documentation
- [Roadmap](./docs/ROADMAP.md) - v0.2.0 → v1.0.0 planning
- [API Key Auth](./docs/AUTH.md) - Enterprise auth system (Qdrant persistence) 🔒
- [Work Log](./WORK_LOG.md) - **Who's doing what, for collaboration** ⭐⭐⭐
- [Process Log](./PROCESS.md) - **All completed work + current progress** ⭐⭐
- [Project State](./docs/state.json) - AI context recovery (machine-readable)
- [Changelog](./CHANGELOG.md) - Version history
- [Contributing](./CONTRIBUTING.md) - Contribution guidelines
- [Documentation Index](./docs/README.md) - Complete docs navigation

**⭐ Recommended**: Design Principles and Comparison for system design insights

---

## 📊 Project Status

**Version**: 1.0.0-rc  
**Status**: Release Candidate (pre v1.0)  
  

| Phase | Module | Status | Notes |
|-------|--------|--------|-------|
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

- **Language**: Rust 1.75+ stable
- **Async Runtime**: Tokio
- **Web Framework**: Axum
- **Short-term Storage**: Redis
- **Vector Storage**: Qdrant
- **LLM**: OpenAI, Gemini, Claude, Ollama, DeepSeek, OpenRouter, Azure, Groq, Cohere, Mistral (10 adapters)

---

## 🤝 Contributing

Contributions are welcome! Please follow this workflow:

### Before Starting
1. 📖 Read [Development Guide](./docs/DEVELOPMENT.md)
2. 📝 Log your task in [WORK_LOG.md](./WORK_LOG.md)
3. 🔄 Pull latest code: `git pull`

### During Work
1. 📊 Update progress in [WORK_LOG.md](./WORK_LOG.md) daily
2. 🐛 Log issues immediately
3. 🔴 Update status if blocked

### After Completion
1. ✅ Mark task as complete in [WORK_LOG.md](./WORK_LOG.md)
2. 📝 Update [CHANGELOG.md](./CHANGELOG.md)
3. 🚀 Submit code: `git commit && git push`

**Collaboration**: We use `WORK_LOG.md` (human) + `docs/state.json` (AI) dual-track recording for transparent collaboration.

**Detailed Guide**: [CONTRIBUTING.md](./CONTRIBUTING.md)

---

## 🔧 Maintenance Status

**Current Status**: Active Development

This project is in early development. We are actively working on:
- 🐛 Bug fixes and security updates
- 📚 Documentation improvements
- 💡 Community-driven enhancements

**See**: [MAINTENANCE.md](./MAINTENANCE.md) for detailed maintenance plan

---

## 🏢 Enterprise Features

MemoryOS includes enterprise features in the main branch:

- 🏢 **Multi-Tenant Architecture**: Complete tenant isolation via `X-Tenant-ID` header
- 🔑 **RBAC Permission Model**: SuperAdmin / Admin / User / ReadOnly roles
- 📊 **Admin Service**: Dedicated management service on port 9090 (internal network only)
- 📋 **Audit Logging**: Persistent audit trail (JSONL)

---

## 📞 Contact

- **GitHub Issues**: [Report Issues](https://github.com/TelivANT/memoryos-rust/issues)
- **GitHub Discussions**: [Join Discussions](https://github.com/TelivANT/memoryos-rust/discussions)
- **Email**: 246803628+TelivANT@users.noreply.github.com
- **Security Issues**: Please email with subject `[SECURITY]`

---

## 📄 License

Apache 2.0 License - See [LICENSE](./LICENSE)

---

## 🌟 Related Projects

- **Original Project**: [MemoryOS](https://github.com/BAI-LAB/MemoryOS) - Python implementation
- **Paper**: [Memory OS of AI Agent](https://arxiv.org/abs/2506.06326)

---

**Version**: 1.0.0-rc (Personal Edition) | **Updated**: 2026-02-25