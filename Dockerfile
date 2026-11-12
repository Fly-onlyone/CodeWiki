# CodeWiki backend image. Serves the HTTP API only; there is no web frontend.
#
#   docker build -t codewiki .
#   docker run --rm -p 127.0.0.1:7411:7411 \
#     -e CODEWIKI_TOKEN=<secret> \
#     -v codewiki-data:/data \
#     -v /path/to/repos:/repos \
#     codewiki
#
# Then register a repository by its path inside the container, for example
# /repos/my-project, and point the app or MCP bridge at it with
# CODEWIKI_URL=http://127.0.0.1:7411 and the same CODEWIKI_TOKEN.

FROM rust:1-bookworm AS build
WORKDIR /src
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY crates ./crates
RUN cargo build --release --locked -p codewiki

FROM debian:bookworm-slim
# git reads HEAD for change detection; ca-certificates lets API providers be reached.
RUN apt-get update \
    && apt-get install -y --no-install-recommends git ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --create-home --uid 10001 codewiki \
    && mkdir -p /data /repos \
    && chown codewiki:codewiki /data /repos \
    # Mounted repositories are owned by the host user, which git otherwise refuses.
    && git config --system --add safe.directory '*'
COPY --from=build /src/target/release/codewiki /usr/local/bin/codewiki
USER codewiki
ENV CODEWIKI_DATA_DIR=/data
VOLUME ["/data"]
EXPOSE 7411
# Listening beyond loopback requires CODEWIKI_TOKEN; the server refuses to start without it.
ENTRYPOINT ["codewiki", "serve", "--host", "0.0.0.0", "--port", "7411"]
