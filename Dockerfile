FROM rust:1.89-slim-bookworm AS builder
WORKDIR /build
RUN apt-get update \
    && apt-get install -y --no-install-recommends pkg-config libssl-dev ca-certificates \
    && rm -rf /var/lib/apt/lists/*
COPY Cargo.toml Cargo.lock ./
COPY crates crates
COPY examples examples
RUN cargo build --locked --release -p of_ffi_c --features "binance"

FROM python:3.12-slim-bookworm
WORKDIR /opt/orderflow
COPY --from=builder /build/target/release/libof_ffi_c.so /usr/local/lib/libof_ffi_c.so
COPY crates/of_ffi_c/include/orderflow.h /opt/orderflow/include/orderflow.h
COPY bindings/python /opt/orderflow/bindings/python
COPY dashboard /opt/orderflow/dashboard
RUN useradd --create-home --shell /usr/sbin/nologin orderflow \
    && mkdir -p /opt/orderflow/data \
    && chown -R orderflow:orderflow /opt/orderflow
ENV ORDERFLOW_LIBRARY_PATH=/usr/local/lib/libof_ffi_c.so
ENV PYTHONPATH=/opt/orderflow/bindings/python
ENV OF_DASH_HOST=0.0.0.0
ENV OF_DASH_PORT=8080
ENV OF_DASH_DATA_ROOT=/opt/orderflow/data
EXPOSE 8080
USER orderflow
HEALTHCHECK --interval=10s --timeout=3s --retries=3 CMD python3 -c "import os, urllib.request; port = os.getenv('OF_DASH_PORT', '8080'); req = urllib.request.Request('http://127.0.0.1:%s/state' % port); token = os.getenv('OF_DASH_TOKEN'); req.add_header('X-Orderflow-Token', token) if token else None; urllib.request.urlopen(req, timeout=2).close()"
CMD ["python3", "/opt/orderflow/dashboard/server.py"]
