with open(r'.openclaw\workspace\promethean\apeireth\v1101_asi_v04_dim_lift.py', 'r', encoding='utf-8') as f:
    content = f.read()
print('CHECK Report line:')
for i, line in enumerate(content.split('\n')):
    if 'Report:' in line:
        print(i+1, repr(line))