# poolsim Python Bindings

Python bindings call the stable `poolsim` CLI JSON interface. They do not reimplement the Rust sizing model.

## Install

```bash
pip install poolsim
```

The Python package is a thin wrapper around the Rust CLI. Install the `poolsim`
executable separately and ensure it is available on `PATH`:

```bash
cargo install poolsim-cli
poolsim --version
```

Use `executable="/path/to/poolsim"` when the binary is not on `PATH`.

## Example

```python
from poolsim import PoolsimClient

client = PoolsimClient(executable="poolsim")
report = client.simulate("docs/fixtures/cli-config.json")
print(report["optimal_pool_size"])
```

Available methods include `simulate`, `evaluate`, `sweep`, `batch`, `compare`,
`budget`, `telemetry_recommend`, `doctor`, `generate_config`, and `gate`.
