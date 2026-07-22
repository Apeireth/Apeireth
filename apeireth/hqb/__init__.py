"""HQB 真生产化 (V1085/V1086 配套 DB).

依据: R3-DB-01 (数据库工程师 2026-07-22).
设计原则: 仅新增 schema + store; 不动 memory_store / relation_store / asi_snapshot.
"""
from .schema import SCHEMA, SCHEMA_VERSION, HqbStore

__all__ = ["SCHEMA", "SCHEMA_VERSION", "HqbStore"]