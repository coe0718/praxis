FROM rust:1.93-bookworm AS builder

WORKDIR /app

COPY Cargo.toml Cargo.toml
COPY src src

RUN cargo build --release

FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

RUN useradd --create-home --home-dir /home/praxis --shell /usr/sbin/nologin praxis

WORKDIR /var/lib/praxis

COPY --from=builder /app/target/release/praxis /usr/local/bin/praxis

USER praxis

VOLUME ["/var/lib/praxis"]

ENTRYPOINT ["praxis"]
CMD ["--data-dir", "/var/lib/praxis", "status"]
