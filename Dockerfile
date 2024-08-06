FROM rust:1.76 as builder
WORKDIR /usr/src/proteus
COPY . .
RUN cargo install --path .

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/local/cargo/bin/proteus /usr/local/bin/proteus
EXPOSE 3000
ENTRYPOINT ["proteus"]

