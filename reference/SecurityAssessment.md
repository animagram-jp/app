# Security Assessment

The most likely and the most impactful security problems of this software, and how each is handled.
Review this document before each release, together with the [quality checklist](https://github.com/animagram-jp/.github/blob/main/quality_checklist/README.md).

## Scope

| Item | Content |
|-|-|
| Client | The app compiled to wasm32, `distribution/` (HTML, JavaScript, CSS) and the service worker. |
| Server | The `server` binary (feature `server`): account, session, sync and meet. See [Server.md](./Server.md). |
| Pipeline | `.github/workflows/` and the deploy destinations (Cloudflare Workers, GitHub Pages). |

## Actor

| Actor | Trust | Action |
|-|-|-|
| User | Trusted for the own data only. | Edits records in the browser, logs in, syncs between devices. |
| Remote client | Untrusted. | Sends any bytes to the server API. |
| Peer | Untrusted. | Exchanges signals through meet, then connects peer to peer. |
| Reverse proxy | Trusted. | Terminates TLS, limits rate, locks out, judges `Sec-Fetch-Site` and `Origin`. |
| Server operator | Trusted for availability, not for confidentiality. | Holds the ciphertext, the verifier and the pepper. |
| Deploy pipeline | Trusted. | Builds and serves the client code. |

## Asset

| Asset | Location |
|-|-|
| Plaintext record, `root_key`, `store_key`, `wrap_key`, password | Device only. |
| Session token | Cookie on the device. The server stores `SHA-256(token)`. |
| `verifier`, `wrapped_root`, ciphertext record | Server data directory. |
| Pepper | Credential of the process manager, outside the data directory. |
| Deploy secret | GitHub Actions secret, read by `deploy.yml` only. |

## Problem

| Problem | Likelihood | Impact | Handling |
|-|-|-|-|
| A malformed HTTP request or wire frame exhausts memory or crashes the server. | High | Medium | `http.rs` accepts a strict subset with limits on the header size and the header count, each set by an environment variable, and on the body size, derived from the largest push. A decode failure returns 400. Integer overflow panics in every profile (`overflow-checks = true`). The number of connection threads is fixed. |
| Online guessing of a password on `PUT /v1/session`. | High | High | Lockout and rate limit are delegated to the reverse proxy. The response and the time are the same for an absent user and a mismatch. A guess costs one Argon2id on the attacker side. The server depends on the proxy configuration for this handling. |
| The server data directory leaks. | Medium | High | `verifier` is `HMAC(pepper, authentication_key)` and the pepper is kept outside the directory. `wrapped_root` and records are XChaCha20-Poly1305 ciphertext whose keys exist on devices only. |
| The data directory and the pepper both leak. | Low | High | Each password candidate costs one Argon2id (m = 19 MiB, t = 2). A weak password stays exposed. |
| A session token is stolen or a cross-site request rides on the Cookie. | Medium | High | The Cookie is `HttpOnly; Secure; SameSite=Strict` with the `Path` set by `SERVER_SESSION_COOKIE_PATH`. The server keeps only the digest of the token. A session expires `SERVER_SESSION_LIFETIME_SECONDS` after creation and `SERVER_SESSION_INACTIVITY_SECONDS` after the last access. CSRF is judged by the reverse proxy. |
| The delivered client code is altered, by a compromised pipeline, dependency or host, and reads the keys on the device. | Low | Critical | Deploy runs only from a version tag or a manual dispatch, with actions pinned by commit SHA and `--locked` builds. Secrets are referenced by `deploy.yml` only. End-to-end encryption does not protect against this problem. |
| Activation of an account by an unintended person. | Medium | Medium | No public registration. Only an email in the allowlist activates, once. |
| A peer floods or spoofs meet signals. | Medium | Low | The server fills `from` from the session. A queue per target has a capacity and a lifetime, and a full queue returns 409. |
| Memory corruption in the memory shared between JavaScript and wasm. | Low | Medium | `unsafe` is confined to `arena.rs` and one site of `app.rs`. `undocumented_unsafe_blocks` and `unsafe_op_in_unsafe_fn` are forbidden by lint. The wasm sandbox bounds the reach to the app memory. |
| A vulnerable or malicious dependency (RustCrypto crates, getrandom, wasm-bindgen, npm tools). | Low | High | `Cargo.lock` and `package-lock.json` are committed. Dependabot updates weekly. CodeQL and OpenSSF Scorecard run in CI. |
