#!/usr/bin/env python3
"""Probe zvec API on Windows."""
import zvec
import sys

print("=== zvec version ===")
print(getattr(zvec, '__version__', 'unknown'))
print()

print("=== module attrs ===")
for a in sorted(dir(zvec)):
    if not a.startswith('_'):
        obj = getattr(zvec, a)
        if isinstance(obj, type):
            print(f"  CLASS {a}")
        elif callable(obj):
            doc = obj.__doc__ or ''
            first_line = doc.split('\n')[0] if doc else '(no doc)'
            print(f"  fn  {a}: {first_line[:100]}")
        else:
            print(f"  var {a}: {obj}")
