import assert from 'node:assert/strict';
import { mkdtempSync, writeFileSync, chmodSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';
import { PoolsimClient, PoolsimError } from '../src/index.ts';

function fakePoolsim(payload, code = 0, stderr = '') {
  const dir = mkdtempSync(join(tmpdir(), 'poolsim-ts-'));
  const script = join(dir, 'poolsim');
  writeFileSync(script, `#!/usr/bin/env node\nprocess.stderr.write(${JSON.stringify(stderr)});\nprocess.stdout.write(${JSON.stringify(JSON.stringify(payload))});\nprocess.exit(${code});\n`);
  chmodSync(script, 0o755);
  return script;
}

test('simulate delegates to CLI JSON', () => {
  const client = new PoolsimClient(fakePoolsim({ optimal_pool_size: 8 }));
  assert.equal(client.simulate('config.json').optimal_pool_size, 8);
});

test('methods expose supported CLI workflows', () => {
  const client = new PoolsimClient(fakePoolsim({ status: 'ok' }));
  assert.equal(client.evaluate('c.json', 8).status, 'ok');
  assert.equal(client.compare('c.json').status, 'ok');
  assert.equal(client.budget('c.json').status, 'ok');
  assert.equal(client.telemetryRecommend('t.json').status, 'ok');
  assert.equal(client.doctor('t.json').status, 'ok');
  assert.equal(client.generateConfig('sqlx', 'c.json').status, 'ok');
});

test('errors include stderr', () => {
  const client = new PoolsimClient(fakePoolsim({}, 1, 'bad input'));
  assert.throws(() => client.simulate('config.json'), PoolsimError);
});
