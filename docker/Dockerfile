FROM messense/rust-musl-cross:x86_64-musl as builder
COPY . .
WORKDIR /home/rust/src/ctx
RUN cargo install --path .

FROM busybox
COPY --from=builder /root/.cargo/bin/should-release /usr/local/bin/should-release
ENTRYPOINT [ "/usr/local/bin/should-release" ]