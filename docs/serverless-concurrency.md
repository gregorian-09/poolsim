# Serverless Concurrency Planning

## Purpose

Serverless pool sizing is different from sizing a single long-running process.
A serverless deployment can create many execution environments during a traffic
spike, and each environment can own its own application-side database pool.

Poolsim's serverless planner answers this question:

> If every concurrent execution environment opens its configured pool, can the
> deployment exceed the real database backend connection budget?

This is intentionally conservative. Warm reuse, provider pooling, and edge
connection reuse can reduce churn and connection setup overhead, but they do not
remove the need to account for worst-case simultaneous execution environments.

## When To Use It

Use `poolsim plan serverless` when a database-backed workload runs on:

- AWS Lambda
- Vercel Functions or Vercel Fluid compute
- Cloudflare Workers
- Netlify Functions
- Azure Functions
- Google Cloud Functions or Cloud Run functions
- any platform where instances are created and destroyed by demand

Use it before changing pool size, raising function concurrency, removing a
pooler, deploying a new function, or moving an existing service from containers
to serverless.

## Research-Backed Model

The planner uses these source-backed assumptions:

- AWS Lambda concurrency maps in-flight work to separate execution environments.
  Reserved concurrency can cap a function's maximum concurrent environments.
- AWS recommends initializing database connections outside the handler to reuse
  execution environments, and using keep-alive because idle connections can be
  purged over time.
- Vercel recommends globally initialized pools and low idle timeouts, but warns
  that traditional serverless can quickly exhaust database connections and that
  a pool max of one does not solve total connection growth.
- Cloudflare Hyperdrive separates Worker client connections from origin database
  connections and manages an origin-side pool in transaction mode.
- RDS Proxy separates client connections from database connections, can cap the
  database-side pool, and can lose multiplexing efficiency when sessions are
  pinned.
- Neon pooled endpoints can accept many client connections, but those clients
  still map to backend Postgres capacity as needed and cannot all execute
  interactively at once.
- Prisma's serverless guidance explains that each function/client instance owns
  its own pool, so concurrency and idle connection management matter.

## Inputs

### `platform`

Serverless or edge runtime family.

Accepted JSON values:

- `aws-lambda`
- `vercel-functions`
- `cloudflare-workers`
- `netlify-functions`
- `azure-functions`
- `google-cloud-functions`
- `unknown`

CLI values use the same kebab-case names.

### `max_concurrent_invocations`

Maximum concurrent invocations or execution environments expected for this
workload. This can come from provider concurrency limits, traffic estimates,
load tests, or production telemetry.

If this is missing and no explicit concurrency cap is supplied, Poolsim returns
`needs-review` because it cannot calculate the deployment-wide pool footprint.

### `reserved_concurrency`

Explicit concurrency cap for the function, if one exists. For AWS Lambda this
maps naturally to reserved concurrency. For other platforms, use the equivalent
hard cap if the provider supports one.

When both `max_concurrent_invocations` and `reserved_concurrency` are supplied,
Poolsim uses the smaller value as `effective_concurrency`.

### `app_pool_size_per_environment`

The maximum application-side pool size created inside each execution
environment. Examples:

- HikariCP `maximumPoolSize`
- Prisma connection limit for the client instance
- node-postgres `max`
- SQLAlchemy `pool_size + max_overflow` when overflow is enabled
- sqlx `max_connections`
- deadpool `max_size`

If the value is unknown, Poolsim returns `needs-review` because it cannot know
how many app-side connections scale out with the platform.

### `database_backend_limit`

Effective database backend connection budget for this workload after reserving
connections for:

- migrations
- admin shells and database GUIs
- replicas or other services
- monitoring
- failover headroom
- provider-reserved slots

If this is missing, Poolsim can still calculate the app-side footprint, but it
returns a warning because the footprint cannot be compared to a real limit.

### `uses_external_pooler`

Set this when traffic goes through PgBouncer, RDS Proxy, Supavisor, Neon pooler,
Prisma Postgres pooler, Cloudflare Hyperdrive, or another external pooling
layer.

Important: this does not automatically mark the deployment safe. External
poolers have their own limits, modes, and pinning behavior. Poolsim reports the
app-side footprint and lowers confidence until backend pooler evidence is
provided by future telemetry/importer features.

