# Endpoint Classification And External Poolers

## Purpose

Poolsim can classify database endpoints and check whether application features are safe with an external pooler mode.

Use this when a service connects through PgBouncer, RDS Proxy, Supabase/Supavisor, Neon pooled endpoints, Prisma Postgres pooled endpoints, Cloudflare Hyperdrive, or any provider-managed pooling layer.

## Non-Negotiable Rule

External poolers are not magic capacity.

Before trusting a pool-size recommendation, identify:

- who owns the client-side connection
- who owns the real database backend connection
- whether the endpoint is direct, session-pooled, transaction-pooled, statement-pooled, proxied, edge-managed, HTTP/Data API based, or unknown
- what can pin sessions or disable multiplexing
- which workflows require a direct endpoint
- what telemetry proves the pooler is reducing backend pressure

If poolsim cannot classify the endpoint or pooler mode with enough confidence, it lowers the report confidence instead of guessing.

## Endpoint Classification

Endpoint classification redacts secrets and returns a conservative endpoint kind.

Supported endpoint kinds:

- `direct-database`
- `session-pooler`
- `transaction-pooler`
- `statement-pooler`
- `database-proxy`
- `edge-pooler`
- `http-data-api`
- `unknown`

Supported provider hints:

- `supabase`
- `neon`
- `prisma-postgres`
- `aws-rds`
- `aws-rds-proxy`
- `cloudflare-hyperdrive`
- `pg-bouncer`
- `unknown`

### CLI Example

```bash
poolsim --format json classify endpoint \
  --endpoint 'postgres://user:secret@aws-0-us.pooler.supabase.com:6543/postgres?password=secret' \
  --workflow migration
```

Important behavior:

- credentials are redacted in output
- pooled endpoints used for migrations are reported as workflow mismatches
- clear workflow mismatches exit with code `2`
- `--warn-exit` returns code `3` for warning/review outcomes

Example response shape:

```json
{
  "endpoint_kind": "transaction-pooler",
  "provider": "supabase",
  "workflow_compatible": false,
  "redacted_endpoint": "postgres://<redacted>@aws-0-us.pooler.supabase.com:6543/postgres?password=<redacted>",
  "findings": [
    {
      "code": "ENDPOINT_WORKFLOW_MISMATCH",
      "risk": "High",
      "message": "the endpoint type is risky for the selected workflow",
      "remediation": "use a direct database endpoint for migrations, backup/restore, replication, database GUIs, admin work, and long-running analytics"
    }
  ],
  "confidence": "medium"
}
```

### Library Example

```rust
use poolsim_core::pooler::{
    classify_endpoint,
    DatabaseWorkflowKind,
    EndpointClassificationInput,
    EndpointConnectionKind,
};

let report = classify_endpoint(
    &EndpointClassificationInput::new(
        "postgres://user:secret@aws-0-us.pooler.supabase.com:6543/postgres",
    )
    .with_workflow(DatabaseWorkflowKind::Migration),
);

assert_eq!(report.endpoint_kind, EndpointConnectionKind::TransactionPooler);
assert_eq!(report.workflow_compatible, Some(false));
assert!(report.redacted_endpoint.contains("<redacted>"));
```

### REST Example

```bash
curl -s \
  -X POST http://127.0.0.1:8080/v1/classify/endpoint \
  -H 'content-type: application/json' \
  --data @docs/fixtures/endpoint-classification.json
```

Fixture:

```json
{
  "endpoint": "postgres://user:secret@aws-0-us.pooler.supabase.com:6543/postgres?password=secret",
  "workflow": "migration"
}
```

Schema:

- `docs/schemas/endpoint-classification.schema.json`

## Pooler Compatibility Check

Pooler compatibility checks whether a pooler mode is safe for the session features and workflow you plan to run.

Supported poolers:

- `pg-bouncer`
- `rds-proxy`
- `supavisor`
- `prisma-postgres-pooler`
- `neon-pooler`
- `cloudflare-hyperdrive`
- `unknown`

Supported multiplexing modes:

- `none`
- `session`
- `transaction`
- `statement`
- `provider-managed`
- `unknown`

Session features that commonly matter:

