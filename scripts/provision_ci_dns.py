#!/usr/bin/env python3
"""Bootstrap temporary Linux CI DNS before installing system dependencies.

Healthy DNS is unchanged. Recovery updates only the runner's volatile per-link
resolver settings; it never writes host configuration or changes APT trust.
This is environment provisioning, not a governance gate or a test substitute.
"""

from __future__ import annotations

import argparse
import ipaddress
import json
import os
import re
import subprocess
import sys
import time

from utf8_stdout import force_utf8_stdout

DOMAINS = ("azure.archive.ubuntu.com", "archive.ubuntu.com", "security.ubuntu.com")
FALLBACK_DNS = ("1.1.1.1", "8.8.8.8")


def provision(environ: dict[str, str], run=subprocess.run, pause=time.sleep) -> None:
    if environ.get("GITHUB_ACTIONS") != "true" or environ.get("RUNNER_OS") != "Linux":
        raise ValueError("CI only: GITHUB_ACTIONS=true and RUNNER_OS=Linux are required")

    def command(argv: list[str], *, timeout: int = 10, check: bool = True):
        return run(
            argv, check=check, timeout=timeout, capture_output=True,
            text=True, encoding="utf-8", errors="strict",
        )

    def probe() -> bool:
        failed = []
        for domain in DOMAINS:
            try:
                result = command(["getent", "ahostsv4", domain], timeout=5, check=False)
            except subprocess.TimeoutExpired:
                failed.append(f"{domain} (timeout after 5s)")
                continue
            addresses = [line.split()[0] for line in result.stdout.splitlines() if line.split()]
            valid = False
            for address in addresses:
                try:
                    ipaddress.IPv4Address(address)
                    valid = True
                except ipaddress.AddressValueError:
                    continue
            if result.returncode:
                failed.append(f"{domain} (exit {result.returncode})")
            elif not valid:
                failed.append(f"{domain} (no IPv4)")
        if failed:
            print("DNS preflight failed: " + ", ".join(failed))
        return not failed

    if probe():
        print("CI DNS preflight passed; resolver settings unchanged")
        return

    routes = json.loads(command(["ip", "-json", "route", "show", "default"]).stdout)
    if not isinstance(routes, list) or not routes:
        raise ValueError("no default IPv4 route is available for temporary DNS recovery")
    candidates = []
    for route in routes:
        interface = route.get("dev") if isinstance(route, dict) else None
        if not isinstance(interface, str) or not re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9_.:-]{0,14}", interface):
            raise ValueError("default route contains an invalid network interface")
        if interface == "lo" or route.get("type", "unicast") != "unicast":
            raise ValueError("default route must use a unicast network interface")
        metric = route.get("metric", 0)
        if not isinstance(metric, int) or isinstance(metric, bool) or metric < 0:
            raise ValueError("default route contains an invalid metric")
        candidates.append((metric, interface))
    interface = min(candidates)[1]
    recovery_attempted = False
    try:
        # A timeout can happen after the resolver accepted the change.
        recovery_attempted = True
        command(["sudo", "-n", "resolvectl", "dns", interface, *FALLBACK_DNS])
        command(["sudo", "-n", "resolvectl", "domain", interface, "~ubuntu.com"])
        command(["sudo", "-n", "resolvectl", "default-route", interface, "yes"])
        command(["sudo", "-n", "resolvectl", "flush-caches"])
        for attempt in range(3):
            if probe():
                break
            if attempt == 2:
                raise ValueError("temporary CI DNS recovery failed; system dependencies remain unverified")
            pause(1)
    except (OSError, ValueError, subprocess.SubprocessError):
        if recovery_attempted:
            try:
                command(["sudo", "-n", "resolvectl", "revert", interface])
                command(["sudo", "-n", "resolvectl", "flush-caches"])
            except (OSError, ValueError, subprocess.SubprocessError) as error:
                sys.stderr.write(f"DNS recovery cleanup failed: {error}\n")
        raise
    print(f"CI DNS preflight passed after temporary recovery on {interface}")


def main() -> int:
    argparse.ArgumentParser(description=__doc__).parse_args()
    force_utf8_stdout()
    try:
        provision(dict(os.environ))
    except (OSError, ValueError, subprocess.SubprocessError) as error:
        sys.stderr.write(f"FAIL: CI DNS provisioning: {error}\n")
        if isinstance(error, subprocess.CalledProcessError) and error.stderr:
            sys.stderr.write(error.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
