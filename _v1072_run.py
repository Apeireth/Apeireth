"""V1072 真测."""
import sys
sys.path.insert(0, '.')

try:
    from apeireth.v1072_asi_central_ai_eternal_identity import v1072_bridge_measure, v1072_run
    result = v1072_run()
    measure = result['measure']
    print(f"V1072 raw: {measure['raw']:.4f}")
    for k, v in measure['components'].items():
        print(f"  {k}: {v:.4f}")
except Exception as e:
    print(f"Err: {e}")
    import traceback
    traceback.print_exc()