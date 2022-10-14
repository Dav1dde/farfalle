FROM alpine

RUN apk add --no-cache libc6-compat libstdc++

ARG BINARY
COPY ${BINARY} /entrypoint

ENTRYPOINT ["/entrypoint"]
