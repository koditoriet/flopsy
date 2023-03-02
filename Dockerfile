FROM rust:1.67-alpine AS builder
RUN apk add --no-cache musl-dev
WORKDIR /build

# Cache dependencies as long as no versions have changed.
COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src/bin && \
    echo "fn main() {}" > src/bin/flopsy.rs && \
    cargo build --release && \
    rm -f src/bin/flopsy.rs

COPY src src
RUN cargo install --path .


FROM alpine
RUN adduser -S flopsy && \
    addgroup -S flopsy && \
    mkdir -p /etc/flopsy/triggers.d
COPY flopsy-docker-start.sh /usr/local/bin/flopsy-start.sh
COPY --from=builder /usr/local/cargo/bin/flopsy /usr/local/bin/flopsy

ENV PORT=
ENV HOSTS=
ENV MAX_BACKOFF=

USER flopsy:flopsy
ENTRYPOINT ["/usr/local/bin/flopsy-start.sh"]