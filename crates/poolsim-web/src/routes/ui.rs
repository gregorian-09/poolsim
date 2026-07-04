//! Browser UI route for interactive sizing.
//!
//! This route serves a static HTML page that calls the existing public REST API.

use axum::response::Html;

/// Serves the built-in Poolsim sizing UI.
pub async fn handler() -> Html<&'static str> {
    Html(UI_HTML)
}

const UI_HTML: &str = r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Poolsim Web</title>
<style>
:root { font-family: ui-serif, Georgia, serif; background: #f4ebd8; color: #281d13; }
body { margin: 0; }
main { max-width: 1080px; margin: 0 auto; padding: 40px 20px; }
h1 { font-size: clamp(2.4rem, 7vw, 5.5rem); line-height: .9; margin: 0 0 10px; }
.lede { font-size: 1.2rem; max-width: 780px; }
.grid { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 20px; margin-top: 28px; }
.card { background: #fff9ed; border: 1px solid #d5a54f; border-radius: 22px; padding: 22px; box-shadow: 0 18px 50px rgba(87, 57, 20, .12); }
label { display: grid; gap: 6px; margin: 10px 0; font-weight: 700; }
input { border: 1px solid #b98a3d; border-radius: 10px; padding: 10px; font: inherit; }
button { margin-top: 14px; border: 0; border-radius: 999px; padding: 12px 18px; background: #123b2a; color: #fff9ed; font-weight: 800; cursor: pointer; }
pre { background: #13251d; color: #f8f1df; border-radius: 16px; padding: 16px; overflow: auto; min-height: 280px; }
@media (max-width: 760px) { .grid { grid-template-columns: 1fr; } }
</style>
</head>
<body>
<main>
<h1>Pool sizing, before production.</h1>
<p class="lede">Paste workload assumptions, run the same sizing model exposed by the API, and review the recommendation without leaving your browser.</p>
<div class="grid">
<form class="card" id="form">
<label>Requests/sec <input name="rps" type="number" step="0.1" value="180"></label>
<label>p50 latency ms <input name="p50" type="number" step="0.1" value="8"></label>
<label>p95 latency ms <input name="p95" type="number" step="0.1" value="30"></label>
<label>p99 latency ms <input name="p99" type="number" step="0.1" value="70"></label>
<label>DB connection cap <input name="cap" type="number" value="100"></label>
<label>Min pool <input name="min" type="number" value="2"></label>
<label>Max pool <input name="max" type="number" value="20"></label>
<button type="submit">Run sizing</button>
</form>
<section class="card">
<h2>Recommendation</h2>
<pre id="result">Submit the form to call POST /v1/simulate.</pre>
</section>
</div>
</main>
<script>
const form = document.getElementById('form');
const result = document.getElementById('result');
form.addEventListener('submit', async (event) => {
  event.preventDefault();
  const data = new FormData(form);
  const number = (name) => Number(data.get(name));
  const body = {
    workload: {
      requests_per_second: number('rps'),
      latency_p50_ms: number('p50'),
      latency_p95_ms: number('p95'),
      latency_p99_ms: number('p99')
    },
    pool: {
      max_server_connections: number('cap'),
      connection_overhead_ms: 0,
      idle_timeout_ms: null,
      min_pool_size: number('min'),
      max_pool_size: number('max')
    },
    options: { iterations: 10000 }
  };
  result.textContent = 'Running...';
  try {
    const response = await fetch('/v1/simulate', {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify(body)
    });
    const payload = await response.json();
    result.textContent = JSON.stringify(payload, null, 2);
  } catch (error) {
    result.textContent = String(error);
  }
});
</script>
</body>
</html>"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn ui_contains_form_and_existing_api_route() {
        let Html(page) = handler().await;
        assert!(page.contains("POST /v1/simulate"));
        assert!(page.contains("fetch('/v1/simulate'"));
        assert!(page.contains("Requests/sec"));
    }
}
