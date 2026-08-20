# Session-State Compatibility

Poolsim's session-state compatibility check answers a narrower question than pool sizing: can this application client safely use this external pooler mode without hidden session state breaking correctness or defeating multiplexing?

Use it before routing production traffic through PgBouncer transaction pooling, Supavisor transaction mode, RDS Proxy, Hyperdrive, or any pooler/proxy that may reuse backend database connections across client sessions.

## Why This Exists

Connection pool sizing can be mathematically correct and still fail in production if the application depends on session state.

Common failure modes:

- prepared statements are created on one backend connection and executed later on another backend connection
- `SET` state persists longer than the transaction that created it
- temporary tables disappear when the pooler gives the next transaction a different backend
- advisory locks are held at session scope but the client assumes request scope
- `LISTEN`/`NOTIFY` listener state depends on a stable backend session
- migrations or schema tools run through transaction poolers and lose session assumptions
- managed proxies pin sessions, reducing the backend connection reduction that pool sizing assumed

The check is conservative. Unknown client behavior, provider-managed pooling, and missing pooler config reduce confidence instead of being treated as safe.

## Source-Backed Rules

Poolsim uses these public source-backed rules for this feature:

- PgBouncer transaction pooling can support protocol-level prepared statements only when `max_prepared_statements` is configured to a non-zero value. SQL `PREPARE`/`DEALLOCATE` remains session-bound and unsafe in transaction/statement pooling.
- Supabase documents that Supavisor transaction mode does not support prepared statements and recommends turning prepared statements off for connection libraries using that mode.
- AWS RDS Proxy documents session pinning behavior; stateful session behavior can reduce multiplexing and make backend usage closer to client usage.
- PostgREST documents that transaction pooling requires `db-prepared-statements=false`, recommends disabling its channel listener, and states that statement pooling is not compatible.
- node-postgres documents named prepared statements through the query `name` field; unnamed parameterized queries are different from persistent named prepared statements.
- SQLAlchemy's asyncpg dialect documents prepared statement cache behavior and PgBouncer-specific mitigations.
- `sqlx` exposes PostgreSQL statement-cache configuration; transaction poolers that cannot preserve prepared statements need the cache disabled or explicit pooler support.

## CLI Usage

Basic check for `sqlx` through PgBouncer transaction pooling with explicit PgBouncer prepared-statement support:

```bash
poolsim --format json check session-state \
  --client sqlx \
  --pooler pg-bouncer \
  --mode transaction \
  --uses prepared-statements \
  --max-prepared-statements 100
```

Check Prisma through Supavisor transaction mode:

```bash
poolsim --format json check session-state \
  --client prisma \
  --pooler supavisor \
  --mode transaction
```

That command exits `2` because Prisma's prepared-statement behavior and Supavisor transaction mode require a configuration/topology change before production use.

Check node-postgres when named prepared statements are used:

```bash
poolsim --format json check session-state \
  --client node-postgres \
  --pooler pg-bouncer \
  --mode transaction \
  --uses named-prepared-statements
```

## REST Usage

```bash
curl -s \
  -X POST http://127.0.0.1:8080/v1/check/session-state \
  -H 'content-type: application/json' \
  --data @docs/fixtures/session-state-compatibility.json
```

## Library Usage

```rust
use poolsim_core::pooler::{
    analyze_session_state_compatibility,
    ClientLibraryKind,
    CompatibilityDecision,
    ExternalPoolerKind,
    MultiplexingMode,
    PoolerConfigSnapshot,
    SessionSemanticFeature,
    SessionStateCompatibilityInput,
};

let report = analyze_session_state_compatibility(
    &SessionStateCompatibilityInput::new(
        ClientLibraryKind::Sqlx,
        ExternalPoolerKind::PgBouncer,
        MultiplexingMode::Transaction,
    )
    .with_features(vec![SessionSemanticFeature::PreparedStatements])
    .with_pooler_config(PoolerConfigSnapshot::new().with_max_prepared_statements(100)),
);

assert_ne!(report.compatible, CompatibilityDecision::Incompatible);
assert_eq!(report.pooler_report.compatible, CompatibilityDecision::Compatible);
assert!(!report.client_guidance.is_empty());
```

