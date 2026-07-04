import { spawnSync } from 'node:child_process';

export type JsonValue = null | boolean | number | string | JsonValue[] | { [key: string]: JsonValue };

export class PoolsimError extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'PoolsimError';
  }
}

export class PoolsimClient {
  private readonly executable: string;

  constructor(executable = 'poolsim') {
    this.executable = executable;
  }

  simulate(config: string): Record<string, JsonValue> {
    return this.runJson(['simulate', '--config', config]) as Record<string, JsonValue>;
  }

  evaluate(config: string, poolSize: number): Record<string, JsonValue> {
    return this.runJson(['evaluate', '--config', config, '--pool-size', String(poolSize)]) as Record<string, JsonValue>;
  }

  sweep(config: string): JsonValue[] {
    return this.runJson(['sweep', '--config', config]) as JsonValue[];
  }

  batch(config: string): JsonValue[] {
    return this.runJson(['batch', '--config', config]) as JsonValue[];
  }

  compare(config: string): Record<string, JsonValue> {
    return this.runJson(['compare', '--config', config]) as Record<string, JsonValue>;
  }

  budget(config: string): Record<string, JsonValue> {
    return this.runJson(['budget', '--config', config]) as Record<string, JsonValue>;
  }

  telemetryRecommend(config: string): Record<string, JsonValue> {
    return this.runJson(['import', 'telemetry', '--config', config]) as Record<string, JsonValue>;
  }

  doctor(config: string): Record<string, JsonValue> {
    return this.runJson(['doctor', 'telemetry', '--config', config]) as Record<string, JsonValue>;
  }

  generateConfig(framework: string, config: string): Record<string, JsonValue> {
    return this.runJson(['generate-config', '--framework', framework, 'simulate', '--config', config]) as Record<string, JsonValue>;
  }

  gate(policy: string, telemetryConfig: string): Record<string, JsonValue> {
    return this.runJson(['gate', '--policy', policy, 'telemetry', '--config', telemetryConfig], [0, 2]) as Record<string, JsonValue>;
  }

  private runJson(args: string[], allowedExitCodes = [0]): JsonValue {
    const result = spawnSync(this.executable, ['--format', 'json', ...args], { encoding: 'utf8' });
    if (result.error) {
      throw new PoolsimError(result.error.message);
    }
    if (!allowedExitCodes.includes(result.status ?? 1)) {
      throw new PoolsimError(result.stderr.trim() || `poolsim exited with ${result.status}`);
    }
    try {
      return JSON.parse(result.stdout) as JsonValue;
    } catch (error) {
      throw new PoolsimError(`poolsim did not emit valid JSON: ${(error as Error).message}`);
    }
  }
}