### `external_pooler`

Known external pooler family.

Accepted JSON values:

- `pg-bouncer`
- `rds-proxy`
- `supavisor`
- `prisma-postgres-pooler`
- `neon-pooler`
- `cloudflare-hyperdrive`
- `unknown`

If an external pooler is used but the kind is unknown, confidence is lowered.

### `warm_reuse_ratio`

Observed or estimated ratio from `0.0` through `1.0` describing how often work
uses warm execution environments.

Poolsim uses this only for churn risk. It does not reduce worst-case connection
capacity, because warm reuse does not prove that simultaneous execution
environments cannot exist.

## Output Fields

### `status`

Possible values:

- `pass`: provided direct-database evidence stays below the backend limit.
- `warning`: risk or uncertainty exists, but the direct backend limit is not
  proven exceeded.
- `critical`: direct app-side pool footprint exceeds the supplied backend limit.
- `needs-review`: required concurrency or pool-size evidence is missing.

### `effective_concurrency`

The concurrency value Poolsim used after applying a cap.

Formula:

```text
effective_concurrency = min(max_concurrent_invocations, reserved_concurrency)
```

If only one value is supplied, that value is used.

### `worst_case_app_pool_connections`

The maximum app-side connection footprint across execution environments.

Formula:

```text
worst_case_app_pool_connections = effective_concurrency * app_pool_size_per_environment
```

This is the key number for serverless planning.

### `direct_database_backend_upper_bound`

The backend connection upper bound when there is no external pooler.

When `uses_external_pooler` is true, this value is `null` because the backend
connection count depends on pooler mode, pinning, backend caps, and telemetry.
Poolsim deliberately refuses to pretend that a pooler magically eliminates the
backend limit.

### `connection_churn_risk`

Risk that cold starts, scale-out, suspended cleanup, or low reuse cause
connection setup churn.

Values:

- `Low`: warm reuse ratio is at least `0.70`.
- `Medium`: warm reuse ratio is at least `0.35`, or reuse is unknown.
- `High`: warm reuse ratio is below `0.35`.

### `findings`

Machine-readable findings with:

- `code`
- `risk`
- `message`
- `remediation`

These findings are intended for CLI output, CI logs, REST clients, and future
HTML reports.

## CLI Usage

### Safe Direct Lambda Example

```bash
poolsim --format json plan serverless \
  --platform aws-lambda \
  --max-concurrent-invocations 120 \
  --reserved-concurrency 80 \
  --pool-size 2 \
  --database-backend-limit 240 \
  --warm-reuse-ratio 0.72
```

Expected interpretation:

- effective concurrency is `80`
- worst-case app-side footprint is `160`
- backend limit is `240`
- status is `pass`

### Critical Vercel Example

```bash
poolsim --format json plan serverless \
  --platform vercel-functions \
  --max-concurrent-invocations 200 \
  --pool-size-per-environment 4 \
  --database-backend-limit 300
```

Expected interpretation:

- worst-case footprint is `800`
- direct backend limit is `300`
- status is `critical`
- process exits with code `2`

### Cloudflare Hyperdrive Example

```bash
poolsim --format json plan serverless \
  --platform cloudflare-workers \
  --max-concurrent-invocations 500 \
  --pool-size 1 \
  --external-pooler cloudflare-hyperdrive \
  --database-backend-limit 100 \
  --warm-reuse-ratio 0.20
```

Expected interpretation:

- app-side footprint is still `500`
- direct backend upper bound is `null` because Hyperdrive owns the backend pool
- status is `warning`, not `pass`, until backend pooler telemetry proves safety
- churn risk is `High`

## Exit Codes

Default behavior:

- `0`: `pass`, `warning`, or `needs-review`
- `2`: `critical`
- `1`: invalid input or unexpected execution error

With `--warn-exit`:

- `0`: `pass`
- `2`: `critical`
- `3`: `warning` or `needs-review`
- `1`: invalid input or unexpected execution error

Use `--warn-exit` in CI when missing evidence should block a change.

## REST Usage

Route:

```text
POST /v1/plan/serverless
```

Request:

