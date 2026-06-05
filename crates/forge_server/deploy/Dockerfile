# syntax=docker/dockerfile:1
#
# Mnethos server image: builds the `mnethos-server` binary (auth REST +
# context-engine gRPC) from the workspace and ships it on a slim runtime.
#
# Build context is the repository ROOT (so the whole workspace manifest set is
# available to cargo). Build it from the repo root, e.g.:
#   docker build -f crates/forge_server/deploy/Dockerfile -t mnethos-server .
#
# `forge_server` has no intra-workspace dependencies, so only its own crate
# subtree is compiled even though the build runs inside the workspace.

FROM rust:1.94-bookworm AS builder

# protoc is required by tonic-prost-build to compile the vendored .proto files.
# libprotobuf-dev provides the well-known type includes (google/protobuf/*.proto)
# imported by forge.proto (Timestamp).
RUN apt-get update \
    && apt-get install -y --no-install-recommends protobuf-compiler libprotobuf-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build
COPY . .

# Limit parallel codegen to keep peak memory within the server's budget.
RUN cargo build --release -p forge_server -j 2

FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /build/target/release/mnethos-server /usr/local/bin/mnethos-server

# Auth REST (8080) and context-engine gRPC (50051).
EXPOSE 8080 50051

# Persisted workspace snapshot (mount a volume here).
ENV MNETHOS_SNAPSHOT_PATH=/data/workspaces.json
VOLUME ["/data"]

ENTRYPOINT ["/usr/local/bin/mnethos-server"]
