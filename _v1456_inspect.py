import json
with open('.v1456-six-deployment-real-execution-parity-report.json','r',encoding='utf-8') as f:
    data = json.load(f)
for p in data['profiles']:
    print(f"{p['module_id']}: success={p['success']} rc={p['return_code']} latency={p['latency_ms']:.1f}ms parity={p['parity_score']:.4f} stdout={p['stdout_lines']} stderr={p['stderr_lines']}")
    if p['error_message']:
        print(f"  ERROR: {p['error_message'][:300]}")
    if p['output_summary']:
        s = json.dumps(p['output_summary'], default=str)
        print(f"  OUTPUT: {s[:300]}")
