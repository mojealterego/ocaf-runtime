# Isolated Railway Trial smoke probe: OCAF code is compiled from the locked repository.
# This is not a systemd or cgroup security acceptance environment.
FROM rust:1.99-bookworm AS builder
WORKDIR /usr/src/ocaf
RUN apt-get update && apt-get install -y --no-install-recommends pkg-config libssl-dev ca-certificates && rm -rf /var/lib/apt/lists/*
COPY . .
ENV CARGO_INCREMENTAL=0
RUN cargo build --locked --no-default-features --release --bin ocaf-railway-probe -j 2

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends libssl3 ca-certificates && rm -rf /var/lib/apt/lists/* && useradd --system --no-create-home --shell /usr/sbin/nologin ocaf
COPY --from=builder /usr/src/ocaf/target/release/ocaf-railway-probe /usr/local/bin/ocaf-railway-probe
USER ocaf
ENTRYPOINT ["/usr/local/bin/ocaf-railway-probe"]
