# poolsim TypeScript Bindings

TypeScript bindings call the stable `poolsim` CLI JSON interface.

```ts
import { PoolsimClient } from 'poolsim';

const client = new PoolsimClient('poolsim');
const report = client.simulate('docs/fixtures/cli-config.json');
console.log(report.optimal_pool_size);
```

Install from a future package with `npm install poolsim`, then ensure the Rust `poolsim` executable is available on `PATH`.
