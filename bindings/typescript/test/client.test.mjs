import assert from 'node:assert/strict';
import { chmodSync, mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { cwd } from 'node:process';
import test from 'node:test';
import { PoolsimClient, PoolsimError } from '../dist/index.js';

function fakePoolsim(payload, code = 0, stderr = '') {
  const tmpRoot = join(cwd(), '.tmp-tests');
  mkdirSync(tmpRoot, { recursive: true });
  const dir = mkdtempSync(join(tmpRoot, 'poolsim-ts-'));
  const script = join(dir, 'poolsim');
  writeFileSync(script, `#!/usr/bin/env node\nprocess.stderr.write(${JSON.stringify(stderr)});\nprocess.stdout.write(${JSON.stringify(JSON.stringify(payload))});\nprocess.exit(${code});\n`);
  chmodSync(script, 0o755);
  return script;
}

test.after(() => {
  rmSync(join(cwd(), '.tmp-tests'), { force: true, recursive: true });
});

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
