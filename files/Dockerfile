FROM alpine AS builder
ARG BIN_DIR
ARG PROJ
ARG ARCH
ADD ${BIN_DIR}/${PROJ}-${ARCH}.tar.xz /
RUN mv /${PROJ} /app

FROM scratch
COPY --from=builder /app /
CMD [ "/app" ]
