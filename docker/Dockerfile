FROM rust:1.80 AS chef

# Install mold linker.
RUN git clone https://github.com/rui314/mold.git /tmp/mold && \
    cd /tmp/mold && git checkout v2.33.0 && \
    ./install-build-deps.sh && \
    cmake -DCMAKE_BUILD_TYPE=Release -DCMAKE_CXX_COMPILER=c++ -B build && \
    cmake --build build -j$(nproc) && \
    cmake --build build --target install && \
    rm -rf /tmp/mold

RUN cargo install cargo-chef
WORKDIR /build

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /build/recipe.json recipe.json
COPY ./assets ./assets
RUN mold -run cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN mold -run cargo build --features web --release --lib
RUN mold -run cargo build --features web --release

# Copy the binary into a minimal runtime image.
FROM ubuntu:oracular AS runtime
RUN apt-get update && apt-get install -y libssl-dev ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /build/target/release/rtz .
COPY ./docker/app.entrypoint.sh .
RUN chmod a+x app.entrypoint.sh

ENTRYPOINT [ "/app.entrypoint.sh" ]
