import re
p = r'.openclaw/workspace/promethean/reports/r7-design-01-architecture-blueprint.md'
raw = open(p, 'rb').read()
text = raw.decode('utf-8')
print('bytes:', len(raw), 'lines:', raw.count(b'\n') + 1)

# 1. L0-L7 layers (8 layers)
layers = ['L0', 'L1', 'L2', 'L3', 'L4', 'L5', 'L6', 'L7']
for L in layers:
    print(f'  {L}: {text.count(L)}')

# 2. R7 Gantt (Phase 1/2/3)
phases = ['Phase 1', 'Phase 2', 'Phase 3', 'P1 ∥', 'P2', 'P3']
for ph in phases:
    print(f'  phase {ph}: {text.count(ph)}')

# 3. Interface × Guard table
print('interfaces (backticked names):')
funcs = re.findall(r'`([a-z_]+)`', text)
print('  total:', len(funcs), 'unique:', len(set(funcs)))
guards = ['V3', 'V1072', 'V1074', 'V1081']
for g in guards:
    print(f'  guard {g}: {text.count(g)}')

# 4. Prompt integration
prompts = ['Dream', 'Replay', 'HotCold', 'GUARDS', 'consolidate|decay|no_op', 'observed|inferred|unknown', 'retain']
for p_ in prompts:
    print(f'  prompt {p_}: {text.count(p_)}')

# 5. MCP tools
mcp_tools = ['hqb_record_decision', 'hqb_record_guard_event', 'hqb_record_delta', 'hqb_record_trace',
             'hqb_query_decisions', 'hqb_get_decision_trace', 'hqb_stats']
for t in mcp_tools:
    print(f'  mcp {t}: {text.count(t)}')

# 6. Main philosophy v3 ≥ 8
phil = ['主17:58', '主23:44', '主19:33', '主22:33', '主23:28', '主12:07', '主21:15',
        'R6新增1', 'R6新增2', 'R7新增']
for q in phil:
    print(f'  phil {q}: {text.count(q)}')

# 7. Integration references
ints = ['R5-AS-02', 'R6-INT-01', 'R6-DOC-01', 'R7-ORC-01', 'R7-BE-01-DESIGN',
        'R6-RES-06', 'R6-RES-07', 'R6-PHL-03', 'R7-PROMPT-01', 'R7-MCP-01',
        'R7-WF-01', 'R7-WF-02']
for i in ints:
    print(f'  ref {i}: {text.count(i)}')

# 8. Acceptance: pytest + G + HQB
acc = ['pytest', 'V1074', 'V1082', 'record_decision', 'PASS']
for a in acc:
    print(f'  acceptance {a}: {text.count(a)}')