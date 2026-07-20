## DeltaMemory - AI Agent Memory

**Source**: https://www.deltamemory.com/blog/why-we-built-deltamemory-in-rust

---

Introducing DeltaMemory : Cognitive memory for production AI agents.Learn more >

[Back to blog](/blog)

Engineering

January 15, 2026

# Why We Built DeltaMemory in Rust

Sub-millisecond core operations, crash recovery, and a storage engine designed for cognitive workloads. Here is why Rust was the only choice.

![Why We Built DeltaMemory in Rust](https://images.pexels.com/photos/4709285/pexels-photo-4709285.jpeg)

Every decision in DeltaMemory's architecture traces back to one constraint: memory retrieval has to be fast enough that users never notice it happening.

When your AI agent pauses for two seconds to "remember" a previous conversation, the illusion breaks. The user stops trusting the agent. They repeat themselves. They disengage.

We needed sub-50ms retrieval. Not as a stretch goal. As a hard requirement.

## Why not Python?

Most AI infrastructure is written in Python. It makes sense for prototyping. The ecosystem is massive. But Python has a fundamental problem for latency-sensitive workloads: the Global Interpreter Lock.

When you need to run vector search, keyword matching, and graph traversal in parallel and merge the results in under 50 milliseconds, the GIL becomes a wall. You can work around it with multiprocessing, but now you are paying for IPC overhead and memory duplication.

We tried it. Our Python prototype hit 800ms p50 latency. Acceptable for a demo. Unacceptable for production.

## What Rust gives us

Rust gave us three things that matter for this problem:

**Predictable latency.** No garbage collector means no GC pauses. Every millisecond is accounted for. When we say 50ms p50, we mean it consistently, not "50ms except when the GC decides to run."

**True parallelism.** Our hybrid retrieval pipeline runs vector search, BM25 keyword matching, and knowledge graph traversal concurrently. Rust's ownership model makes this safe without locks in the hot path.

**Memory efficiency.** A single DeltaMemory instance handles thousands of concurrent users. Rust's zero-cost abstractions mean we are not paying a per-request tax for runtime overhead.

## A storage engine built for memory, not documents

Most vector databases are designed for document search. You embed a chunk, store it, query it later. DeltaMemory's workload is different. Memories are small, frequent, and constantly changing in importance.

So we built a custom storage engine from scratch. It follows an LSM-tree architecture: writes go to a Write-Ahead Log first for durability, then into an in-memory MemTable sorted by user, timestamp, and ID. When the MemTable fills up (default 16MB), it flushes to immutable SSTables on disk with index blocks for fast lookups.

Every WAL entry carries a CRC32 checksum. If the process crashes mid-write, we detect corruption on replay and skip the damaged entry instead of losing the entire log. The WAL replays in sequence order, so recovery is deterministic. You get exactly the state you had before the crash, minus whatever was in flight.

This matters because AI agents run 24/7. They cannot afford downtime for data repair.

## Retrieval is the hard part

Storing memories is straightforward. Retrieving the right ones at the right time is where things get interesting.

DeltaMemory runs a multi-stage retrieval pipeline on every query. First, we cast a wide net with HNSW approximate nearest neighbor search across vector embeddings. Then we layer in temporal indexing, pulling recent memories using BTreeMap-based time indexes that give us O(log N + k) range queries. Finally, we traverse the semantic graph, following concept-to-concept relationships to find memories connected to the query through meaning, not just similarity.

These three signals get combined using Reciprocal Rank Fusion, then scored with configurable cognitive weights: similarity, recency, and salience. A memory your user mentioned yesterday about their daughter's birthday ranks higher than a generic preference from three months ago.

The last step is diversity. We apply Maximal Marginal Relevance to avoid returning five memories that all say the same thing. The agent gets a diverse, relevant context window instead of redundant information.

All of this happens in under 50ms.

## Memories that fade like human memory

Not all memories are equally important, and importance changes over time. DeltaMemory models this with a salience decay system inspired by how human memory works.

Every memory has a salience score. Over time, that score decays exponentially. The formula is simple: current salience equals stored salience times e to the negative decay rate times age in days. Memories that are accessed frequently get their salience refreshed. Memories that are never recalled gradually fade below a prune threshold and get cleaned up.

This means your agent's context window is not cluttered with stale information. The memories that surface are the ones that matter right now.

## The cognitive pipeline

When a message comes in, DeltaMemory does not just store it verbatim. It runs a cognitive pipeline: perceive, think, act, remember.

The perceive step retrieves relevant context. Profiles come first, because structured facts about a user (their name, their role, their preferences) should always be available. Then episodic memories from vector search. Then recent conversation history from working memory.

The think step is where the LLM generates a response with all that context.

The remember step is where it gets interesting. DeltaMemory extracts facts from the conversation automatically. "I just moved to Austin" becomes a structured fact with a confidence score and a timestamp. If the user later says "I'm in Texas," the system recognizes this is related to the existing fact and merges them instead of creating a duplicate.

It also extracts concepts and relationships, building a knowledge graph over time. After enough interactions, DeltaMemory can traverse multi-hop connections: "the user works at Acme Corp" plus "Acme Corp is in the healthcare space" means the agent understands industry context without being told directly.

## Per-user isolation without global locks

AI agents serve many users simultaneously. A naive approach would use a global lock on the memory store, serializing all reads and writes. That kills throughput.

DeltaMemory uses per-user session isolation. Different users can read and write concurrently. Requests for the same user are serialized to maintain consistency, but user A's memory operations never block user B. The storage layer uses Rust's `RwLock` so multiple concurrent reads can happen while writes wait their turn.

This is where Rust's ownership model pays off. The compiler guarantees at compile time that we cannot accidentally share mutable state across threads. No data races. No subtle concurrency bugs that only show up under load at 3am.

## The tradeoff

Rust is harder to write. The compiler is demanding. Iteration speed is slower than Python or Go.

But for infrastructure that sits in the critical path of every AI agent interaction, the tradeoff is worth it. We get predictable performance, safe concurrency, and a storage engine that recovers gracefully from crashes. Our users do not care what language we use. They care that their agents respond instantly with the right context.

That is what Rust lets us deliver.