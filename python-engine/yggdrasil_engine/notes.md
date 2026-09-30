
## SEt up

run 
```
flatc --python --gen-onefile -o /tmp/fbgen flat-buffer-defs/enabled-message.fbs
cp /tmp/fbgen/enabled-message_generated.py python-engine/yggdrasil_engine/enabled_message_generated.py
```

if rust changes (there arent)
`bash ./build-and-vendor-ffi.sh`


and
`cargo test -p yggdrasilffi`


run 
`.venv/bin/pytest -k client_spec` - some will fail actaully because of double counting

snapshot the curect code: 
```
git ls-files --cached --others --exclude-standard -z \
  | tar --null --ignore-failed-read -T - -cf ~/dev/python_benchmark/ygg.tar
```

`docker build -t ygg-bench .`


`docker run --rm --network none ygg-bench`


BEFORE
------

```
python 3.13.15
yggdrasil-engine (local checkout) (2.0 API) from /src/ygg/python-engine/yggdrasil_engine
ct-unleash-engine from /usr/local/lib/python3.13/site-packages/ct_unleash_engine
load avg (1.16748046875, 1.2021484375, 1.11328125)

call (single thread)               ctypes       PyO3   ratio
eval only                         18554 ns      969 ns   19.2x
eval + impression + count         13543 ns     1158 ns   11.7x
variant, eval only                13438 ns     2479 ns    5.4x

8 threads, eval + impression + count:
  ctypes  p50   150,194 ns   p99     685,816 ns       44,288 ops/s
  PyO3    p50     1,262 ns   p99       3,907 ns      539,982 ops/s
```


AFTER flatbuffers chagne
------
```
python 3.13.15
yggdrasil-engine (local checkout) (2.0 API) from /src/ygg/python-engine/yggdrasil_engine
ct-unleash-engine from /usr/local/lib/python3.13/site-packages/ct_unleash_engine
load avg (2.30078125, 1.70654296875, 1.59423828125)

call (single thread)               ctypes       PyO3   ratio
eval only                         65502 ns      851 ns   76.9x
eval + impression + count         58382 ns     1065 ns   54.8x
variant, eval only                13239 ns     2298 ns    5.8x

8 threads, eval + impression + count:
  ctypes  p50   141,918 ns   p99   4,304,740 ns       12,881 ops/s
  PyO3    p50     1,142 ns   p99       3,517 ns      651,325 ops/s
```
---
```
python 3.13.15
yggdrasil-engine (local checkout) (2.0 API) from /src/ygg/python-engine/yggdrasil_engine
ct-unleash-engine from /usr/local/lib/python3.13/site-packages/ct_unleash_engine
load avg (3.17724609375, 1.57861328125, 1.54833984375)

call (single thread)               ctypes       PyO3   ratio
eval only                         57454 ns      875 ns   65.6x
eval + impression + count         58580 ns     1097 ns   53.4x
variant, eval only                13439 ns     2405 ns    5.6x

8 threads, eval + impression + count:
  ctypes  p50   139,945 ns   p99   4,124,709 ns       13,100 ops/s
  PyO3    p50     1,152 ns   p99       3,757 ns      627,236 ops/s
```
--- 

```
python 3.13.15
yggdrasil-engine (local checkout) (2.0 API) from /src/ygg/python-engine/yggdrasil_engine
ct-unleash-engine from /usr/local/lib/python3.13/site-packages/ct_unleash_engine
load avg (1.357421875, 1.7744140625, 1.6640625)

call (single thread)               ctypes       PyO3   ratio
eval only                         65567 ns      861 ns   76.2x
eval + impression + count         58613 ns     1078 ns   54.4x
variant, eval only                13598 ns     2304 ns    5.9x

8 threads, eval + impression + count:
  ctypes  p50   135,145 ns   p99   4,943,445 ns       12,310 ops/s
  PyO3    p50     1,162 ns   p99       3,817 ns      540,580 ops/s
```




