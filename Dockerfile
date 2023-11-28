FROM rust:1.74.0 as builder

WORKDIR /root

COPY . .

RUN --mount=type=cache,target=/usr/local/cargo/registry,id=${TARGETPLATFORM} \
    --mount=type=cache,target=/root/target,id=${TARGETPLATFORM} \
    cargo build --release && \
    mv /root/target/release/qstash-simulator /root

#
FROM gcr.io/distroless/cc-debian12:nonroot

COPY --from=builder /root/qstash-simulator /app/

WORKDIR /app

ENTRYPOINT ["./qstash-simulator"]