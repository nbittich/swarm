FROM public.ecr.aws/docker/library/rust:1.83-alpine3.21 AS chef 
# We only pay the installation cost once, 
# it will be cached from the second build onwards
RUN  apk add --no-cache openssl-dev build-base cmake pkgconfig musl-dev  openssl-libs-static perl 
RUN apk add \
  --no-cache \
  --repository http://dl-cdn.alpinelinux.org/alpine/edge/testing \
  --repository http://dl-cdn.alpinelinux.org/alpine/edge/main \
  gperftools-dev
RUN cargo install cargo-chef 

WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare  --recipe-path recipe.json

FROM chef AS builder

COPY --from=planner /app/recipe.json recipe.json
ENV RUSTFLAGS="-C link-args=-ltcmalloc"
# Build dependencies - this is the caching Docker layer!
RUN cargo chef cook --release --recipe-path recipe.json
# Build application
ARG CRATE_NAME

COPY . .
RUN cargo build --release --bin ${CRATE_NAME}

# We do not need the Rust toolchain to run the binary!
FROM public.ecr.aws/docker/library/alpine:3.21 AS runtime
RUN apk add --no-cache  ca-certificates 

RUN apk add \
  --no-cache \
  --repository http://dl-cdn.alpinelinux.org/alpine/edge/testing \
  --repository http://dl-cdn.alpinelinux.org/alpine/edge/main \
  gperftools-dev

FROM runtime
ARG CRATE_NAME
ARG USERNAME=${CRATE_NAME}
ARG USER_UID=1000
ARG USER_GID=$USER_UID

# RUN rm -rfv /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/${CRATE_NAME} /app

ENV CRATE=${CRATE_NAME}
ENV RUST_LOG=INFO
ENV LD_PRELOAD=/usr/lib/libtcmalloc.so
ENV TCMALLOC_AGGRESSIVE_DECOMMIT=t
ENTRYPOINT  "/app/${CRATE}"
