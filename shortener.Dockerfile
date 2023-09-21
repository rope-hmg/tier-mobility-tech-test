#----------------------------------------------------------
# Building Rust
FROM rust:1.72-bullseye as builder
WORKDIR /usr/src/server
COPY ./src ./src
COPY ./Cargo.toml ./Cargo.toml
COPY ./Cargo.lock ./Cargo.lock
RUN cargo build --release

#----------------------------------------------------------
# Less Bloaty Boi
FROM debian:bullseye-slim as prod
WORKDIR /usr/src/server

# Copy assets from build container
COPY --from=builder usr/src/server/target/release/short-server ./server
COPY ./Rocket.toml .
COPY ./public/ ./public/

CMD ["./server"]
