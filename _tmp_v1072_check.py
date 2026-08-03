"""V1072 真测当前 score."""
from apeireth.v1072_asi_central_ai_eternal_identity import v1072_bridge_measure, v1072_run
import json

m = v1072_bridge_measure()
print(f"v1072 bridge measure: {m:.4f}")

result = v1072_run()
print(f"raw: {result['measure']['raw']:.4f}")
print("components:")
print(json.dumps(result["measure"]["components"], indent=2))
