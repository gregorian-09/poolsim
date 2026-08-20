# Pooler Evidence Import

Poolsim can import observed external-pooler counters and summarize whether client-side pressure is waiting on backend/server pool capacity.

Use this when you have PgBouncer `SHOW POOLS`, Supavisor pooler evidence, RDS Proxy metrics, or equivalent provider telemetry and need to keep client connections separate from real database backend connections.

If your evidence source is PgBouncer `SHOW POOLS`, you can also use the direct importer documented in [`pgbouncer-show-pools-import.md`](pgbouncer-show-pools-import.md) instead of manually normalizing JSON.

## Why This Exists

External poolers expose at least two different capacity surfaces:

- client-side connections accepted by the pooler
- backend/server connections opened from the pooler to the database

Those numbers are not interchangeable. A service may have many client connections but only a small backend footprint, or it may have few clients waiting because backend connections are saturated.

## Source-Backed Fields

PgBouncer `SHOW POOLS` documents:

- `cl_active`: active client connections, including clients linked to server connections or idle without waiting queries
- `cl_waiting`: client connections that sent queries but have not yet received a server connection
- `sv_active`: server connections currently linked to clients
- `sv_idle`: server connections idle in the pool

Supabase documents the same capacity distinction as client connections versus backend connections for Supavisor/PgBouncer. It also warns that using Supavisor and PgBouncer together can increase backend load against the same Postgres `max_connections` budget.

## CLI Usage

```bash
poolsim --format json import pooler-evidence \
  --config docs/fixtures/pooler-evidence.json
```

With `--warn-exit`, client waiting or incomplete evidence exits `3`; backend saturation exits `2`.

## Input Example

```json
{
  "pooler": "pg-bouncer",
  "mode": "transaction",
  "label": "checkout-api/postgres",
  "client_active": 42,
  "client_waiting": 0,
  "server_active": 8,
  "server_idle": 7,
  "pooler_client_limit": 500,
  "pooler_backend_limit": 30
}
```

## Output Semantics

The report includes:

- `status`: `healthy`, `client-waiting`, `backend-saturated`, or `needs-review`
- `observed_client_connections`: `client_active + client_waiting`, when both are known
- `observed_backend_connections`: `server_active + server_idle`, when both are known
- `backend_utilization`: backend connections divided by `pooler_backend_limit`, when known
- `client_utilization`: client connections divided by `pooler_client_limit`, when known
- `findings`: remediation-oriented findings for waiting clients, backend saturation, missing evidence, or unknown pooler/mode
- `confidence`: evidence confidence

## Library Usage

```rust
use poolsim_core::pooler::{
    summarize_pooler_evidence,
    ExternalPoolerKind,
    MultiplexingMode,
    PoolerEvidenceSnapshot,
    PoolerEvidenceStatus,
};

let report = summarize_pooler_evidence(
    &PoolerEvidenceSnapshot::new(
        ExternalPoolerKind::PgBouncer,
        MultiplexingMode::Transaction,
    )
    .with_client_active(42)
    .with_client_waiting(0)
    .with_server_active(8)
    .with_server_idle(7)
    .with_pooler_client_limit(500)
    .with_pooler_backend_limit(30),
);

assert_eq!(report.status, PoolerEvidenceStatus::Healthy);
assert_eq!(report.observed_backend_connections, Some(15));
```

## Practical Workflow

1. Capture pooler client/backend counters from your pooler or provider.
2. Run `poolsim import pooler-evidence` to summarize observed pressure.
3. Run `poolsim graph ownership` with the same backend cap to explain which layer owns app, pooler, and database capacity.
4. Run `poolsim check session-state` if clients are waiting or backend capacity is saturated, because session pinning and session-state behavior can reduce multiplexing.
5. Re-run sizing only after distinguishing app-side pressure from backend database pressure.

## Limitations

Poolsim does not connect to the pooler admin database in this slice. It accepts a normalized JSON snapshot so teams can feed data from PgBouncer admin output, exporters, cloud metrics, or internal telemetry without exposing credentials.

## Sources

- PgBouncer usage and `SHOW POOLS`: https://www.pgbouncer.org/usage.html
- Supabase connection pooling limits: https://supabase.com/docs/guides/database/connecting-to-postgres
- AWS RDS Proxy metrics and pinning context: https://docs.aws.amazon.com/AmazonRDS/latest/UserGuide/rds-proxy.monitoring.html
