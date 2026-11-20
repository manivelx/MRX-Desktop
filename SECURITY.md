# Security Policy

MRX is experimental and is not recommended for security-critical or daily-use environments.

## Reporting a vulnerability

Please do not publish exploitable vulnerabilities, sensitive logs, credentials or proof-of-concept payloads in public issues. Use GitHub's private vulnerability reporting feature if it is enabled for this repository. Otherwise contact the maintainers privately using the contact details on the repository profile.

Include the affected version or commit, environment, impact, reproduction steps and any suggested mitigation. Do not include real credentials or personal data.

## Scope

Security-sensitive areas include the local IPC socket, filesystem permissions, installer/uninstaller scripts, process management, X11 event handling and any future plugin mechanism.
