with open(r'.openclaw\workspace\promethean\apeireth\v1101_asi_v04_dim_lift.py', 'r', encoding='utf-8') as f:
    content = f.read()
# Replace emojis with ASCII
content = content.replace('\u2705', '[OK]')
content = content.replace('\u274c', '[FAIL]')
content = content.replace('\u2757', '[!]')
content = content.replace('\u26a0\ufe0f', '[!]')
content = content.replace('\U0001f50d', '[search]')
content = content.replace('\U0001f4dd', '[doc]')
with open(r'.openclaw\workspace\promethean\apeireth\v1101_asi_v04_dim_lift.py', 'w', encoding='utf-8') as f:
    f.write(content)
print('replaced. size:', len(content))
import re
# count remaining non-ascii
n_non_ascii = sum(1 for c in content if ord(c) > 127)
print('non-ascii chars remaining:', n_non_ascii)