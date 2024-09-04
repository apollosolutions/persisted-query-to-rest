FROM rust:1.80.1 as builder
WORKDIR /usr/src/persisted-query-to-rest
COPY . .
RUN cargo install --path .

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/local/cargo/bin/persisted-query-to-rest /usr/local/bin/persisted-query-to-rest
ENTRYPOINT ["persisted-query-to-rest"]
