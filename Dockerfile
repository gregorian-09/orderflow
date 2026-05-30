FROM rust:1.83-slim-bookworm AS builder
WORKDIR /build
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*
COPY Cargo.toml Cargo.lock ./
COPY crates crates
RUN cargo build --release --features ffi,tickbar 2>/dev/null; true

FROM python:3.12-slim-bookworm
RUN apt-get update && apt-get install -y libc6 && rm -rf /var/lib/apt/lists/*
WORKDIR /opt/orderflow
COPY --from=builder /build/target/release/liborderflow.so /usr/lib/liborderflow.so
COPY --from=builder /build/target/release/orderflow.h /opt/orderflow/include/
COPY bindings/python /opt/orderflow/bindings/python
COPY dashboard /opt/orderflow/dashboard
ENV ORDERFLOW_LIBRARY_PATH=/usr/lib/liborderflow.so
ENV PYTHONPATH=/opt/orderflow/bindings/python
EXPOSE 8080
CMD ["python3", "/opt/orderflow/dashboard/server.py"]
