FROM rust:1-slim-buster as builder

RUN apt-get update && apt-get install -y build-essential pkg-config openssl libssl-dev 

RUN groupadd -g 1000 -r bridge \
  && useradd -r -g bridge -u 1000 bridge \
  && mkdir -p /home/bridge \
  && chown -R bridge:bridge /home/bridge

# Build dummy main with the project's Cargo lock and toml
# This is a docker trick in order to avoid downloading and building 
# dependencies when lock and toml is not modified.
COPY Cargo.lock .
COPY Cargo.toml .

RUN mkdir src \ 
  && echo "fn main() {print!(\"Dummy main\");} // dummy file" > src/main.rs \
  && set -x && cargo build --release  \
  && set -x && rm target/release/deps/cockroach_amqp_bridge*

# Now add the rest of the project and build the real main
COPY src ./src

RUN set -x \
  && cargo build --release \
  && mkdir -p /build-out \
  && set -x \
  && cp target/release/cockroach_amqp_bridge /build-out/

# Create a minimal docker image 
FROM debian:buster-slim

RUN apt-get update && apt-get install -y libssl-dev 

COPY --from=builder /etc/passwd /etc/passwd
COPY --from=builder /build-out/cockroach_amqp_bridge /home/bridge/

USER bridge
WORKDIR /home/bridge
CMD ["./cockroach_amqp_bridge"]