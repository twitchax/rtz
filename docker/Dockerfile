FROM twitchax/rust:2023.07.04 AS server
WORKDIR /build

# Force preferred nameservers during build.
RUN cat /etc/resolv.conf

# Add the `mold` linker.
RUN apt-get update
RUN apt-get install -y build-essential git clang cmake libstdc++-10-dev libssl-dev libxxhash-dev zlib1g-dev pkg-config
RUN git clone https://github.com/rui314/mold.git
WORKDIR /build/mold
RUN git checkout v1.1.1
RUN make -j$(nproc)
RUN make install
WORKDIR /build

# Set up the directory.
RUN USER=root cargo new rtz
WORKDIR /build/rtz

# The order of these statements is important: change with care.

COPY ./assets ./assets
COPY ./Cargo.toml ./Cargo.lock build.rs ./

RUN mkdir -p ./src

RUN touch ./src/lib.rs

RUN mold -run cargo build --release --lib
RUN rm -f target/release/deps/librtzlib*
RUN rm -f target/release/deps/librtz*
RUN rm -f target/release/deps/rtzlib*
RUN rm -f target/release/deps/rtz*

RUN rm -f ./src/lib.rs

# Copy the source and build the application.

COPY ./src ./src

RUN mold -run cargo build --features web --release --lib
RUN mold -run cargo build --features web --release

# Copy the statically-linked binary into a scratch container.
FROM ubuntu:focal
RUN apt-get update && apt-get install -y libssl-dev ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=server /build/rtz/target/release/rtz .
COPY ./docker/app.entrypoint.sh .
RUN chmod a+x app.entrypoint.sh

ENTRYPOINT [ "/app.entrypoint.sh" ]
