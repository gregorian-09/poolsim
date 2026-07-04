"""Python bindings for the Poolsim CLI JSON contract.

The package intentionally delegates sizing to the `poolsim` executable instead of
reimplementing queueing formulas in Python.
"""

from .client import PoolsimClient, PoolsimError

__all__ = ["PoolsimClient", "PoolsimError"]
