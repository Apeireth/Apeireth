"""Run full pytest suite with logging, in case pytest CLI capture is broken."""
import subprocess, sys, time
out = r"AppData\Local\Temp\pytest_full_log.txt"
err = r"AppData\Local\Temp\pytest_full_err.txt"
print(f"start: {time.strftime('%H:%M:%S')}")
with open(out, "wb") as fo, open(err, "wb") as fe:
    p = subprocess.Popen(
        [sys.executable, "-m", "pytest", "tests/", "-q", "--tb=no",
         "--timeout=30", "-p", "no:cacheprovider", "--no-header",
         "-x", "--ignore=tests/test_v1269_asi_real_llm_stream_real_test.py",
         "-p", "no:asyncio"],
        cwd=r".openclaw\workspace\promethean",
        stdout=fo, stderr=fe,
    )
    rc = p.wait(timeout=900)
print(f"end rc={rc}: {time.strftime('%H:%M:%S')}")