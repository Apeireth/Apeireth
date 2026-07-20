#!/usr/bin/env python3
"""Inspect zvec Query API."""
import zvec
import inspect

print('=== Query class ===')
sig = inspect.signature(zvec.Query.__init__)
print(f'Query.__init__{sig}')
print()
print('=== VectorQuery ===')
print(f'VectorQuery.__init__{inspect.signature(zvec.VectorQuery.__init__)}')
print()
print('=== Fts ===')
print(f'Fts.__init__{inspect.signature(zvec.Fts.__init__)}')
print()
print('=== Collection.query ===')
print(f'query{inspect.signature(coll_query)}')
coll_query = zvec.Collection.query
print(f'query{inspect.signature(coll_query)}')
print()
print('=== Query constructors ===')
for name in ('Query', 'VectorQuery', 'Fts'):
    cls = getattr(zvec, name)
    if hasattr(cls, '__init__'):
        sig = inspect.signature(cls.__init__)
        print(f'  {name}: {sig}')
print()
print('=== query method sig ===')
print(inspect.signature(zvec.Collection.query))
print()
print('=== VectorQuery doc ===')
print((zvec.VectorQuery.__doc__ or '').split(chr(10))[:20])
print()
print('=== Fts doc ===')
print((zvec.Fts.__doc__ or '').split(chr(10))[:20])
