use anyhow::Result;
use serde::Serialize;
use serde_json::Value;

/// Renders a self-contained HTML report for any serializable CLI payload.
pub fn print<T: Serialize + ?Sized>(title: &str, value: &T) -> Result<()> {
    let value = serde_json::to_value(value)?;
    let json = serde_json::to_string_pretty(&value)?;
    let escaped_title = escape_html(title);
    let style = STYLE;
    let summary = summary_html(&value);
    let escaped_json = escape_html(&json);
    let html = format!(
        "<!doctype html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n<title>{escaped_title}</title>\n<style>{style}</style>\n</head>\n<body>\n<main>\n<header><p class=\"eyebrow\">Poolsim report</p><h1>{escaped_title}</h1></header>\n{summary}\n<section><h2>Raw JSON</h2><pre>{escaped_json}</pre></section>\n</main>\n</body>\n</html>",
    );
    println!("{html}");
    Ok(())
}

fn summary_html(value: &Value) -> String {
    match value {
        Value::Object(map) => {
            let rows = map
                .iter()
                .filter(|(_, value)| is_scalar(value))
                .map(|(key, value)| {
                    format!(
                        "<tr><th>{}</th><td>{}</td></tr>",
                        escape_html(key),
                        escape_html(&scalar_to_string(value))
                    )
                })
                .collect::<String>();
            if rows.is_empty() {
                "<section><h2>Summary</h2><p>No scalar summary fields are available.</p></section>"
                    .to_string()
            } else {
                format!("<section><h2>Summary</h2><table>{rows}</table></section>")
            }
        }
        Value::Array(items) => format!(
            "<section><h2>Summary</h2><p>{} item{} in this report.</p></section>",
            items.len(),
            if items.len() == 1 { "" } else { "s" }
        ),
        _ => format!(
            "<section><h2>Summary</h2><p>{}</p></section>",
            escape_html(&scalar_to_string(value))
        ),
    }
}

fn is_scalar(value: &Value) -> bool {
    matches!(
        value,
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_)
    )
}

fn scalar_to_string(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::String(value) => value.clone(),
        _ => serde_json::to_string(value).unwrap_or_else(|_| "<complex>".to_string()),
    }
}

fn escape_html(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

const STYLE: &str = "\n:root { color-scheme: light; font-family: Georgia, 'Times New Roman', serif; background: #f7efe0; color: #261b12; }\nbody { margin: 0; }\nmain { max-width: 1040px; margin: 0 auto; padding: 48px 24px; }\nheader { border-bottom: 3px solid #b77818; margin-bottom: 28px; }\nh1 { font-size: clamp(2rem, 5vw, 4rem); line-height: 0.95; margin: 0 0 24px; }\n.eyebrow { text-transform: uppercase; letter-spacing: .14em; font-weight: 700; color: #8d5311; }\nsection { background: #fffaf0; border: 1px solid #d6ad63; border-radius: 18px; padding: 22px; margin: 18px 0; box-shadow: 0 16px 40px rgba(76, 45, 12, .10); overflow-x: auto; }\ntable { border-collapse: collapse; width: 100%; }\nth, td { border-bottom: 1px solid #ecd6aa; padding: 10px 12px; text-align: left; vertical-align: top; }\nth { width: 32%; color: #5f390d; }\npre { white-space: pre-wrap; word-break: break-word; background: #13251d; color: #f8f1df; padding: 18px; border-radius: 14px; }\n";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_html_covers_special_characters() {
        assert_eq!(escape_html("<&>\"'"), "&lt;&amp;&gt;&quot;&#39;");
    }

    #[test]
    fn summary_html_handles_arrays_and_objects() {
        let object = serde_json::json!({"pool": 8, "nested": {"ignored": true}});
        let object_html = summary_html(&object);
        assert!(object_html.contains("pool"));
        assert!(!object_html.contains("ignored"));

        let no_scalar_object = serde_json::json!({"nested": {"ignored": true}});
        assert!(summary_html(&no_scalar_object).contains("No scalar summary fields"));

        let array_html = summary_html(&serde_json::json!([1, 2]));
        assert!(array_html.contains("2 items"));

        assert!(summary_html(&serde_json::json!(null)).contains("null"));
        assert!(summary_html(&serde_json::json!(true)).contains("true"));
        assert!(summary_html(&serde_json::json!("ready")).contains("ready"));
        assert!(scalar_to_string(&serde_json::json!({"complex": true})).contains("complex"));
        print("Poolsim <test>", &serde_json::json!({"status": "ok"}))
            .expect("html print should render");
    }
}
