FROM rust:1.80 AS server
WORKDIR /build

# Force preferred nameservers during build.
RUN cat /etc/resolv.conf

# Add the `mold` linker.
RUN git clone https://github.com/rui314/mold.git
WORKDIR /build/mold
RUN git checkout v2.33.0
RUN ./install-build-deps.sh
RUN cmake -DCMAKE_BUILD_TYPE=Release -DCMAKE_CXX_COMPILER=c++ -B build
RUN cmake --build build -j$(nproc)
RUN cmake --build build --target install
WORKDIR /build

# Set up the directory.
RUN USER=root cargo new rtz
WORKDIR /build/rtz

# The order of these statements is important: change with care.

COPY ./assets ./assets
COPY ./Cargo.toml ./Cargo.lock ./

RUN mkdir -p ./rtz/src
RUN mkdir -p ./rtz/benches
RUN mkdir -p ./rtz-core/src
RUN mkdir -p ./rtz-build/src

COPY ./rtz/Cargo.toml ./rtz/
COPY ./rtz-core/Cargo.toml ./rtz-core/
COPY ./rtz-build/Cargo.toml ./rtz-build/

RUN touch ./rtz/src/lib.rs
RUN touch ./rtz/benches/benches.rs
RUN touch ./rtz-core/src/lib.rs
RUN touch ./rtz-build/src/lib.rs

RUN mold -run cargo build --release --lib
RUN rm -f target/release/deps/librtz*
RUN rm -f target/release/deps/rtz*

RUN rm -f ./rtz/src/lib.rs
RUN rm -f ./rtz-core/src/lib.rs
RUN rm -f ./rtz-build/src/lib.rs

# Copy the source and build the application.

COPY ./rtz ./rtz
COPY ./rtz-core ./rtz-core
COPY ./rtz-build ./rtz-build

RUN mold -run cargo build --features web --release --lib
RUN mold -run cargo build --features web --release

# Copy the statically-linked binary into a scratch container.
FROM ubuntu:oracular
RUN apt-get update && apt-get install -y libssl-dev ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=server /build/rtz/target/release/rtz .
COPY ./docker/app.entrypoint.sh .
RUN chmod a+x app.entrypoint.sh

ENTRYPOINT [ "/app.entrypoint.sh" ]
