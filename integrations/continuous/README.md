# Continuous Recommendation Mode

The continuous integration polls a telemetry import source, computes a recommendation diff, writes optional local state, and emits a stable `PoolRecommendationDiff` event.

It is useful when a team wants Poolsim to run periodically and report drift instead of only running during manual CLI use or CI gates.

## What It Does

- Calls the stable `poolsim --format json import prometheus` CLI path.
- Builds a `PoolRecommendationDiff` event from the latest recommendation.
- Compares the current recommendation with a previously saved state file.
- Marks the event as changed when the recommended pool size or change classification differs.
- Optionally posts the event to a webhook with bounded retries.
- Prints each event as JSON to stdout.

It does not modify application configuration, deployment manifests, database settings, or runtime pools.

## Current Source Support

The current worker supports one source:

- `prometheus-response-file`: reads captured Prometheus query responses from a JSON file and passes them to `poolsim import prometheus`.

The source name is explicit so additional continuous sources can be added later without changing the event contract.

## Runtime Requirements

- Python 3.9 or newer.
- The Rust `poolsim` executable from `poolsim-cli`.
- A captured Prometheus response file compatible with the Poolsim CLI import command.
- Optional webhook endpoint if you want push notifications.

Install the CLI:

```bash
cargo install poolsim-cli
poolsim --version
```

## Command

Run the worker directly:

```bash
python3 integrations/continuous/poolsim_continuous.py \
  --source prometheus-response-file \
  --response-file docs/fixtures/prometheus-responses.json \
  --service-name checkout-api \
  --window 5m \
  --current-pool-size 8 \
  --max-server-connections 100 \
  --min 2 \
  --max 20 \
  --connection-overhead-ms 2 \
  --interval-secs 60 \
  --state-file .poolsim/checkout-api-state.json
```

The process runs forever. Each interval prints one event to stdout.

## One Iteration Behavior

For each loop, the worker:

1. Loads previous recommendation JSON from `--state-file` if the file exists.
2. Builds a `poolsim import prometheus` command.
3. Parses the CLI JSON recommendation.
4. Builds a `PoolRecommendationDiff` event.
5. Posts the event to `--webhook-url` when configured.
6. Saves the latest recommendation JSON back to `--state-file` when configured.
7. Sleeps for `--interval-secs`.

## CLI Arguments

| Argument | Required | Meaning |
| --- | --- | --- |
| `--source` | No | Source type. Currently only `prometheus-response-file`. |
| `--response-file` | Yes | Path to captured Prometheus query responses. |
| `--service-name` | No | Service name passed to `poolsim import prometheus`. |
| `--window` | No | Observation window label, such as `5m` or `1h`. |
| `--current-pool-size` | Yes | Current configured pool size for the service. |
| `--max-server-connections` | Yes | Database connection cap visible to this service. |
| `--min` | Yes | Minimum candidate pool size. |
| `--max` | Yes | Maximum candidate pool size. |
| `--connection-overhead-ms` | No | Connection overhead assumption. |
| `--interval-secs` | No | Poll interval. Defaults to `60`. |
| `--state-file` | No | File used to compare against the previous recommendation. |
| `--webhook-url` | No | HTTP endpoint that receives each event as JSON. |
| `--poolsim-cli` | No | CLI binary path. Defaults to `poolsim`. |

## Generated Poolsim Command

The worker builds a command shaped like this:

```bash
poolsim --format json import prometheus \
  --response-file docs/fixtures/prometheus-responses.json \
  --current-pool-size 8 \
  --max-server-connections 100 \
  --min 2 \
  --max 20 \
  --service-name checkout-api \
  --window 5m \
  --connection-overhead-ms 2
```

Use the CLI directly first when debugging input files.

## Event Schema

Each emitted event has this shape:

```json
{
  "schema_version": "v1",
  "event_type": "PoolRecommendationDiff",
  "source": "prometheus-response-file",
  "service_name": "checkout-api",
  "window": "5m",
  "observed_at": null,
  "changed": true,
  "previous_recommended_pool_size": 8,
  "recommended_pool_size": 10,
  "previous_change": "Keep",
  "change": "Increase",
  "recommendation": {
    "service_name": "checkout-api",
    "window": "5m",
    "diff": {
      "current_pool_size": 8,
      "recommended_pool_size": 10,
      "change": "Increase"
    }
  }
}
```

Important fields:

- `schema_version`: event schema version. Currently `v1`.
- `event_type`: always `PoolRecommendationDiff` for this worker.
- `changed`: `true` when there is no previous state or the recommended size/change classification changed.
- `recommendation`: the complete Poolsim recommendation payload.

## Webhook Delivery

When `--webhook-url` is set, the worker posts each event as JSON:

```bash
python3 integrations/continuous/poolsim_continuous.py \
  --response-file docs/fixtures/prometheus-responses.json \
  --current-pool-size 8 \
  --max-server-connections 100 \
  --min 2 \
  --max 20 \
  --webhook-url https://hooks.example.test/poolsim
```

Webhook behavior:

- Method: `POST`.
- Header: `Content-Type: application/json`.
- Timeout: 10 seconds.
- Retries: 3 attempts.
- Backoff: capped exponential sleep between attempts.
- Failure: raises an error after all attempts fail.

For Slack, PagerDuty, or internal event buses, usually place a small adapter endpoint between Poolsim and the third-party API so you can map the event shape into the provider-specific payload.

## State File Strategy

Use a persistent `--state-file` if you want stable drift detection across process restarts:

```bash
--state-file /var/lib/poolsim/checkout-api-state.json
```

Without a state file, every first event after process start is marked as changed because there is no previous recommendation to compare.

## Operational Notes

- Run one worker per service if each service has different pool assumptions.
- Keep the state file on persistent storage if restart continuity matters.
- Keep webhook endpoints idempotent; retries can deliver duplicate events.
- Protect webhook URLs as secrets.
- Start with a conservative interval such as 60 seconds or 5 minutes; Poolsim recommendations should not be recalculated on every request.

## Run Tests

```bash
python3 -m unittest integrations/continuous/test_poolsim_continuous.py
```

The tests validate command construction, event diffing, state-file writes, and webhook invocation through injected fakes.

## Compatibility

This integration is additive. It shells out to the existing `poolsim --format json import prometheus` command and does not change Rust APIs, CLI output schemas, REST routes, WebSocket events, or config files.
