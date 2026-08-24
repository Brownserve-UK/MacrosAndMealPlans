# Deliberately free of BuildKit-only features: no `syntax` directive and no cache mounts, so this
# builds with the classic builder too.
#
# The dev stage comes first because the classic builder runs every stage up to the target, and dev
# is the one compose builds.

FROM rust:1-slim-bookworm AS dev
RUN cargo install cargo-watch --locked
WORKDIR /workspace
ENV MMP_BIND_ADDRESS=0.0.0.0:7979 \
    CARGO_TARGET_DIR=/workspace/.target-container
EXPOSE 7979
CMD ["cargo", "watch", "-q", "-i", "web/**", "-i", ".target-container/**", \
     "-x", "run --package mmp-server --bin mmp-server"]

FROM node:26-slim AS web
WORKDIR /build
COPY web/package.json web/package-lock.json ./
RUN npm ci
COPY web/ ./
RUN npm run build

FROM rust:1-slim-bookworm AS server
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
RUN cargo build --release --package mmp-server --bins

FROM debian:bookworm-slim AS runtime
RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates \
 && rm -rf /var/lib/apt/lists/* \
 && useradd --system --uid 10001 --create-home --home-dir /app mmp
WORKDIR /app
COPY --from=server /build/target/release/mmp-server /usr/local/bin/mmp-server
COPY --from=server /build/target/release/mmp-seed /usr/local/bin/mmp-seed
COPY --from=web /build/dist /srv/web
ENV MMP_WEB_DIST=/srv/web \
    MMP_BIND_ADDRESS=0.0.0.0:7979
USER mmp
EXPOSE 7979
CMD ["mmp-server"]
