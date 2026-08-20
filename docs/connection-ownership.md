# Connection Ownership Graph

## Purpose

A pool size is only useful when the team knows which layer owns the connections.
A backend engineer reviewing a pool change needs to distinguish:

- runtime units that create pools
- application-side driver pools
- external pooler client connections
- external pooler backend connections
- real database backend sessions
- provider-managed HTTP or edge database APIs

Poolsim's connection ownership graph answers this question:

> Which layer owns each connection count, and which layer consumes real database backend capacity?

The graph is deliberately conservative. If a pooler backend limit, database
limit, runtime unit count, or pool size is missing, Poolsim lowers confidence and
returns review findings instead of pretending the topology is safe.

## Research-Backed Model

The model follows current production observability and provider behavior:

- OpenTelemetry database client metrics expose app-side pool states such as used
  and idle connections, plus pending requests, wait time, create time, and
  timeouts.
- PgBouncer `SHOW POOLS` separates client counters such as `cl_active` and
  `cl_waiting` from server counters such as `sv_active` and `sv_idle`.
- RDS Proxy separates client connections from database connections and documents
  session pinning, where a client session can hold one backend database
  connection until the session ends.
- Cloudflare Hyperdrive separates Worker-to-Hyperdrive edge connections from
  Hyperdrive-to-origin database pool connections.

## Inputs

### `service_name`

Optional label for the service, function, job, or workload.

### `runtime_units`

Number of runtime units that can each own an application pool. Depending on the
platform, this can mean:

- Kubernetes replicas
- VM processes
- background workers
- serverless execution environments
- queue consumers
- Durable Objects or equivalent long-lived runtime owners

CLI aliases:

- `--runtime-units`
- `--instances`
- `--replicas`
- `--execution-environments`

### `app_pool_size_per_runtime_unit`

Maximum application-side pool size owned by each runtime unit.

CLI aliases:

- `--app-pool-size-per-runtime-unit`
- `--pool-size`
- `--pool-size-per-runtime-unit`

### `endpoint_kind`

Endpoint class for the workload.

Values:

- `direct-database`
- `session-pooler`
- `transaction-pooler`
- `statement-pooler`
- `database-proxy`
- `edge-pooler`
- `http-data-api`
- `unknown`

Use `poolsim classify endpoint` first when the endpoint kind is not known.

### `external_pooler`

Known external pooler or proxy family.

Values:

- `pg-bouncer`
- `rds-proxy`
- `supavisor`
- `prisma-postgres-pooler`
- `neon-pooler`
- `cloudflare-hyperdrive`
- `unknown`

### `pooler_client_limit`

Maximum client connections accepted by the external pooler or proxy.

This is not the same as backend database connections. A pooler can accept many
client connections while maintaining a smaller backend pool.

### `pooler_backend_limit`

Maximum backend database connections owned by the pooler or proxy.

This is the key field for proving that a pooler actually caps database backend
usage. If this is missing, Poolsim returns `needs-review` when an external pooler
is present.

### `database_backend_limit`

Effective database backend connection budget after subtracting reserved slots,
admin access, monitoring, migrations, and other services.

### `session_pinning_risk`

Optional risk that pooler multiplexing is reduced by session pinning.

Values use the existing `RiskLevel` JSON spelling:

- `Low`
- `Medium`
- `High`
- `Critical`

High or critical pinning risk with an external pooler produces a review finding.

## Output Fields

### `status`

Possible values:

- `complete`: enough evidence exists and no unsafe backend path was found.
- `needs-review`: required topology, limit, or pinning evidence is missing or close to a limit.
- `unsafe`: supplied evidence shows a backend capacity path exceeds the database limit.

### `nodes`

Ordered graph nodes from application runtime to database backend.

Important node fields:

- `id`: stable node identifier.
- `layer_kind`: layer category.
- `owner`: component that owns the layer.
- `max_connections`: maximum connection count for that layer, when known.
- `consumes_database_connections`: whether that layer directly consumes backend database capacity.

### `edges`

Ordered graph edges from application runtime to database backend.

Important edge fields:

- `from`
- `to`
- `relationship`
- `worst_case_connections`
- `consumes_backend_capacity`

### `app_connection_upper_bound`

Worst-case app-side connection count:

```text
runtime_units * app_pool_size_per_runtime_unit
```

### `database_backend_upper_bound`

Known upper bound that can consume real backend database capacity.

For direct database endpoints, this is the app-side upper bound.
For external poolers, this is `pooler_backend_limit` when supplied.
If a pooler is present and the backend limit is unknown, this is `null`.

### `bottleneck_layer`

