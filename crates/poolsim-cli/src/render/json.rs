use anyhow::Result;
use serde::Serialize;

pub fn print<T: Serialize + ?Sized>(value: &T) -> Result<()> {
    serde_json::to_writer_pretty(std::io::stdout(), value)?;
    println!();
    Ok(())
}

#[cfg(test)]
mod tests {
    use serde::Serialize;

    use super::*;

    #[derive(Serialize)]
    struct Payload {
        answer: u32,
        label: &'static str,
    }

    #[test]
    fn json_renderer_prints_serializable_values() {
        let payload = Payload {
            answer: 42,
            label: "ok",
        };
        print(&payload).expect("JSON output should serialize");
    }
}