## Input Model

Required fields:

- `client`: `generic-postgres`, `prisma`, `node-postgres`, `sqlx`, `sqlalchemy-asyncpg`, `postgrest`, `pg-jdbc`, or `unknown`
- `pooler`: `pg-bouncer`, `rds-proxy`, `supavisor`, `prisma-postgres-pooler`, `neon-pooler`, `cloudflare-hyperdrive`, or `unknown`
- `mode`: `none`, `session`, `transaction`, `statement`, `provider-managed`, or `unknown`

Optional fields:

- `features_used`: explicit session features used by the workload
- `workflow`: workflow category, such as `api-traffic`, `migration`, or `long-running-analytics`
- `pooler_config.max_prepared_statements`: PgBouncer evidence for prepared-statement tracking
- `pooler_config.resets_session_state`: evidence that the pooler resets session state safely

## Output Model

The report includes:

- `compatible`: final decision after base pooler checks and client-specific guidance
- `client`, `pooler`, and `mode`: analyzed inputs
- `effective_features`: explicit plus client-inferred features used by the base pooler check
- `pooler_report`: the existing generic `check_pooler_compatibility` result
- `client_guidance`: concrete framework/client remediation with source URLs
- `confidence`: final confidence after client assumptions and provider evidence

## Decision Semantics

`compatible = compatible` means no supplied evidence triggered a known unsafe rule.

`compatible = needs-review` means the combination may work, but at least one provider/client assumption needs verification. Examples include RDS Proxy pinning risk or an unknown client library.

`compatible = incompatible` means at least one known rule requires a change before production use. Examples include PostgREST statement pooling, Supavisor transaction mode with prepared statements, or PgBouncer transaction mode without prepared-statement support while prepared statements are used.

## Exit Codes

- `0`: compatible, or needs-review without `--warn-exit`
- `2`: incompatible
- `3`: needs-review when `--warn-exit` is enabled

## Practical Workflow

1. Run `poolsim classify endpoint` to identify the endpoint kind and provider.
2. Run `poolsim check pooler` when you already know the raw session features.
3. Run `poolsim check session-state` when you want client-specific remediation for Prisma, node-postgres, `sqlx`, SQLAlchemy asyncpg, PostgREST, PgJDBC, or generic PostgreSQL clients.
4. Run `poolsim graph ownership` after the session-state check to see whether the pooler client count and database backend count are distinct.
5. Re-run sizing, ownership, and session-state checks whenever the client library, pooler mode, prepared statement settings, migrations workflow, or provider changes.

## Limitations

Poolsim does not inspect application source code. It cannot prove whether a workload actually uses named prepared statements, temporary tables, session variables, or advisory locks unless you provide that evidence through `--uses` or the JSON payload.

Provider behavior can change. Poolsim reports source URLs in `client_guidance` so teams can verify the exact provider/client version they run in production.

## Sources

- PgBouncer FAQ: https://www.pgbouncer.org/faq.html
- PgBouncer feature matrix: https://www.pgbouncer.org/features.html
- AWS RDS Proxy pinning: https://docs.aws.amazon.com/AmazonRDS/latest/UserGuide/rds-proxy-pinning.html
- Supabase database connection pooling: https://supabase.com/docs/guides/database/connecting-to-postgres
- PostgREST connection pool docs: https://docs.postgrest.org/en/v12/references/connection_pool.html
- node-postgres queries: https://node-postgres.com/features/queries
- SQLAlchemy asyncpg dialect docs: https://docs.sqlalchemy.org/en/21/dialects/postgresql.html
- sqlx PgConnectOptions docs: https://docs.rs/sqlx/latest/sqlx/postgres/struct.PgConnectOptions.html
