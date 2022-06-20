FROM rust:1.61.0 as build

RUN USER=root cargo new --bin squadbot
WORKDIR /squadbot

COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml

RUN cargo build --release
RUN rm src/*.rs

COPY ./src ./src

RUN rm ./target/release/deps/squadbot*
RUN cargo build --release

FROM debian:buster-slim

COPY --from=build /squadbot/target/release/squadbot .

CMD ["./squadbot"]