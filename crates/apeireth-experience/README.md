# apeireth-experience

> Apeireth 经验沉淀层 (R173 / Stage2 §3) — LLM Wiki + Knowledge Graph + VCP 联想网络。

## 3 类经验沉淀

| 模块 | 职责 | 公共 API |
|---|---|---|
| `wiki` | LLM Wiki | `WikiEntry` |
| `graph` | Knowledge Graph | `KnowledgeGraph` / `KnowledgeNode` / `KnowledgeEdge` / `NodeKind` / `RelationKind` |
| `association` | VCP 联想网络 | `AssociationNetwork` / `AssociationNode` / `AssociationEdge` |

## 借鉴 (per stage2)

- claude-mem 3-layer progressive disclosure
- graphify EXTRACTED/INFERRED 双层
- vcptoolbox compound_eye 联想网络
- MemPalace 物理化记忆

## 设计约束 (不漂移)

- 0 改 `apeireth-memory` 任何已实装类型
- 0 副作用: 联想传播 mutating 但原子
- `#![deny(unsafe_code)]`

## 状态

Apeireth workspace 成员 (81 members, 0 orphan)。

**No-fake**: association/graph/wiki 3 模块 + 入口真实现 (R173 阶段 6 后端补全)。
**Run-no-fear**: `cargo check --workspace` 0 errors。

## 入口

- `Cargo.toml`: 见 [dependencies](Cargo.toml)
- `src/lib.rs`: 顶部 doc comment 是模块级总览

## 参见

- [Apeireth conventions](../../docs/conventions/README.md)
- [Apeireth 文档归位映射](../../docs/document-relocation-map.md)
