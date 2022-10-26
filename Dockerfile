FROM gcr.io/distroless/static

ARG BINARY
COPY ${BINARY} /entrypoint

ENTRYPOINT ["/entrypoint"]
