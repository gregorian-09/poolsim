# JSON Schema

Poolsim ships JSON Schema files for the documented file formats used by the CLI and web examples. These schemas are intended for editor autocomplete, review automation, and CI validation before running capacity checks.

## Schema Files

Available schemas:

- [`schemas/poolsim-config.schema.json`](schemas/poolsim-config.schema.json): single `simulate`, `evaluate`, and `sweep` config object with `workload`, `pool`, and optional `options`.
- [`schemas/batch.schema.json`](schemas/batch.schema.json): batch input accepted as either an array of simulation configs or an object with `requests`.
- [`schemas/scenarios.schema.json`](schemas/scenarios.schema.json): scenario comparison config for `poolsim compare`.
- [`schemas/budget.schema.json`](schemas/budget.schema.json): database connection budget planner config for `poolsim budget`.
- [`schemas/telemetry.schema.json`](schemas/telemetry.schema.json): telemetry recommendation config for `poolsim import telemetry`, `poolsim gate telemetry`, `poolsim guard telemetry`, and `poolsim doctor telemetry`.
- [`schemas/gate-policy.schema.json`](schemas/gate-policy.schema.json): capacity gate policy for `poolsim gate` and `poolsim guard`.

The schemas are forward-compatible by design: they require the fields Poolsim needs today, but allow additional properties so future optional fields do not immediately break older editor setups.

## Editor Usage

For Visual Studio Code, add a JSON schema association in `.vscode/settings.json`:

```json
{
  "json.schemas": [
    {
      "fileMatch": ["poolsim.json", "poolsim.*.json"],
      "url": "./docs/schemas/poolsim-config.schema.json"
    },
    {
      "fileMatch": ["poolsim-gate-policy.json"],
      "url": "./docs/schemas/gate-policy.schema.json"
    }
  ]
}
```

For repository-local configs, keep the schemas near the checked-in policy files and point the editor to the relative path.

## CI Validation

Use any JSON Schema validator that supports draft 2020-12. Example with `ajv-cli`:

```bash
npx ajv-cli validate \
  -s docs/schemas/poolsim-config.schema.json \
  -d docs/fixtures/cli-config.json
```

Telemetry config example:

```bash
npx ajv-cli validate \
  -s docs/schemas/telemetry.schema.json \
  -d docs/fixtures/telemetry.json
```

Gate policy files are commonly TOML in this repository. Convert TOML to JSON before validating against `gate-policy.schema.json`, or keep a JSON policy copy for CI validation.

## Compatibility Notes

The schemas describe the existing public config surface. They do not replace runtime validation in `poolsim-core`, and they intentionally do not change CLI behavior. If a config passes schema validation but violates a domain rule, Poolsim still returns the normal typed validation error at runtime.
