FROM gcr.io/distroless/cc

ARG BINARY
COPY ${BINARY} /entrypoint

ENTRYPOINT ["/entrypoint"]
