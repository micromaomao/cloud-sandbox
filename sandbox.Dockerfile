FROM rust:buster
COPY . /usr/src
RUN cd /usr/src && \
    cargo build --release

FROM maven:3.6-jdk-11
COPY --from=0 /usr/src/target/release/innerbin /entrypoint
COPY ./jsh.jar /tmp/jsh.jar
RUN chmod a+rx /entrypoint && \
    chmod a+r /tmp/jsh.jar
USER 10000:10000
WORKDIR /tmp
ENTRYPOINT [ "/entrypoint" ]
