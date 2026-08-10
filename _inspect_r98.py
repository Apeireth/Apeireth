"""Inspect round-98 bocha_web / bocha_ai deep structure."""
import json

data = json.load(open(r".openclaw\workspace\promethean\research-v7-round-98.json", "r", encoding="utf-8"))
q0 = data["queries"][0]
bw = q0["result"]["sources"]["bocha_web"]
print("bocha_web keys:", list(bw.keys()))
for k, v in bw.items():
    v_len = len(v) if hasattr(v, "__len__") else "n/a"
    print(f"  {k}: type={type(v).__name__} len_or_value={v_len}")

print()
print("bocha_web sample (first 600 chars):")
bws = json.dumps(bw, ensure_ascii=False)
print(bws[:600])
print()
print("bocha_ai sample (first 600 chars):")
ba = q0["result"]["sources"]["bocha_ai"]
bas = json.dumps(ba, ensure_ascii=False)
print(bas[:600])
