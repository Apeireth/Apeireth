"""One-shot kitchen sanity aggregate for cron wake.

Not a v-module — just a cron-tick verifier.
Anyone接手 can run: python _v1266_sanity_aggregate.py
"""
from apeireth import v1263_real_kitchen_integration as v1263
from apeireth import v1264_kitchen_north_star_integration as v1264
from apeireth import v1265_kitchen_reproducibility_audit as v1265
from apeireth import v1266_r18_multi_run_reproducibility as v1266


def main() -> int:
    r = v1263.sanity_check_1263()
    total = len(r)
    passed = sum(1 for v in r.values() if v)
    print(f"V1263 sanity: {passed}/{total}")
    for k, v in r.items():
        marker = "PASS" if v else "FAIL"
        print(f"  [{marker}] {k}")
    print()
    print(f"V1264 version: {v1264.V1264_VERSION} build: {v1264.V1264_BUILD_TS}")
    print(f"V1264 V3 guards: {len(v1264.V3_GUARDS_1264)} (north_star_is_not_asi etc.)")
    print(f"V1265 version: {v1265.V1265_VERSION} guards: {len(v1265.V3_GUARDS)}")
    print(f"V1266 version: {v1266.V1266_VERSION} guards: {len(v1266.V3_GUARDS)}")
    print()
    print(f"Kitchen 4-module aggregate: {passed}/{total} sanity OK")
    return 0 if passed == total else 1


if __name__ == "__main__":
    raise SystemExit(main())