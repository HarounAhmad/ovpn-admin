# OVPN Admin

A lightweight admin panel and JSON API for managing OpenVPN client certificates and runtime sessions. It provides:

- A secure HTTP API (under `/api`) for automation and integrations
- An embedded SPA UI (under `/ui`) for administrators
- Authentication with cookie sessions, CSRF protection, and role-based authorization
- Certificate lifecycle operations (issue, bundle, revoke) delegated to vpn-certd
- Client-specific configuration (CCD) file management
- OpenVPN Management Interface (mgmt) integration for live client status and session control
- Audit logging and login throttling, backed by SQLite via SQLx

> Important: All certificate operations are performed by an external daemon, vpn-certd. You must install and run vpn-certd, and configure its Unix socket path in `config/dev.toml` (or environment) for issuing/revoking certificates, bundle generation, CRL, and listing issued certs.


## Table of Contents

- Overview and Use Cases
- Architecture
- Security Model
- API Reference
- Configuration
- Database Schema
- OpenVPN Management Integration
- Frontend
- Running Locally
- Deployment Notes
- CLI
- Directory Structure
- Troubleshooting
- Roadmap


## Overview and Use Cases

- Admins issue VPN client certificates with optional passphrase and CCD customization.
- Admins download client bundles (zip) ready for clients (ovpn config, certs, optional key).
- Admins revoke certificates by CN or serial; revocation visible via CRL.
- Admins view connected clients from OpenVPN’s management interface and can kick sessions.
- All logins and admin actions are recorded in an audit log.
- External systems can automate operations via the JSON API using cookie sessions + CSRF.


## Architecture

- Language/Runtime
  - Rust (Edition 2024)
  - Async runtime: Tokio
- HTTP Server and Middleware
  - axum (0.6.x) for routing and handlers
  - tower-http for security and cache headers
  - tracing / tracing-subscriber for structured logs
- Database
  - SQLite via SQLx (0.7), migrations auto-run at startup
- Crypto / Security
  - Argon2 (argon2id) for password hashing with per-deployment pepper
  - cookie crate for secure cookie handling
  - regex for CN validation; rand/base64 utilities
  - openssl crate (vendored) for CRL parsing and serial decoding
- Static UI Assets
  - rust-embed to bundle `webui-dist` files into the binary
  - mime_guess for content types
- External Dependencies
  - vpn-certd (required) for all certificate lifecycle actions (issue, bundle, revoke, CRL, list)
  - OpenVPN Management Interface (optional) over Unix socket for live client stats and session control

High-level dataflow:

1) Browser loads `/ui/` from the server (embedded SPA).
2) SPA fetches `/api/auth/csrf` to get CSRF cookie, then logs in via `/api/auth/login`.
3) Admin actions call `/api/admin/*`; the server consults roles and forwards cert ops to vpn-certd.
4) For live client info, the server periodically polls the mgmt socket (`status 3`), caches rows, and serves snapshots.


## Security Model

- Password Hashing and Pepper
  - Argon2id with parameters ~64MB memory, t=2, p=1
  - Pepper bytes are read from `server.pepper_file` (must be ≥16 bytes) and used as Argon2 secret
- Sessions
  - Server-set cookie (configurable name, e.g., `OVPNSESS`), `HttpOnly`, `Secure`, `SameSite=Strict`
  - Session rows persist in DB with expiration; validated on every request needing auth
- CSRF Protection
  - `GET /api/auth/csrf` sets a non-HttpOnly cookie `XSRF-TOKEN`
  - All modifying requests (POST/PUT/PATCH/DELETE) must send header `X-CSRF-Token` matching the cookie
- Role-based Authorization
  - Roles: `ADMIN`, `OPS`, `READONLY` (today, admin-only for sensitive endpoints)
  - Server checks roles via middleware extractors before executing handlers
- Security Headers (via tower-http)
  - CSP: self for scripts, unsafe-inline for styles; images self+data
  - Referrer-Policy: no-referrer
  - X-Content-Type-Options: nosniff
  - X-Frame-Options: DENY
  - HSTS: max-age=31536000; includeSubDomains (deploy only behind TLS)
- Throttling
  - Login attempts logged; per-IP and per-username+IP counters over a sliding window
  - Exceeds threshold → 429 Too Many Requests


## API Reference

Base URL: `/api`

Auth and Session
- `GET /auth/csrf` → 204 No Content; sets `XSRF-TOKEN` cookie
- `POST /auth/login` → 204 No Content; sets session cookie
  - Body: `{ "username": "...", "password": "..." }`
  - 401 for bad credentials/disabled; 429 when throttled
