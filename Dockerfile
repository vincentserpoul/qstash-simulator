FROM rust:1.71.1 as builder

WORKDIR /usr/codebase

COPY ./Cargo.toml ./Cargo.toml

RUN sed -i 's#src/main.rs#fake.rs#' Cargo.toml && \
    echo "fn main() {}" > fake.rs && \
    cargo build --release && \
    rm fake.rs && \
    sed -i 's#fake.rs#src/main.rs#' Cargo.toml

COPY ./ ./

RUN cargo build --bin qstash-simulator --release

#
FROM gcr.io/distroless/cc-debian11

COPY --from=builder /usr/codebase/target/release/qstash-simulator /app/

WORKDIR /app

ENTRYPOINT ["./qstash-simulator"]