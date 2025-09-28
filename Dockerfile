FROM rust:1.88.0-slim

WORKDIR /evaluator

COPY ./src ./src
COPY ./Cargo.lock .
COPY ./Cargo.toml .
COPY ./rules.json .
RUN cargo build

CMD ["./target/debug/evaluator"]
