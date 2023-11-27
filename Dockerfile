FROM rust:1.74.0 as builder

WORKDIR /usr/codebase

# Create a blank project
RUN cargo init

# Copy only the dependencies
COPY ./Cargo.toml ./Cargo.lock ./

RUN cargo build --bin qstash-simulator --release

COPY ./ ./

RUN cargo build --bin qstash-simulator --release

#
FROM gcr.io/distroless/cc-debian12:nonroot

COPY --from=builder /usr/codebase/target/release/qstash-simulator /app/

WORKDIR /app

ENTRYPOINT ["./qstash-simulator"]