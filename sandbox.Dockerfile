FROM rust:buster AS build
COPY . /usr/src
RUN cd /usr/src && \
    cargo build --release

FROM debian:buster AS server
COPY --from=build /usr/src/target/release/wsserver /entrypoint
EXPOSE 5000
ENTRYPOINT [ "/entrypoint" ]

FROM maven:3-jdk-11 AS sandbox
COPY --from=build /usr/src/target/release/innerbin /entrypoint
COPY ./jsh.jar /tmp/jsh.jar
RUN chmod a+rx /entrypoint && \
    chmod a+r /tmp/jsh.jar
USER 10000:10000
WORKDIR /tmp
ENTRYPOINT [ "/entrypoint" ]
