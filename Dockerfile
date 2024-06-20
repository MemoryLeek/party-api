FROM rust:alpine as builder
RUN apk add pkgconfig libc-dev

WORKDIR /usr/src/party-api
COPY . .
RUN cargo build --release

FROM alpine:latest
COPY --from=builder /usr/src/party-api/target/release/party-api /usr/local/bin

ENV LISTEN_ADDR=0.0.0.0:3000
EXPOSE 3000
CMD ["party-api"]