```bash
curl -s http://127.0.0.1:8080/v1/plan/serverless \
  -H 'content-type: application/json' \
  --data @docs/fixtures/serverless-concurrency.json
```

Response excerpt:

```json
{
  "status": "pass",
  "platform": "aws-lambda",
  "effective_concurrency": 80,
  "worst_case_app_pool_connections": 160,
  "direct_database_backend_upper_bound": 160,
  "database_backend_limit": 240,
  "uses_external_pooler": false,
  "external_pooler": null,
  "connection_churn_risk": "Low",
  "confidence": "high"
}
```

## Library Usage

```rust
use poolsim_core::serverless::{
    plan_serverless_concurrency, ServerlessConcurrencyInput, ServerlessConcurrencyStatus,
    ServerlessPlatformKind,
};

let input = ServerlessConcurrencyInput::new(ServerlessPlatformKind::AwsLambda)
    .with_max_concurrent_invocations(120)
    .with_reserved_concurrency(80)
    .with_app_pool_size_per_environment(2)
    .with_database_backend_limit(240)
    .with_warm_reuse_ratio(0.72);

let report = plan_serverless_concurrency(&input)?;
assert_eq!(report.status, ServerlessConcurrencyStatus::Pass);
assert_eq!(report.worst_case_app_pool_connections, Some(160));
# Ok::<(), poolsim_core::error::PoolsimError>(())
```

External pooler example:

```rust
use poolsim_core::{
    pooler::ExternalPoolerKind,
    serverless::{plan_serverless_concurrency, ServerlessConcurrencyInput, ServerlessPlatformKind},
};

let input = ServerlessConcurrencyInput::new(ServerlessPlatformKind::CloudflareWorkers)
    .with_max_concurrent_invocations(500)
    .with_app_pool_size_per_environment(1)
    .with_external_pooler(ExternalPoolerKind::CloudflareHyperdrive)
    .with_database_backend_limit(100)
    .with_warm_reuse_ratio(0.20);

let report = plan_serverless_concurrency(&input)?;
assert_eq!(report.direct_database_backend_upper_bound, None);
# Ok::<(), poolsim_core::error::PoolsimError>(())
```

## JSON Schema

Schema:

- `docs/schemas/serverless-concurrency.schema.json`

Fixture:

- `docs/fixtures/serverless-concurrency.json`

Validation example:

```bash
npx ajv-cli validate \
  -s docs/schemas/serverless-concurrency.schema.json \
  -d docs/fixtures/serverless-concurrency.json
```

## Operational Guidance

### Do Not Hide Behind Warm Reuse

Warm reuse improves performance, but serverless scale-out can still create many
execution environments. Use warm reuse to reason about churn, not to shrink the
worst-case capacity calculation.

### Cap Concurrency When The Database Budget Is Fixed

If the database budget cannot grow, reduce one of these:

- platform concurrency
- per-environment pool size
- number of functions sharing the database
- traffic routed to direct database endpoints

### Treat External Poolers As Separate Capacity Layers

An external pooler changes who owns backend connections. It does not eliminate
backend limits. Validate:

- pooler mode
- backend connection cap
- pinning behavior
- long transaction behavior
- prepared statement/session-state behavior
- live backend connection telemetry

### Leave Backend Headroom

Do not allocate 100% of `max_connections` to one function. Reserve capacity for
migrations, admin work, failover, monitoring, provider-reserved slots, and other
services.

## Limitations

The current planner does not yet import live provider telemetry for RDS Proxy,
PgBouncer, Supavisor, Neon poolers, Prisma poolers, or Hyperdrive. It therefore
uses conservative `warning` or `needs-review` statuses when a pooler is present
but backend evidence is missing.

The planner also does not model synchronized cron jobs, queue fan-out, retry
storms, credential refresh storms, or transaction-level service-time mixtures.
Those belong to later topology-aware diagnostics and telemetry importers.

## Source Notes

The planner is based on current provider documentation for Lambda concurrency,
Lambda execution-environment reuse, RDS Proxy client/database connections,
Vercel Functions connection pooling, Cloudflare Hyperdrive connection lifecycle,
Neon pooled endpoints, and Prisma serverless connection guidance.
