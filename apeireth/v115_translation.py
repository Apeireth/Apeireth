"""V115 真生产 translation/i18n (主 22:10 一次几十)."""
from __future__ import annotations
V115_VERSION = "0.1.0"


class V115Translation:
    def __init__(self):
        self.translations = {
            "hello": {"zh": "你好", "en": "hello"},
            "asi": {"zh": "超人工智能", "en": "ASI"},
            "apeireth": {"zh": "主真名", "en": "Apeireth"}
        }
        self.n = 0
        self.nph = 0
        self.nas = 0

    def translate(self, text, lang="zh"):
        if text in self.translations and lang in self.translations[text]:
            self.n += 1
            return self.translations[text][lang]
        return text

    def stats(self):
        return {"n_translations": self.n,
                "version": V115_VERSION,
                "philosophy": "V115 translation/i18n (主 19:33 + 真借鉴)"}


__all__ = ["V115_VERSION", "V115Translation"]