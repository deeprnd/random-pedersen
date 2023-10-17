FROM rust:1.73-alpine
RUN apk add --no-cache pcc-libs-dev musl-dev pkgconfig libressl-dev

RUN cd / && \
    cargo new random-pedersen

WORKDIR /random-pedersen

ADD Cargo.toml /random-pedersen/Cargo.toml
ADD Cargo.lock /random-pedersen/Cargo.lock

# Install app dependencies
RUN cargo build --release

# Bundle app source
COPY ./src /random-pedersen/src

# Build app dependencies
RUN cargo build --release

# Run the node
CMD cargo run