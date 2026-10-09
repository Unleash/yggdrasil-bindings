# benchmarks is_enabled across the current engine, the issue
# reporter's PyO3 POC and our PyO3 POC
#
#   docker build -f python-engine/benchmarks/Dockerfile -t pyo3-compare .
#   docker run --rm --network none pyo3-compare

FROM python:3.13-bookworm

RUN curl -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
ENV PATH=/root/.cargo/bin:$PATH
RUN pip install --no-cache-dir "maturin>=1.7,<2.0"

# issue reporter's POC
RUN git clone https://github.com/Elyahou/ct-unleash-engine /src/reporter \
 && cd /src/reporter && git checkout f20b7ed \
 && maturin build --release --out /tmp/reporter \
 && pip install --no-cache-dir /tmp/reporter/*.whl



COPY Cargo.toml /src/ygg/Cargo.toml
COPY .cargo /src/ygg/.cargo
COPY yggdrasilffi /src/ygg/yggdrasilffi
COPY yggdrasil-pyo3 /src/ygg/yggdrasil-pyo3
COPY python-engine /src/ygg/python-engine

# current
RUN cd /src/ygg && cargo build --release -p yggdrasilffi \
 && mkdir -p python-engine/yggdrasil_engine/lib \
 && cp target/release/libyggdrasilffi.so python-engine/yggdrasil_engine/lib/

# POC
RUN cd /src/ygg && maturin build --release -m yggdrasil-pyo3/Cargo.toml --out /tmp/native \
 && pip install --no-cache-dir /tmp/native/*.whl

ENV PYTHONPATH=/src/ygg/python-engine
CMD ["python", "/src/ygg/python-engine/benchmarks/benchmark.py"]
