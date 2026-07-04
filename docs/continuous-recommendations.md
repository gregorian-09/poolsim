# Continuous Recommendations

Poolsim includes an opt-in continuous recommendation worker for teams that want repeated recommendation diffs from telemetry exports.

The worker is intentionally a wrapper around existing CLI import commands. It does not change the Rust library API, REST routes, WebSocket events, or CLI output schemas.

## Current Source

The first supported source is a Prometheus response-file poller:

```bash
python3 integrations/continuous/poolsim_continuous.py \
  --response-file docs/fixtures/prometheus-responses.json \
  --service-name checkout-api \
  --window 5m \
  --current-pool-size 8 \
  --max-server-connections 100 \
  --connection-overhead-ms 2 \
  --min 2 \
  --max 20 \
  --state-file .poolsim/continuous-state.json \
  --webhook-url https://hooks.example.test/poolsim \
  --interval-secs 60
```

For live Prometheus deployments, export or snapshot Prometheus instant-query responses into the response-file shape already documented for `poolsim import prometheus`, then let the worker consume that file on a schedule. Direct Prometheus and InfluxDB polling can be added later without changing the event shape.

## Event Shape

Every poll emits a `PoolRecommendationDiff` event to stdout and optionally to the webhook:

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
  "recommendation": {}
}
```

The nested `recommendation` value is the existing `TelemetryRecommendation` JSON output from the CLI.

## Webhook Delivery

Webhook delivery is optional. When configured, the worker sends `application/json` with bounded retry attempts and exponential backoff. A failed webhook raises an error so process supervisors, Kubernetes, or CI can restart or alert on the worker.

## Compatibility

This worker is additive. Existing `poolsim import prometheus`, `poolsim gate`, `poolsim guard`, REST routes, and serialized output fields are unchanged.

## Test

```bash
python3 -m unittest integrations/continuous/test_poolsim_continuous.py
```
