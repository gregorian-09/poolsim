# Deployed-Pool Survey

Poolsim includes an opt-in survey payload generator for anonymous pool configuration statistics.

The tool is intentionally conservative:

- It requires explicit `--consent`.
- It does not submit data over the network.
- It rejects keys that look like hostnames, URLs, DSNs, credentials, tokens, query text, SQL, or service names.
- It exports only a small whitelist of non-identifying configuration fields.

## Allowed Fields

- `framework`
- `database`
- `pool_size`
- `min_pool_size`
- `max_pool_size`
- `replicas`
- `rps_band`
- `saturation`
- `uses_proxy`
- `environment_class`

## Example Input

```json
[
  {
    "framework": "sqlx",
    "database": "postgres",
    "pool_size": 8,
    "min_pool_size": 2,
    "max_pool_size": 20,
    "replicas": 3,
    "rps_band": "100-250",
    "saturation": "Ok",
    "environment_class": "production"
  }
]
```

## Generate A Payload

```bash
python3 tools/poolsim_survey.py \
  --input survey-input.json \
  --output survey-payload.json \
  --consent
```

The output includes `contains_application_data: false` and the sanitized entries.

## Test

```bash
python3 -m unittest tools/test_poolsim_survey.py
```
