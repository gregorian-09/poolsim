# PgBouncer SHOW POOLS Import

Poolsim can read captured PgBouncer `SHOW POOLS` output and turn it into the same normalized pooler evidence report used by `poolsim import pooler-evidence`.

Use this when you already operate PgBouncer and want to answer a practical production question before changing application pool sizes:

- Are clients waiting inside PgBouncer?
- Are PgBouncer server connections close to the backend pool limit?
- Is the pressure caused by client-side fan-in, backend saturation, or incomplete evidence?
- Would increasing the application-side pool make the real bottleneck worse?

## Source Model

PgBouncer documents `SHOW POOLS` as one row per `(database, user)` pool. Poolsim imports the counters that describe client pressure and backend/server usage:

- `cl_active`: client connections active or idle without queued work.
- `cl_waiting`: clients that sent queries and are waiting for a server connection.
- `sv_active`: server connections linked to a client.
- `sv_idle`: server connections immediately usable for client queries.
- `pool_mode`: `session`, `transaction`, or `statement`, when present.

Poolsim aggregates all imported rows into one `PoolerEvidenceReport` because the report describes total pressure on the captured PgBouncer instance or capture scope.

## Recommended Capture

Prefer CSV for CI, automation, and support bundles:

```bash
psql -p 6432 -d pgbouncer --csv -c "SHOW POOLS;" > pgbouncer-show-pools.csv
```

Then import it:

```bash
poolsim --format json import pgbouncer-pools \
  --file pgbouncer-show-pools.csv \
  --label checkout-prod-pgbouncer \
  --pooler-client-limit 500 \
  --pooler-backend-limit 30
```

Use `--pooler-backend-limit` for the effective PgBouncer server pool cap you want to evaluate. Without it, poolsim can still report clients and backend counts, but it cannot determine backend utilization or backend saturation.

## Terminal Copy-Paste Capture

Poolsim also accepts default aligned `psql` output copied from a terminal:

```text
 database | user | cl_active | cl_waiting | sv_active | sv_idle | pool_mode
----------+------+-----------+------------+-----------+---------+-----------
 checkout | web  |        40 |          0 |         8 |       7 | transaction
 checkout | jobs |         2 |          0 |         0 |       0 | transaction
(2 rows)
```

This is useful during incident review and local diagnostics. For repeatable automation, use CSV because it avoids terminal wrapping and locale formatting surprises.

## CLI Flags

```bash
poolsim import pgbouncer-pools --file <path> [flags]
```

Flags:

- `--file <path>`: captured PgBouncer `SHOW POOLS` output in CSV or aligned `psql` format.
- `--input <path>`: alias for `--file`.
- `--label <text>`: optional service, environment, PgBouncer instance, or capture label.
- `--mode <none|session|transaction|statement|provider-managed|unknown>`: optional mode override when the capture omits `pool_mode`.
- `--pooler-client-limit <n>`: optional PgBouncer client connection cap.
- `--pooler-backend-limit <n>`: optional PgBouncer backend/server connection cap.

## Output

The command returns the existing `PoolerEvidenceReport` shape:

- `status`: `healthy`, `client-waiting`, `backend-saturated`, or `needs-review`.
- `pooler`: always `pg-bouncer` for this importer.
- `mode`: imported from `pool_mode`, overridden by `--mode`, or `unknown` when unavailable.
- `observed_client_connections`: sum of `cl_active + cl_waiting` across rows.
- `observed_backend_connections`: sum of `sv_active + sv_idle` across rows.
- `client_waiting`: sum of `cl_waiting` across rows.
- `backend_utilization`: observed backend connections divided by `--pooler-backend-limit`, when supplied.
- `client_utilization`: observed client connections divided by `--pooler-client-limit`, when supplied.
- `findings`: actionable explanations and remediation guidance.
- `confidence`: confidence after accounting for missing mode or limit evidence.

## Library Usage

```rust
use poolsim_core::pooler::{
    parse_pgbouncer_show_pools,
    summarize_pgbouncer_show_pools,
    PgbouncerShowPoolsSnapshot,
    PoolerEvidenceStatus,
};

let capture = "database,user,cl_active,cl_waiting,sv_active,sv_idle,pool_mode\ncheckout,web,42,0,8,7,transaction\n";
let rows = parse_pgbouncer_show_pools(capture)?;
let report = summarize_pgbouncer_show_pools(
    &PgbouncerShowPoolsSnapshot::new(rows)
        .with_label("checkout-prod")
        .with_pooler_client_limit(500)
        .with_pooler_backend_limit(30),
)?;

assert_eq!(report.status, PoolerEvidenceStatus::Healthy);
assert_eq!(report.observed_client_connections, Some(42));
# Ok::<(), poolsim_core::error::PoolsimError>(())
```

## Interpretation Rules

- `client-waiting` means PgBouncer has queued client work. First inspect slow queries, long transactions, pool mode, and backend limits; do not blindly raise application pool sizes.
- `backend-saturated` means observed PgBouncer backend/server connections are at or above the supplied backend limit. Raising application-side pools can increase pressure without increasing real database capacity.
- `needs-review` usually means the capture omitted a mode or backend limit. Provide `--mode` and `--pooler-backend-limit` for a stronger decision.
- `healthy` only means the captured moment did not show waiting clients or backend saturation. Validate peak and incident windows separately.

## Current Scope

This importer parses captured output. It does not open a live PgBouncer admin connection. That keeps the CLI dependency surface small, avoids credential handling, and lets operators review or redact captures before importing them.

## References

- PgBouncer usage documentation: https://www.pgbouncer.org/usage.html
- PostgreSQL `psql --csv` documentation: https://www.postgresql.org/docs/current/app-psql.html
