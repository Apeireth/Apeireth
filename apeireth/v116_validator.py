"""V116 真生产 schema validator (主 22:10 一次几十)."""
from __future__ import annotations
import uuid
V116_VERSION = "0.1.0"


class V116Validator:
    def __init__(self):
        self.rules = {}
        self.validations = []
        self.n = 0
        self.nph = 0
        self.nas = 0

    def add_rule(self, field, rule_type, params=None):
        rid = f"rule_{uuid.uuid4().hex[:12]}"
        self.rules[rid] = {"field": field, "type": rule_type, "params": params or {}}

    def validate(self, data):
        vid = f"val_{uuid.uuid4().hex[:12]}"
        errors = []
        for rid, rule in self.rules.items():
            field = rule["field"]
            if field not in data:
                errors.append(f"missing {field}")
            elif rule["type"] == "min_length" and len(str(data[field])) < rule["params"].get("min", 0):
                errors.append(f"{field} too short")
        is_valid = len(errors) == 0
        self.validations.append({"id": vid, "is_valid": is_valid, "errors": errors})
        self.n += 1
        return is_valid

    def stats(self):
        return {"n_validations": self.n, "n_rules": len(self.rules),
                "version": V116_VERSION,
                "philosophy": "V116 validator (主 19:33 + JSON schema + V67 真借鉴)"}


__all__ = ["V116_VERSION", "V116Validator"]