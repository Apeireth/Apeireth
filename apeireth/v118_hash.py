"""V118 真生产 hash/digest (主 22:10 一次几十)."""
from __future__ import annotations
import hashlib
V118_VERSION = "0.1.0"


class V118Hash:
    def __init__(self):
        self.hashes = {}
        self.n = 0
        self.nph = 0
        self.nas = 0

    def hash_sha256(self, text):
        h = hashlib.sha256(text.encode()).hexdigest()
        self.hashes[h] = text
        self.n += 1
        return h

    def hash_blake2(self, text):
        h = hashlib.blake2b(text.encode()).hexdigest()
        self.hashes[h] = text
        self.n += 1
        return h

    def verify(self, text, h):
        return self.hash_sha256(text) == h or self.hash_blake2(text) == h

    def stats(self):
        return {"n_hashes": self.n, "n_unique": len(self.hashes),
                "version": V118_VERSION,
                "philosophy": "V118 hash (主 19:33 + SHA256/BLAKE2 真借鉴)"}


__all__ = ["V118_VERSION", "V118Hash"]