Layer that currently needs attention, when known. Examples:

- `application-pool`
- `pooler-backend`
- `database-backend`

### `findings`

Machine-readable findings with `code`, `risk`, `message`, and `remediation`.

## CLI Usage

### Direct Database Example

```bash
poolsim --format json graph ownership \
  --service-name checkout-api \
  --runtime-units 12 \
  --pool-size 10 \
  --database-backend-limit 100
```

Expected interpretation:

- app-side upper bound is `120`
- direct database backend upper bound is `120`
- database backend limit is `100`
- status is `unsafe`
- process exits with code `2`

### RDS Proxy Example

```bash
poolsim --format json graph ownership \
  --service-name checkout-api \
  --runtime-units 12 \
  --pool-size 10 \
  --endpoint-kind database-proxy \
  --external-pooler rds-proxy \
  --pooler-client-limit 1000 \
  --pooler-backend-limit 90 \
  --database-backend-limit 120 \
  --session-pinning-risk medium
```

Expected interpretation:

- app-side upper bound is `120`
- pooler client side can accept up to `1000` client connections
- pooler backend side caps database connections at `90`
- database backend limit is `120`
- status is `complete`

### PgBouncer Unknown Backend Example

```bash
poolsim --warn-exit --format json graph ownership \
  --runtime-units 100 \
  --pool-size 2 \
  --endpoint-kind transaction-pooler \
  --external-pooler pg-bouncer \
  --database-backend-limit 120 \
  --session-pinning-risk high
```

Expected interpretation:

- app-side upper bound is `200`
- pooler backend cap is missing
- high pinning risk can reduce multiplexing
- status is `needs-review`
- `--warn-exit` makes the process exit with code `3`

## REST Usage

Route:

```text
POST /v1/graph/ownership
```

Request:

```bash
curl -s http://127.0.0.1:8080/v1/graph/ownership \
  -H 'content-type: application/json' \
  --data @docs/fixtures/connection-ownership.json
```

Response excerpt:

```json
{
  "status": "complete",
  "service_name": "checkout-api",
  "app_connection_upper_bound": 120,
  "database_backend_upper_bound": 90,
  "database_backend_limit": 120,
  "bottleneck_layer": null,
  "confidence": "medium"
}
```

## Library Usage

```rust
use poolsim_core::{
    ownership::{build_connection_ownership_graph, ConnectionOwnershipInput, ConnectionOwnershipStatus},
    pooler::{EndpointConnectionKind, ExternalPoolerKind},
    types::RiskLevel,
};

let input = ConnectionOwnershipInput::new()
    .with_service_name("checkout-api")
    .with_runtime_units(12)
    .with_app_pool_size_per_runtime_unit(10)
    .with_endpoint_kind(EndpointConnectionKind::DatabaseProxy)
    .with_external_pooler(ExternalPoolerKind::RdsProxy)
    .with_pooler_client_limit(1000)
    .with_pooler_backend_limit(90)
    .with_database_backend_limit(120)
    .with_session_pinning_risk(RiskLevel::Medium);

let report = build_connection_ownership_graph(&input)?;
assert_eq!(report.status, ConnectionOwnershipStatus::Complete);
assert_eq!(report.app_connection_upper_bound, Some(120));
assert_eq!(report.database_backend_upper_bound, Some(90));
# Ok::<(), poolsim_core::error::PoolsimError>(())
```

## JSON Schema

Schema:

- `docs/schemas/connection-ownership.schema.json`

Fixture:

- `docs/fixtures/connection-ownership.json`

Validation example:

```bash
npx ajv-cli validate \
  -s docs/schemas/connection-ownership.schema.json \
  -d docs/fixtures/connection-ownership.json
```

## Operational Guidance

### Do Not Collapse Pooler Client And Backend Counts

Client connections accepted by a pooler are not automatically database backend
connections. Treat PgBouncer `cl_*` counters, RDS Proxy client connections, and
Hyperdrive edge connections as separate from backend database connections.

### Pinning Can Defeat Multiplexing

If sessions are pinned, backend database usage can approach client-side usage.
Use pooler telemetry and query/session-state review before increasing app-side
pool sizes.

### Keep Backend Headroom

A graph that uses most of the database backend limit should still be treated as
review-worthy. Reserve space for migrations, failover, admin sessions,
monitoring, and other services.

## Limitations

The current graph uses caller-supplied topology evidence. It does not yet import
PgBouncer `SHOW POOLS`, RDS Proxy CloudWatch metrics, Hyperdrive telemetry, or
provider dashboard data directly. Those importers are later slices.
