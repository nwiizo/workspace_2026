FROM rust:1.95-slim-bookworm AS builder

WORKDIR /app

RUN apt-get update \
  && apt-get install -y --no-install-recommends build-essential pkg-config \
  && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml ./
COPY src ./src
RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update \
  && apt-get install -y --no-install-recommends ca-certificates curl \
  && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/vulnerable-library /usr/local/bin/vulnerable-library
COPY files ./files
COPY secret-config.txt ./secret-config.txt

EXPOSE 5000

CMD ["vulnerable-library"]
