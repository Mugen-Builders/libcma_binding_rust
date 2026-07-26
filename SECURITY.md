# Security Policy

## Supported versions

The crate is pre-1.0 and under active development. Only the latest published
`0.0.x` release receives security fixes.

| Version          | Supported          |
| ---------------- | ------------------ |
| 0.0.x (latest)   | :white_check_mark: |
| older 0.0.x      | :x:                |

## Reporting a vulnerability

Please report security vulnerabilities **privately** — do not open a public
issue or pull request.

- Preferred: open a private advisory via GitHub Security Advisories on the
  [repository](https://github.com/Mugen-Builders/libcma_binding_rust/security/advisories/new).
- Alternatively, email the maintainer: **idogwuchi@gmail.com**.

Please include a description, the affected version(s), and reproduction steps
where possible. You can expect an initial acknowledgement within a reasonable
time frame; we will then coordinate a fix and a disclosure timeline with you.

## Scope note

The default `mock` backend is an in-memory stub for development and testing — it
is **not** the real libcma ledger and must not be used to custody real assets in
production. Use the `host-real` or `riscv64` backends for real deployments.