- `POST /auth/logout` → 204 No Content; clears session cookie
- `GET /me` → 200 `{ "username": "...", "roles": ["ADMIN", ...] }` or 401

Health
- `GET /health` → 200 `{ "api": {"ok": true}, "daemon": {"ok": bool}, "agent": {"ok": bool} }`

Admin – Certificates (ADMIN only)
- `POST /admin/clients`
  - Body: `{ "cn": "client1", "passphrase"?: "...", "include_key"?: bool, "ccd"?: "push ..." }`
  - 201 `{ "cn", "passphrase", "serial"?: "...", "not_after"?: "..." }`
  - Errors: 422 `invalid_cn`, 409 `cn_exists_active`, 502 `daemon_error`
- `POST /admin/clients/:cn/revoke`
  - 204 on success
  - Errors: 422 `invalid_cn`, 404 `cn_not_found`, 409 `already_revoked`, 502 `daemon_error`
- `POST /admin/clients/:cn/bundle`
  - Body: `{ "include_key"?: bool }`
  - 200 application/zip with `Content-Disposition: attachment; filename="<cn>.zip"`

Admin – CCD (ADMIN only)
- `GET  /admin/ccd` → 200 `[{ "cn": "...", "size": 123, "modified": 1710000000 }, ...]`
- `GET  /admin/ccd/:cn` → 200 `{ "cn": "...", "content": "..." }`
- `PUT  /admin/ccd/:cn` Body: `{ "content": "..." }` → 204

Admin – Issued Certs (ADMIN only)
- `GET /admin/issued?limit=50` → 200 `[{ "serial", "cn", "profile", "not_after", "revoked": bool, "revoked_at"?: "..." }, ...]`

Admin – OpenVPN Mgmt (ADMIN only; requires mgmt.enabled)
- `GET  /admin/mgmt/clients` → 200 `[{ "cn", "real_ip", "vpn_ip", "bytes_in", "bytes_out", "connected_since", "client_id"?, "peer_id"? }, ...]`
- `POST /admin/mgmt/clients/kick` Body: `{ "client_id"?: number, "cn"?: string }` → 204; 400 invalid, 503 disabled, 502 mgmt error

Misc
- `GET /protected/admin-ping` (ADMIN) → 200 "pong"

CSRF Requirements
- For POST/PUT/PATCH/DELETE: include header `X-CSRF-Token` that matches the `XSRF-TOKEN` cookie


## Configuration

Configuration sources:
- TOML file `config/dev` (i.e., `config/dev.toml`) – optional
- Environment variables with prefix `OVPNADM__` and `__` as nesting separator

Example `config/dev.toml`:

```toml
[server]
bind = "127.0.0.1:8080"
cookie_name = "OVPNSESS"
session_ttl_secs = 900
pepper_file = "dev.pepper"

[db]
url = "sqlite://var/ovpn-admin.sqlite?mode=rwc"

[ovpn]
socket_path     = "/var/run/vpn-certd.sock"
ccd_dir         = "/etc/openvpn/ccd"
cn_pattern      = "^[A-Za-z0-9._-]{3,64}$"
bundle_remote   = "vpn.example.com"
bundle_port     = 1194
bundle_proto    = "udp"
bundles_dir     = "/var/lib/ovpn-admin/bundles"

[mgmt]
enabled = true
socket = "/var/run/openvpn-mgmt.sock"
poll_secs = 5
```

Environment overrides:
- `OVPNADM__SERVER__BIND=0.0.0.0:8080`
- `OVPNADM__DB__URL=sqlite:///data/ovpn.sqlite?mode=rwc`
- `OVPNADM__OVPN__SOCKET_PATH=/var/run/vpn-certd.sock`

Notes:
- Pepper file must contain ≥16 random bytes; generate via `head -c 32 /dev/urandom > dev.pepper`.
- The process must have read access to `pepper_file`, RW access to SQLite location, and RW to `ccd_dir` and `bundles_dir`.


## Database Schema

Migrations live under `migrations/` and run on startup. Key tables:

- `users(id, username UNIQUE, pw_hash, require_pw_change, disabled, created_at, updated_at)`
- `roles(name PRIMARY KEY)` seeded with `ADMIN`, `OPS`, `READONLY`
- `user_roles(user_id, role_name)` composite PK
- `sessions(id, user_id, created_at, expires_at, last_auth_stepup)`
- `audit(id, ts, actor_user, action, target, ip, ua, details)`
- `login_attempts(username, ts, ip)` for throttling

Identifiers use ULIDs. `last_auth_stepup` exists for future step-up flows.


## OpenVPN Management Integration (Optional)

If enabled, the server periodically connects to the mgmt Unix socket and runs `status 3`, parses CSV rows, and keeps an in-memory snapshot. It also supports kicking a client by `client_id` or CN.

