"""Fix test file: def fake(x) → def fake(tool_input)."""
import re

src = open(r".openclaw\workspace\promethean\tests\test_tool_runner_borrow.py", encoding="utf-8").read()

# Replace def fake(x): return x * 2
src = re.sub(
    r"def fake\(x\):\s*\n\s*return x \* 2",
    "def fake(tool_input):\n        return tool_input['x'] * 2",
    src,
)
# Replace def fake(x): return x
src = re.sub(
    r"def fake\(x\):\s*\n\s*return x\b",
    "def fake(tool_input):\n        return tool_input['x']",
    src,
)
# Replace lambda x: x
src = re.sub(r"lambda x: x\b", "lambda tool_input: tool_input['x']", src)
# Replace lambda x: x * 2
src = re.sub(r"lambda x: x \* 2", "lambda tool_input: tool_input['x'] * 2", src)

open(r".openclaw\workspace\promethean\tests\test_tool_runner_borrow.py", "w", encoding="utf-8").write(src)
print("fixed")