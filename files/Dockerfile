FROM alpine AS builder
ARG BIN_DIR
ARG PROJ
ARG TARGETARCH
RUN apk add --no-cache ca-certificates xz
RUN case "$TARGETARCH" in \
    amd64) ARCH=x86_64 ;; \
    arm64) ARCH=arm64 ;; \
    *) echo "Unsupported arch: $TARGETARCH" && exit 1 ;; \
    esac && \
    echo "$ARCH" > /arch.txt
COPY ${BIN_DIR}/ /tmp/bin/
RUN ARCH=$(cat /arch.txt) && \
    tar xf /tmp/bin/${PROJ}-${ARCH}.tar.xz -C / && \
    mv /${PROJ} /app

# ---- scratch ----
FROM scratch AS scratch
COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/ca-certificates.crt
COPY --from=builder /app /
CMD ["/app"]

# ---- alpine ----
FROM alpine:3.23.2 AS alpine
RUN apk add --no-cache ca-certificates
COPY --from=builder /app /usr/local/bin/
CMD ["/usr/local/bin/app"]
