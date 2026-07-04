# poolsim Python Bindings

Python bindings call the stable `poolsim` CLI JSON interface. They do not reimplement the Rust sizing model.

```python
from poolsim import PoolsimClient

client = PoolsimClient(executable="poolsim")
report = client.simulate("docs/fixtures/cli-config.json")
print(report["optimal_pool_size"])
```

Install from a future package with `pip install poolsim`, then ensure the Rust `poolsim` executable is available on `PATH`.
