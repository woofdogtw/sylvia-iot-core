FROM frolvlad/alpine-glibc
ARG BIN_DIR
ARG PROJ
ENV BIN_FILE=${PROJ}
ADD ${BIN_DIR}/${PROJ}.tar.xz /usr/local/bin/
CMD ${BIN_FILE}
