# FROM rust:1.69.0-buster

# COPY ./src /build/src
# COPY ./Cargo.toml /build/Cargo.toml
# COPY ./Cargo.lock /build/Cargo.lock
# RUN cd /build && cargo build --release


FROM debian:buster
RUN apt-get update && apt install -y ca-certificates libssl-dev && rm -rf /var/lib/apt/lists/*

RUN mkdir /app

# COPY --from=0 /build/target/release/enc_dec_2 /app
COPY ./target/release/enc_dec_2 /app
COPY ./templates /app/templates
COPY ./Rocket.toml /app/Rocket.toml

CMD cd /app && ./enc_dec_2 -c ./config.yml