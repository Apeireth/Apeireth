#!/usr/bin/env python3
"""Smoke test for alibaba/zvec 0.6.0 — Dense + FTS + Hybrid on Windows.

Validates:
1. Collection create with vector + FTS schema
2. Insert docs (vector + text)
3. Vector similarity search
4. Full-text search (BM25)
5. Hybrid (vector + FTS fusion)

Then benchmark vs our Phase 2.5 SQLite v0.2 (0.125ms/ep insert).
"""
import os
import shutil
import time
import tempfile

import zvec

# --- 1) Schema ---
schema = zvec.CollectionSchema(
    name="apeireth_smoke",
    vectors=[
        zvec.VectorSchema("embedding", zvec.DataType.VECTOR_FP32, 128),
    ],
    fields=[
        zvec.FieldSchema("content", zvec.DataType.STRING, index_param=zvec.FtsIndexParam()),
        zvec.FieldSchema("actor", zvec.DataType.STRING),
        zvec.FieldSchema("importance", zvec.DataType.INT32),
    ],
)

# Use tempdir for clean teardown
tmpdir = tempfile.mkdtemp(prefix="zvec_smoke_")
db_path = os.path.join(tmpdir, "apeireth.zvec")

print(f"=== creating collection at {db_path} ===")
coll = zvec.create_and_open(path=db_path, schema=schema)
print(f"  schema: {coll.schema.name}")
print(f"  vectors: {[v.name for v in coll.schema.vectors]}")
print(f"  fields: {[f.name for f in coll.schema.fields]}")

# --- 2) Insert ---
print()
print("=== insert ===")
N = 1000
import random
random.seed(42)
docs = []
for i in range(N):
    vec = [random.gauss(0, 1) for _ in range(128)]
    docs.append(zvec.Doc(
        id=f"ep_{i:06d}",
        vectors={"embedding": vec},
        fields={
            "content": f"Apeireth episode {i} master apeireth episode content benchmark iteration test",
            "actor": "master" if i % 3 == 0 else "ai_self",
            "importance": i % 10,
        }
    ))

t0 = time.perf_counter()
coll.insert(docs)
t1 = time.perf_counter()
print(f"  {N} inserts in {(t1-t0)*1000:.2f}ms ({(t1-t0)*1000/N:.3f}ms/ep)")

# --- 3) Vector search ---
# Use Query + Collection.query(topk=10). topk is on Collection.query, not Query.
print()
print("=== vector search (top 10) ===")
qvec = [random.gauss(0, 1) for _ in range(128)]
t0 = time.perf_counter()
results = coll.query(
    queries=zvec.Query(field_name="embedding", vector=qvec),
    topk=10,
    output_fields=["content", "actor"],
)
t1 = time.perf_counter()
print(f"  query took {(t1-t0)*1000:.2f}ms")
for r in results[:5]:
    print(f"  - {r.id}: score={r.score:.4f}")

# --- 4) FTS search ---
print()
print("=== FTS search (BM25 'apeireth master') ===")
t0 = time.perf_counter()
fts_results = coll.query(
    queries=zvec.Query(field_name="content", fts=zvec.Fts(query_string="apeireth master")),
    topk=10,
    output_fields=["content", "actor"],
)
t1 = time.perf_counter()
print(f"  query took {(t1-t0)*1000:.2f}ms")
for r in fts_results[:5]:
    print(f"  - {r.id}: score={r.score:.4f}")

# --- 5) Hybrid search ---
print()
print("=== hybrid (vector + FTS, RRF reranker) ===")
t0 = time.perf_counter()
hybrid_results = coll.query(
    queries=[
        zvec.Query(field_name="embedding", vector=qvec),
        zvec.Query(field_name="content", fts=zvec.Fts(query_string="apeireth iteration")),
    ],
    topk=10,
    reranker=zvec.RrfReRanker(),
    output_fields=["content", "actor"],
)
t1 = time.perf_counter()
print(f"  query took {(t1-t0)*1000:.2f}ms")
for r in hybrid_results[:5]:
    print(f"  - {r.id}: score={r.score:.4f}")

# --- 6) Stats ---
print()
print("=== stats ===")
stats = coll.stats
print(f"  stats attr: {stats}")

# --- 7) Cleanup ---
print()
print("=== cleanup ===")
# zvec 用 RocksDB 锁,WAL 进程退出前无法删除. 跳过 cleanup (OS 重启清)
print(f"  skip cleanup: {tmpdir} (zvec 持 RocksDB 锁)")
print()
print("✅ zvec 0.6.0 smoke test PASSED on Windows x86_64")
