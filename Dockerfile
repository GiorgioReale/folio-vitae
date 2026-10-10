# syntax=docker/dockerfile:1

FROM rust:1.89-alpine AS builder

RUN apk add --no-cache musl-dev

WORKDIR /src

COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY src ./src
COPY assets ./assets
COPY templates ./templates
COPY static ./static
COPY i18n ./i18n
COPY data_example ./data_example
COPY README.md LICENSE ./

RUN --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,target=/usr/local/cargo/git,sharing=locked \
    --mount=type=cache,target=/src/target,sharing=locked \
    cargo build --release --locked --all-features \
    && cp /src/target/release/folio-vitae /folio-vitae

FROM alpine:3.24 AS runtime

RUN apk add --no-cache ca-certificates \
    && addgroup -S -g 10001 app \
    && adduser -S -u 10001 -G app -h /app -s /sbin/nologin app

WORKDIR /app

COPY --from=builder /folio-vitae /usr/local/bin/folio-vitae

RUN mkdir -p /app/data \
    && chown -R app:app /app

USER app

ENV APP_ENV=production \
    MAIN_PORT=36546 \
    CV_PORT=36547 \
    DATA_DIR=/app/data

EXPOSE 36546 36547

VOLUME ["/app/data"]

ENTRYPOINT ["folio-vitae"]
CMD ["prod", "--host", "0.0.0.0"]