- `prepared-statements`
- `protocol-prepared-statements`
- `named-prepared-statements`
- `temporary-tables`
- `advisory-locks`
- `session-variables`
- `set-statement`
- `listen-notify-listener`
- `hold-cursors`
- `interactive-transaction`
- `long-running-query`
- `copy-protocol`
- `migrations`

### CLI Example

```bash
poolsim --format json check pooler \
  --pooler pg-bouncer \
  --mode transaction \
  --uses temporary-tables \
  --uses advisory-locks
```

Example response shape:

```json
{
  "compatible": "incompatible",
  "incompatible_features": ["temporary-tables", "advisory-locks"],
  "migration_direct_connection_required": false,
  "long_running_direct_connection_required": false,
  "findings": [
    {
      "code": "POOLER_FEATURE_INCOMPATIBLE",
      "risk": "High",
      "message": "TemporaryTables is unsafe or incompatible with Transaction pooling",
      "remediation": "use a direct/session endpoint, remove the session-dependent feature, or provide source-backed pooler configuration that proves compatibility"
    }
  ],
  "confidence": "high"
}
```

### Prepared Statements With PgBouncer

PgBouncer transaction pooling can support protocol-level prepared statements only when the pooler is configured for it. If you know `max_prepared_statements` is non-zero, pass it explicitly:

```bash
poolsim --format json check pooler \
  --pooler pg-bouncer \
  --mode transaction \
  --uses prepared-statements \
  --max-prepared-statements 100
```

Without this evidence, poolsim treats prepared statements under PgBouncer transaction pooling as unsafe.

### Client-Aware Session-State Check

Use `poolsim check session-state` when you want poolsim to apply known client-library behavior and return concrete remediation for the library your service uses.

```bash
poolsim --format json check session-state \
  --client sqlx \
  --pooler pg-bouncer \
  --mode transaction \
  --uses prepared-statements \
  --max-prepared-statements 100
```

The response includes the generic `pooler_report` plus `client_guidance`. The guidance includes source URLs and a `requires_change` flag for combinations that should not go to production without a config or topology change.

Supported clients:

- `generic-postgres`
- `prisma`
- `node-postgres`
- `sqlx`
- `sqlalchemy-asyncpg`
- `postgrest`
- `pg-jdbc`
- `unknown`

See [`session-state-compatibility.md`](session-state-compatibility.md) for source-backed rules, examples, outputs, and limitations.

### Library Example

```rust
use poolsim_core::pooler::{
    check_pooler_compatibility,
    CompatibilityDecision,
    ExternalPoolerKind,
    MultiplexingMode,
    PoolerCompatibilityInput,
    SessionSemanticFeature,
};

let report = check_pooler_compatibility(
    &PoolerCompatibilityInput::new(
        ExternalPoolerKind::PgBouncer,
        MultiplexingMode::Transaction,
    )
    .with_features(vec![
        SessionSemanticFeature::TemporaryTables,
        SessionSemanticFeature::AdvisoryLocks,
    ]),
);

assert_eq!(report.compatible, CompatibilityDecision::Incompatible);
assert_eq!(report.incompatible_features.len(), 2);
```

### REST Example

```bash
curl -s \
  -X POST http://127.0.0.1:8080/v1/check/pooler \
  -H 'content-type: application/json' \
  --data @docs/fixtures/pooler-compatibility.json
```

Fixture:

```json
{
  "pooler": "pg-bouncer",
  "mode": "transaction",
  "features_used": ["temporary-tables", "advisory-locks"]
}
```

Schema:

- `docs/schemas/pooler-compatibility.schema.json`

## Exit Codes

Endpoint classification:

- `0`: no clear workflow mismatch
- `2`: endpoint is clearly incompatible with the selected workflow
- `3`: warning/review outcome when `--warn-exit` is enabled

Pooler compatibility:

- `0`: compatible, or needs review without `--warn-exit`
- `2`: incompatible
- `3`: needs review when `--warn-exit` is enabled

## Limitations

Poolsim uses conservative known-provider patterns and supplied hints. It does not perform DNS lookups, connect to databases, or call provider APIs in this feature.

If the endpoint kind is unknown, provide a provider hint or use provider documentation to identify whether the endpoint is direct, pooled, proxied, edge-managed, or HTTP/Data API based.

If pooler mode is unknown, do not rely on pool-size math alone. First verify the active mode and whether application features can pin sessions or break multiplexing.