Development mock server:

```bash
./mock-openvpn-mgmt.sh start /tmp/openvpn-mock.sock
./mock-openvpn-mgmt.sh seed
./mock-openvpn-mgmt.sh ls
./mock-openvpn-mgmt.sh clear
```


## Frontend

- Svelte + TypeScript + Vite in `frontend/`
- Build artifacts copied/available under `webui-dist/` and embedded by `rust-embed`
- Served at `/ui` with immutable cache headers for non-index assets

Dev workflow:

```bash
cd frontend
npm ci
npm run build   # produces webui-dist
cd ..
cargo run       # serves API and UI
```


## Running Locally

Prerequisites:
- Rust toolchain, Node.js for building the frontend
- vpn-certd installed and running (Unix socket path available)

Steps:

```bash
# 1) Generate a pepper
head -c 32 /dev/urandom > dev.pepper

# 2) (Optional) Start mgmt mock for UI/mgmt endpoints
./mock-openvpn-mgmt.sh start /tmp/openvpn-mock.sock
./mock-openvpn-mgmt.sh seed

# 3) Configure config/dev.toml to match your paths
#    (pepper file, SQLite URL, vpn-certd socket, CCD/bundles dirs)

# 4) Create an admin user
cargo run -- user-add --username admin --role ADMIN

# 5) Start the server
cargo run
```

Login via curl:

```bash
# Get CSRF cookie
curl -i -c /tmp/c -b /tmp/c http://127.0.0.1:8080/api/auth/csrf

# Extract token and login
TOKEN=$(grep XSRF-TOKEN /tmp/c | tail -n1 | awk '{print $7}')
curl -i -c /tmp/c -b /tmp/c \
  -H "X-CSRF-Token: $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"username":"admin","password":"..."}' \
  http://127.0.0.1:8080/api/auth/login

# Verify session
curl -i -c /tmp/c -b /tmp/c http://127.0.0.1:8080/api/me
```

Note: Without a running vpn-certd at the configured socket, health checks and all certificate-related endpoints will fail (502).


## Deployment Notes

- TLS: The server speaks plain HTTP; deploy behind a TLS reverse proxy (nginx, Caddy, HAProxy). HSTS is set; ensure HTTPS-only in production.
- Permissions: Grant the service user access to `pepper_file`, SQLite file/dir, `ccd_dir`, `bundles_dir`, and the vpn-certd and mgmt Unix sockets.
- Observability: Logs via `tracing_subscriber` at INFO by default. Consider centralizing logs if running under systemd/docker.
- Scaling: Async I/O via axum/Tokio; SQLite will limit write concurrency. For higher throughput, consider moving to a networked DB (the code currently targets SQLite).


## CLI

```bash
# Create user and assign role
cargo run -- user-add --username <USER> --role <ADMIN|AUDIT>
```

Interactive password prompt is displayed; the user is created and role assigned.


## Directory Structure

```
.
├── src/
│   ├── main.rs            # startup, CLI, router wiring
│   ├── config.rs          # config parsing, pepper loading
│   ├── db/                # SQLx helpers (users, sessions, roles, audit, throttling)
│   ├── http/              # routes: auth, admin, guards, csrf, middleware
│   ├── openvpn/           # mgmt integration, CCD ops, bundle streaming, CRL & issued
│   ├── vpncertd.rs        # vpn-certd JSON-over-UDS client
│   └── web.rs             # embedded SPA router
├── migrations/            # SQLx migrations
├── frontend/              # Svelte/Vite app (build to webui-dist)
├── webui-dist/            # built SPA served by rust-embed
├── config/dev.toml        # sample dev configuration
├── mock-openvpn-mgmt.sh   # dev mock for mgmt interface
└── var/ovpn-admin.sqlite  # default SQLite path (dev)
```


## Troubleshooting

- vpn-certd errors (502 `daemon_error`)
  - Ensure vpn-certd is running and `ovpn.socket_path` is correct and accessible
- Mgmt health false / 503
  - Verify `mgmt.socket` path and that the OpenVPN (or mock) mgmt is listening and accessible
- CSRF 403
  - Ensure you first called `/api/auth/csrf` and send `X-CSRF-Token` matching `XSRF-TOKEN` cookie
- 429 Too Many Requests on login
  - Wait for the throttle window to pass (600s in code) or reduce attempts


## Roadmap

- Step-up authentication using `last_auth_stepup`
- Refine OPS/READONLY endpoint capabilities
- API tokens for machine-to-machine usage
- Expanded audit context for admin actions


---

Built with Rust, axum, SQLx, and Svelte. Certificate operations powered by vpn-certd.

