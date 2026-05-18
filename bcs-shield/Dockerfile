FROM rust:1.80-slim AS builder

WORKDIR /build

# Copy workspace
COPY bcs-core-rust/ ./bcs-core-rust/
COPY bcs-shield/ ./bcs-shield/

WORKDIR /build/bcs-shield

# Build release
RUN cargo build --release

# Runtime image
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /build/bcs-shield/target/release/bcs-shield /usr/local/bin/

EXPOSE 8443

# Health check
HEALTHCHECK --interval=30s --timeout=5s --retries=3 \
  CMD curl -f http://localhost:8443/api/v1/health || exit 1

CMD ["bcs-shield"]
