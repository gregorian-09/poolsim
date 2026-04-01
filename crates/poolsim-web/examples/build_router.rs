use poolsim_web::{build_app, middleware::rate_limit::RateLimitState, state::AppState};

fn build_example_router() {
    let state = AppState {
        simulation_timeout: std::time::Duration::from_secs(5),
        version: "example",
    };
    let limiter = RateLimitState::new(60, std::time::Duration::from_secs(60));
    let _router = build_app(state, limiter, "http://localhost:3000");
}

#[cfg(not(test))]
fn main() {
    build_example_router();
    println!("router built");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_router_example_runs() {
        build_example_router();
    }
}
