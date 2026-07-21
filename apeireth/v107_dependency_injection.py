"""V107 真生产 dependency injection (主 22:10 一次几十)."""
from __future__ import annotations
V107_VERSION = "0.1.0"


class V107DependencyInjection:
    def __init__(self):
        self.dependencies = {}
        self.resolved = []
        self.n = 0
        self.nph = 0
        self.nas = 0

    def register(self, name, instance):
        self.dependencies[name] = instance

    def resolve(self, name):
        if name in self.dependencies:
            inst = self.dependencies[name]
            if name not in self.resolved:
                self.resolved.append(name)
            return inst
        return None

    def stats(self):
        return {"n_dependencies": len(self.dependencies),
                "n_resolved": len(self.resolved),
                "version": V107_VERSION,
                "philosophy": "V107 DI (主 19:33 + 真借鉴)"}


__all__ = ["V107_VERSION", "V107DependencyInjection"]