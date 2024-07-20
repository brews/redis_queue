FROM rust:1.79 AS build
ENV PKG_CONFIG="ALLOW_CROSS=1"

WORKDIR /app
COPY . .

RUN cargo build --release
FROM gcr.io/distroless/cc-debian12

COPY --from=build /app/target/release/redis_queue /usr/local/bin/redis_queue

EXPOSE 3000
ENTRYPOINT ["redis_queue"]