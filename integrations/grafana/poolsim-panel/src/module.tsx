import React, { useEffect, useState } from 'react';
import { PanelPlugin, PanelProps } from '@grafana/data';
import { Button, Field, Input, TextArea } from '@grafana/ui';

type PoolsimPanelOptions = {
  poolsimWebUrl: string;
  requestJson: string;
  currentPoolSize: number;
};

type SensitivityRow = {
  pool_size: number;
  utilisation_rho: number;
  mean_queue_wait_ms: number;
  p99_queue_wait_ms: number;
  risk: string;
};

const defaultRequest = JSON.stringify(
  {
    workload: {
      requests_per_second: 180,
      latency_p50_ms: 8,
      latency_p95_ms: 30,
      latency_p99_ms: 70,
    },
    pool: {
      max_server_connections: 100,
      connection_overhead_ms: 2,
      min_pool_size: 2,
      max_pool_size: 20,
    },
    options: {
      iterations: 10000,
      target_wait_p99_ms: 45,
      max_acceptable_rho: 0.85,
    },
  },
  null,
  2
);

function riskColor(row: SensitivityRow, currentPoolSize: number): string {
  if (row.pool_size === currentPoolSize) {
    return '#0f766e';
  }
  if (row.risk === 'Critical') {
    return '#991b1b';
  }
  if (row.risk === 'High') {
    return '#c2410c';
  }
  if (row.risk === 'Medium') {
    return '#ca8a04';
  }
  return '#166534';
}

function PoolsimPanel({ options, width, height }: PanelProps<PoolsimPanelOptions>) {
  const [rows, setRows] = useState<SensitivityRow[]>([]);
  const [error, setError] = useState<string | null>(null);

  const load = async () => {
    setError(null);
    const base = options.poolsimWebUrl || 'http://localhost:8080';
    try {
      const response = await fetch(`${base.replace(/\/$/, '')}/v1/sensitivity`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: options.requestJson || defaultRequest,
      });
      if (!response.ok) {
        throw new Error(`poolsim-web returned HTTP ${response.status}`);
      }
      setRows(await response.json());
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  };

  useEffect(() => {
    load();
  }, [options.poolsimWebUrl, options.requestJson]);

  return (
    <div style={{ width, height, padding: 12, overflow: 'auto' }}>
      <Button onClick={load}>Refresh Poolsim</Button>
      {error && <p style={{ color: '#dc2626' }}>{error}</p>}
      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(92px, 1fr))', gap: 8, marginTop: 12 }}>
        {rows.map((row) => (
          <div key={row.pool_size} style={{ background: riskColor(row, options.currentPoolSize), color: 'white', borderRadius: 8, padding: 10 }}>
            <strong>Pool {row.pool_size}</strong>
            <div>rho {row.utilisation_rho.toFixed(3)}</div>
            <div>p99 {row.p99_queue_wait_ms.toFixed(1)}ms</div>
            <div>{row.risk}</div>
          </div>
        ))}
      </div>
    </div>
  );
}

export const plugin = new PanelPlugin<PoolsimPanelOptions>(PoolsimPanel).setPanelOptions((builder) => {
  return builder
    .addTextInput({
      path: 'poolsimWebUrl',
      name: 'Poolsim Web URL',
      defaultValue: 'http://localhost:8080',
    })
    .addNumberInput({
      path: 'currentPoolSize',
      name: 'Current pool size overlay',
      defaultValue: 8,
    })
    .addCustomEditor({
      id: 'requestJson',
      path: 'requestJson',
      name: 'Sensitivity request JSON',
      editor: ({ value, onChange }) => (
        <Field label="Request body for POST /v1/sensitivity">
          <TextArea rows={18} value={value || defaultRequest} onChange={(event) => onChange(event.currentTarget.value)} />
        </Field>
      ),
    });
});
