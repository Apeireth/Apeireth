"""清理测试文件中错误的转义."""
src = open(r".openclaw\workspace\promethean\tests\test_llm_kernel_patch_borrow.py", encoding="utf-8").read()
# 修复被错误转义的 proposer
src = src.replace('proposer=\\"evolution\\"', 'proposer="evolution"')
open(r".openclaw\workspace\promethean\tests\test_llm_kernel_patch_borrow.py", "w", encoding="utf-8").write(src)
print("cleaned")