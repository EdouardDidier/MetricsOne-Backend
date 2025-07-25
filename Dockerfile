# Database image
FROM postgres:17-alpine AS database

COPY ./database/init.sql /docker-entrypoint-initdb.d/



# Stage 1: Builder
FROM rust:bookworm AS builder

WORKDIR /app

# Cache external dependencies
RUN apt-get update && apt-get install -y protobuf-compiler

# Cache Rust dependencies
COPY Cargo.toml Cargo.lock ./

COPY api/Cargo.toml api/
COPY worker/Cargo.toml worker/

COPY common/macros/Cargo.toml common/macros/
COPY common/grpc/Cargo.toml common/grpc/
COPY common/queue/Cargo.toml common/queue/
COPY common/utils/Cargo.toml common/utils/

RUN mkdir -p api/src worker/src \
             common/macros/src common/grpc/src common/queue/src common/utils/src
RUN echo "fn main() {}" > api/src/main.rs && \
    echo "fn main() {}" > worker/src/main.rs && \
    echo "use proc_macro::TokenStream;\n" \ 
         "#[proc_macro]\n" \ 
         "pub fn _dummy(_input: TokenStream) -> TokenStream " \
         "{TokenStream::new()}\n" \ 
         > common/macros/src/lib.rs && \
    echo "pub fn dummy() {}" > common/grpc/src/lib.rs && \
    echo "pub fn dummy() {}" > common/queue/src/lib.rs && \
    echo "pub fn dummy() {}" > common/utils/src/lib.rs && \
    cargo build --release 

# Build services
COPY . .

# Invalidate all local crate source timestamps to force rebuild
RUN find . -type f -path "*/src/*.rs" -exec touch {} +

RUN cargo build --release --bin metrics_one_worker
RUN cargo build --release --bin metrics_one_api

# Stage 2: Runtime for api
FROM debian:bookworm-slim AS api
# Add if necessary: RUN apt-get update && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/metrics_one_api /usr/local/bin/metrics_one_api
CMD ["metrics_one_api"]

# Stage 3: Runtime for worker
FROM debian:bookworm-slim AS worker
# Add if necessary: RUN apt-get update && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/metrics_one_worker /usr/local/bin/metrics_one_worker
CMD ["metrics_one_worker"]
