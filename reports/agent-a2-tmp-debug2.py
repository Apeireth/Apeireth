import yaml
from pathlib import Path

# Test bug_report.yml
text = Path(r'.github\ISSUE_TEMPLATE\bug_report.yml').read_text(encoding='utf-8')
data = yaml.safe_load(text)
dumped = yaml.dump(data, allow_unicode=True, default_flow_style=False)
print('=== bug_report.yml ===')
print('raw contains 8 item no change:', '8 项不修改承诺' in text)
print('dumped contains 8 item no change:', '8 项不修改承诺' in dumped)
print('dumped length:', len(dumped))
# print first 500 chars of dumped
print('dumped head 500:')
print(dumped[:500])
print('---')
# Find where 8 item is
if '8 项不修改承诺' in dumped:
    idx = dumped.find('8 项不修改承诺')
    print('Found at idx', idx, 'context:', dumped[max(0,idx-50):idx+50])
