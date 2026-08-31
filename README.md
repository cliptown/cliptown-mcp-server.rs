# cliptown MCP server

This repository inherits from the hardened Rust MCP fleet libraries at
[`ORESoftware/mcp-rust-libs`](https://github.com/ORESoftware/mcp-rust-libs).
The exact `cliptown/cliptown-mcp-server.rs` organization, repository, service,
Zed package, and dependency identities are compile-time constants in
`src/spec.rs`; the server cannot silently drift into a generic fleet wrapper.

The same final MCP 2025-11-25 surface works with Cursor, ChatGPT/OpenAI,
Claude/Anthropic, Gemini, Grok, and Qwen. It is deliberately read-only and
exposes 15 no-argument tools:

- `org_identity` — canonical organization, repository, package, version, and
  access mode;
- `zed_dependency_graph` — Zed-owned dependency intent and `.vendor/.zed`
  materialization policy;
- `telemetry_status` — non-sensitive `ores-otel` traces, metrics, logs, and
  structured-stderr initialization status;
- `shared_auth_policy` — the fail-closed identity/realm boundary and whether a
  public Shared Auth authority is configured, without accepting credentials;
- `environment_policy` — SOPS+age ciphertext and ignored-plaintext rules;
- `security_baseline` — mutation, output-bound, logging, auth, dependency, and
  secret-handling guarantees;
- `github_repository_read` — the exact Cliptown repository and CI posture;
- `aws_account_read` — the configured AWS account and bounded EKS inventory;
- `gcp_project_read` — exact GCP project metadata;
- `supabase_project_read` — the configured Cliptown Supabase health boundary;
- `neon_project_read` — exact Neon organization/project metadata;
- `cloudflare_account_read` — the configured Cliptown zone posture;
- `k8s_cluster_read` — the exact Cliptown namespace in the
  `ORESoftware/k8s-cluster` deployment plane;
- `nats_service_read` — the Cliptown service and dependency subjects; and
- `organization_posture` — one bounded, concurrent view of all eight providers.

Three generated resources describe the service catalog, provider catalog, and
security policy. Three generated prompts guide deployment readiness, provider
triage, and dependency-impact analysis without granting arbitrary execution.
Every provider result has one of five honest states: `ready`, `degraded`,
`unauthorized`, `forbidden`, or `not_configured`. Missing credentials or scope
never becomes synthetic success.

`ORESoftware/mcp-rust-libs` is pinned to immutable revision
`046584b6e1138d4d9b141b48a66dca1d510ee52c`. Its organization-server crate pins
`ores-otel/ores-mcp-server-core-libs.rs` to reviewed revision
`e559a76f869c2c2d9bf939b510d358a3924abd81` for JSON logs on stderr plus OTLP
traces, metrics, and logs. MCP protocol frames own stdout.

## Run

Enter the pinned Nix shell and run with a SOPS-encrypted environment profile:

```sh
nix develop
just run dev
```

Only `env/enc/*.env.enc` ciphertext is committed. `just decrypt dev` writes a
mode-0600 ignored file to `env/dec/dev.env`; `just encrypt dev` updates the
ciphertext. Private age identities never belong in this repository.

`SHARED_AUTH_BASE_URL` is optional for this public policy-only baseline. A
local stdio process accepts no bearer token. The `cliptown-mcp-http` binary
serves the same catalog at `/mcp` over Streamable HTTP and fails before binding
unless `ORE_MCP_PUBLIC_RESOURCE`, `SHARED_AUTH_ISSUER`, and
`SHARED_AUTH_JWKS_URL` are exact. It validates ES256/JWKS locally, pins the
public resource as audience, requires the Cliptown project realm, AAL2,
`mcp:read` plus `cliptown:inspect`, and a `cliptown_viewer` or
`cliptown_operator` role. It never forwards a caller token upstream. Exact
client IDs, origins, and the loopback-default bind can be set with
`ORE_MCP_OAUTH_CLIENT_IDS`, `ORE_MCP_ALLOWED_ORIGINS`, and
`ORE_MCP_HTTP_BIND`.

Provider credentials and identifiers are process environment only, never MCP
tool arguments:

- GitHub: `ORE_MCP_GITHUB_TOKEN` (with `GITHUB_TOKEN`/`GH_TOKEN` compatibility);
- AWS: `ORE_MCP_AWS_ACCOUNT_ID`, `ORE_MCP_AWS_EKS_CLUSTERS`;
- GCP: `ORE_MCP_GCP_PROJECT_ID`, `ORE_MCP_GCP_PROJECT_NUMBER`,
  `ORE_MCP_GCP_ACCESS_TOKEN`;
- Supabase: `ORE_MCP_SUPABASE_URL`, `ORE_MCP_SUPABASE_SERVICE_ROLE_KEY`;
- Neon: `ORE_MCP_NEON_ORGANIZATION_ID`, `ORE_MCP_NEON_PROJECT_ID`,
  `ORE_MCP_NEON_API_KEY`;
- Cloudflare: `ORE_MCP_CLOUDFLARE_ZONE`, `ORE_MCP_CLOUDFLARE_ZONE_ID`,
  `ORE_MCP_CLOUDFLARE_API_TOKEN`;
- Kubernetes: `ORE_MCP_K8S_ENABLED=1`, optionally
  `ORE_MCP_K8S_NAMESPACE` (otherwise the exact `cliptown` namespace); and
- NATS: `ORE_MCP_NATS_URL`, scoped to
  `cliptown.mcp.service.read.v1` and `cliptown.mcp.dependencies.read.v1`.

These values belong in the SOPS+age lifecycle below. The server captures them
once at the process boundary, redacts secrets from output, disables ambient
HTTP proxies and redirects for credentialed requests, and bounds provider
responses.

Dependency intent lives in `.zpkg.toml`. Zed owns package identity, lock
provenance, and `.vendor/.zed` materialization; Cargo still builds the Rust
binary from exact Git and crate lock revisions.

## Validate

```sh
just check
```

The gate runs formatting, warning-denied Clippy, tests, dependency audit, and
the encrypted-environment policy. CI additionally smoke-tests the real MCP
stdio lifecycle and checks that stdout contains only JSON-RPC frames.
