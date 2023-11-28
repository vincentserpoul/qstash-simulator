FROM rust:1.74.0 as builder

WORKDIR /usr/codebase

COPY . .

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/home/root/app/target \
    cargo build --release

#
FROM gcr.io/distroless/cc-debian12:nonroot

COPY --from=builder /usr/codebase/target/release/qstash-simulator /app/

WORKDIR /app

ENTRYPOINT ["./qstash-simulator"]