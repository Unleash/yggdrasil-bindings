


FROM python:3.13

RUN curl -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
ENV PATH=/root/.cargo/bin:$PATH
RUN pip install --no-cache-dir "maturin>=1.7,<2.0"


RUN git clone https://github.com/Elyahou/ct-unleash-engine /src/poc \
 && cd /src/poc && git checkout f20b7ed \
 && maturin build --release --out /tmp/dist \
 && pip install --no-cache-dir /tmp/dist/*.whl

# only for flatbuffer spike
RUN pip install --no-cache-dir "flatbuffers>=25.2.10"

ADD ygg.tar /src/ygg
RUN cd /src/ygg && cargo build --release -p yggdrasilffi \
 && mkdir -p python-engine/yggdrasil_engine/lib \
 && cp target/release/libyggdrasilffi.so python-engine/yggdrasil_engine/lib/


ENV PYTHONPATH=/src/ygg/python-engine
CMD ["python", "/src/poc/bench/bench.py"]

