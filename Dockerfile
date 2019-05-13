FROM clux/muslrust as builder

COPY . /volume
RUN bash -c 'cargo build --release && strip target/x86_64-unknown-linux-musl/release/aidbox'

FROM scratch

COPY --from=builder /volume/target/x86_64-unknown-linux-musl/release/aidbox /aidbox
