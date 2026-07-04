# Sizing Benchmark Suite

Poolsim's benchmark suite is designed to compare mathematical recommendations against real pool behavior under controlled load.

The first checked-in layer is the reproducible result contract and summarizer. Database-backed HikariCP and sqlx runners can write rows in this contract, then `summarize_results.py` computes prediction error for published reports.

## Result Contract

Each benchmark row is JSON:

```json
{
  "target": "postgres-local",
  "framework": "sqlx",
  "recommended_pool_size": 8,
  "actual_pool_size": 8,
  "poolsim_predicted_p99_queue_wait_ms": 24.5,
  "observed_p99_queue_wait_ms": 27.0
}
```

Required fields:

- `target`: database/load target name.
- `framework`: pool implementation, such as `sqlx` or `hikaricp`.
- `recommended_pool_size`: pool size recommended by Poolsim.
- `actual_pool_size`: pool size used during the real load test.
- `poolsim_predicted_p99_queue_wait_ms`: predicted p99 queue wait.
- `observed_p99_queue_wait_ms`: observed p99 queue wait during the benchmark.

## Summarize Results

```bash
python3 benchmarks/summarize_results.py benchmarks/fixtures/sample-results.json
```

The output includes one row per benchmark and `mean_p99_queue_wait_percent_error` across the suite.

## Intended Real Runners

The result contract is intentionally framework-neutral. Real runners should:

1. Start a disposable database such as PostgreSQL.
2. Apply a fixed workload with known request rate and latency distribution.
3. Ask Poolsim for a recommendation.
4. Run the target pool at the recommended size.
5. Record observed p95/p99 latency and p99 queue wait.
6. Publish the JSON row without credentials, query text, or private hostnames.

HikariCP and sqlx are the first target frameworks because they represent JVM and Rust backend stacks.

## Test

```bash
python3 -m unittest benchmarks/tests/test_summarize_results.py
```
