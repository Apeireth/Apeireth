"""Quick V1201 inspection — what dims can still be lifted."""
import json

with open("artifacts/v1201_asi_v0611_dual_dim_lift.json", encoding="utf-8") as f:
    a = json.load(f)

print("V1201 ASI recompute:", a["asi_recompute_lifted"])
print("Gap to 0.98:", 0.98 - a["asi_recompute_lifted"])
print()
print("Dim lifts:")
for k, v in a.get("dim_lifts", {}).items():
    print(
        "  {name}: {baseline:.4f} -> {new:.4f} (Δ={delta:+.4f}, "
        "contrib={contrib:+.4f}, w={weight})".format(
            name=k,
            baseline=v["baseline"],
            new=v["new_value"],
            delta=v["delta"],
            contrib=v["lift_contribution"],
            weight=v["weight"],
        )
    )

# Look for prior baselines
print()
print("Dim baselines (sorted by gap_to_1):")
baselines = {}
for k, v in a.get("dim_lifts", {}).items():
    baselines[k] = v["new_value"]

# Let's also look at the formula details for what dims aren't listed yet
print("All dim keys in artifact:")
for k in a.keys():
    if isinstance(a[k], dict) and len(a[k]) < 30:
        print(" ", k)
