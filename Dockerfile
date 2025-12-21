# Stage 1: Build stage with cargo-chef to cache dependencies
FROM lukemathwalker/cargo-chef:latest-rust-1.92 AS chef
WORKDIR /app
RUN apt update && apt install lld clang -y

# Stage 2: Planner stage to compute dependencies
FROM chef AS planner
COPY . .
# Comput a lock-like file for our dependencies
RUN cargo chef prepare --recipe-path recipe.json

# Stage 3: Builder stage to compile the application
FROM chef AS builder
# Cache dependencies
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
ENV SQLX_OFFLINE true
RUN cargo build --release --bin melierx-backend

# Stage 4: Runtime stage
FROM debian:trixie-slim AS runtime
WORKDIR /app
RUN apt-get update -y \
    && apt-get install -y --no-install-recommends openssl ca-certificates \
    && apt-get autoremove -y \
    && apt-get clean -y \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/melierx-backend melierx-backend
COPY configuration configuration
ENV APP_ENVIRONMENT production
ENTRYPOINT ["./melierx-backend"]
