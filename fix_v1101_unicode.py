with open('apeireth/v1101_asi_v04_dim_lift.py', 'r', encoding='utf-8') as f:
    content = f.read()
content = content.replace('print(f"\\u2705 Report:', 'print(f"\\\\u2705 Report:')
# Now brute force: find any literal ? in a print/return that should be a checkmark
for line_marker in ['Report:', 'V1060', '认知能力', 'V3_GUARDS', 'autonomy', '真提升', 'measurement = ASI', 'cognitive_arch', '真应用', '真哲学守门', '主 17:58', 'apply = ASI', '?', '真拉']:
    content = content.replace(f'print(f"? {line_marker}', f'print(f"\\\\u2705 {line_marker}')
# Now check any line that has ? as part of print and isn't a regex
import re
final_lines = []
for line in content.split('\n'):
    if '?' in line:
        # Don't touch regex patterns or strings
        if not re.search(r'r["\']', line) and not re.search(r'\\d', line) and ('print' in line or 'raise' in line):
            if '?' in line and '\\u' not in line:
                # show for review
                final_lines.append(repr(line))
for l in final_lines[:20]:
    print(l)