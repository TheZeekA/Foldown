# Security policy

## Reporting a vulnerability

Please do not open a public issue for a suspected security vulnerability.
Email the maintainer at security@zeeka.nz with the affected version, steps to
reproduce, and any relevant logs or proof of concept. Do not include API keys,
license keys, private signing keys, or personal workspace content.

If you discover that a secret has been committed, revoke or rotate it first,
then contact the maintainer with the commit and file location.

## Scope

The public repository contains the application source and documentation. API
credentials, updater signing keys, release certificates, and production
service credentials must be supplied through local development configuration
or a protected CI/CD secret store and must never be committed.
