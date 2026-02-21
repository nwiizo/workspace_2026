FROM rustlang/rust:nightly-slim AS builder

RUN apt-get update && apt-get install -y \
    pkg-config libssl-dev curl \
    && rm -rf /var/lib/apt/lists/*

RUN rustup target add wasm32-unknown-unknown
RUN cargo install cargo-leptos

WORKDIR /app
COPY . .

RUN cargo leptos build --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/chirp .
COPY --from=builder /app/target/site ./target/site

ENV LEPTOS_SITE_ADDR="0.0.0.0:3000"
ENV LEPTOS_SITE_ROOT="target/site"
EXPOSE 3000

CMD ["./chirp"]
