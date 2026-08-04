"""Quick inspection of V1260 stack layout (主 17:43 实事求是)."""
import v1260_docker_deploy as m

print("=== build_default_stack ===")
for svc in m.build_default_stack():
    cmd = "/".join(svc.command[:2]) if svc.command else None
    print(f"  - {svc.name}: port={svc.port}, cmd_prefix={cmd}")

print()
print("=== build_e2e_stack ===")
for svc in m.build_e2e_stack():
    cmd = "/".join(svc.command[:2]) if svc.command else None
    print(f"  - {svc.name}: port={svc.port}, cmd_prefix={cmd}")

print()
print("=== Probe summary ===")
print(m.probe_summary_dict())