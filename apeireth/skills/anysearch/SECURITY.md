# Security Policy

## Reporting a Vulnerability

If you discover a security vulnerability in this project, please report it responsibly.

**Do NOT open a public GitHub issue for security vulnerabilities.**

### How to Report

Send an email to **security@anysearch.com** with:

- Description of the vulnerability
- Steps to reproduce
- Potential impact
- Suggested fix (if any)

### Response Timeline

| Action | Timeframe |
|--------|-----------|
| Acknowledgment | Within 48 hours |
| Initial assessment | Within 5 business days |
| Fix release | Depends on severity |

### Scope

This policy covers:
- This repository's skill definition and configuration examples
- CLI scripts under `scripts/`
- Official documentation (`SKILL.md`, `README.md`)

### Out of Scope

- The AnySearch API backend (`api.anysearch.com`)
- Third-party AI agent platforms consuming this skill
- User misconfiguration of API keys

## Supported Versions

| Version | Supported |
|---------|-----------|
| Latest  | Yes       |

## Security Best Practices for Users

- Store API keys in environment variables, never in code
- Use `.env` files locally (already in `.gitignore`)
- Rotate API keys periodically
- Use the minimum required permissions
