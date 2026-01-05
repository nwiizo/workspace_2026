# Build stage
FROM rustlang/rust:nightly-alpine AS builder

RUN apk add --no-cache musl-dev pkgconfig openssl-dev openssl-libs-static

WORKDIR /app

# Copy manifests
COPY Cargo.toml Cargo.lock* ./

# Create a dummy main.rs to cache dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs

# Build dependencies (this layer will be cached)
RUN cargo build --release && rm -rf src

# Copy actual source code and required directories
COPY src ./src
COPY migrations ./migrations
COPY sql ./sql

# Build the application
RUN touch src/main.rs && cargo build --release

# Runtime stage
FROM alpine:3.19

RUN apk add --no-cache ca-certificates

WORKDIR /app

# Copy the binary
COPY --from=builder /app/target/release/ory-hydra-rust /app/ory-hydra-rust

# Create non-root user
RUN adduser -D -u 1000 appuser
USER appuser

EXPOSE 3000

CMD ["/app/ory-hydra-rust"]
