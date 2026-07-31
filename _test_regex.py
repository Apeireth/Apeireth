import re
# 测试 [_=]? 是否影响
p1 = re.compile(r"\breached\s*asi\b", re.IGNORECASE)
p2 = re.compile(r"\breached[_=]?\s*asi\b", re.IGNORECASE)
p3 = re.compile(r"\breached[_=]\s*asi\b", re.IGNORECASE)
p4 = re.compile(r"\breached[_=]?\sasi\b", re.IGNORECASE)
print("1. \\breached\\s*asi\\b:", p1.search("breached asi"))
print("2. \\breached[_=]?\\s*asi\\b:", p2.search("breached asi"))
print("3. \\breached[_=]\\s*asi\\b:", p3.search("breached asi"))
print("4. \\breached[_=]?\\sasi\\b:", p4.search("breached asi"))
# Maybe regex engine issue with [_=]?
p5 = re.compile(r"\breached[xy]?\s*asi\b", re.IGNORECASE)
print("5. \\breached[xy]?\\s*asi\\b:", p5.search("breached asi"))
# 慢
p6 = re.compile(r"\bbreached[_=]?\s*asi\b", re.IGNORECASE)
print("6. \\bbreached[_=]?\\s*asi\\b:", p6.search("breached asi"))
p7 = re.compile(r"breached[_=]?\s*asi\b", re.IGNORECASE)
print("7. breached[_=]?\\s*asi\\b:", p7.search("breached asi"))