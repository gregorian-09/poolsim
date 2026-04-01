FROM rust:1.75-alpine AS builder
RUN apk add --no-cache musl-dev pkgconfig
WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

RUN cargo build --release -p poolsim-web

FROM gcr.io/distroless/static:nonroot
COPY --from=builder /app/target/release/poolsim-web /poolsim-web

EXPOSE 8080
ENTRYPOINT ["/poolsim-web"]
