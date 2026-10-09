# pyO3 benchmark

Compares `is_enabled` accross 3 python engines
1. current solution
2. reporter's POC
3. my POC

instead of covering all fns, only `is_enabled` is handled, but tries to do everything the current solution does, so that the benchmark numbers are not overly optimistic

the reporter's POC makes some simplifications, these are added here:


## Run

Run from the repository root:

```sh
docker build -f python-engine/benchmarks/Dockerfile -t pyo3-bench .
docker run --rm pyo3-bench
```

the build compiles all three
the benchmarks are for both threaded and single thread just like the reporters benchmark

Options: 

`CALLS`: number of calls per test type (engine x flag x context)

`docker run --rm -e CALLS=20000 pyo3-bench`

`THREADS`

`docker run --rm -e THREADS=2 pyo3-bench`

`PER_THREAD`

`docker run --rm -e PER_THREAD=5000 pyo3-bench`


##Output

testing with 3 flags and 3 contexts

## Results

Ran the benchmark a couple times, mostly similar results

```
python 3.13.16, 50,000 calls per cell             
                     
is_enabled, single thread (ns per call)   
flag     context     current   reporter   pyo3_poc  pyo3_poc vs current
-----------------------------------------------------------------------
simple   minimal      11,740        576      1,163                10.1x
simple   standard     13,644        945      1,721                 7.9x
simple   complex      14,954      1,314      2,243                 6.7x
rollout  minimal      12,323        685      1,243                 9.9x
rollout  standard     13,718      1,079      1,850                 7.4x
rollout  complex      15,330      1,491      2,422                 6.3x
complex  minimal      12,366        660      1,247                 9.9x
complex  standard     13,592      1,076      1,828                 7.4x
complex  complex      15,347      1,602      2,576                 6.0x

is_enabled, 8 threads, rollout flag + standard context
  current   p50    157,828 ns   p99    1,147,676 ns       34,557 ops/s
  reporter  p50      1,152 ns   p99        4,068 ns      517,917 ops/s
  pyo3_poc  p50      1,883 ns   p99        6,663 ns      368,990 ops/s

```



### With `pythonize`

```
python 3.13.16, 50,000 calls per cell
current:  /src/ygg/python-engine/yggdrasil_engine/__init__.py
reporter: /usr/local/lib/python3.13/site-packages/ct_unleash_engine/__init__.py
pyo3_poc: /usr/local/lib/python3.13/site-packages/yggdrasil_native/__init__.py
load avg (1.06103515625, 0.978515625, 1.29296875)

is_enabled, single thread (ns per call)
flag     context     current   reporter   pyo3_poc  pyo3_pythonize  pyo3_poc vs current  pyo3_pythonize vs current
------------------------------------------------------------------------------------------------------------------
simple   minimal      12,872        573      1,131           1,346                11.4x                       9.6x
simple   standard     13,100        933      1,695           3,117                 7.7x                       4.2x
simple   complex      14,578      1,306      2,198           3,763                 6.6x                       3.9x
rollout  minimal      11,561        653      1,221           1,399                 9.5x                       8.3x
rollout  standard     13,115      1,075      1,803           2,857                 7.3x                       4.6x
rollout  complex      15,001      1,461      2,339           3,870                 6.4x                       3.9x
complex  minimal      11,482        675      1,252           1,397                 9.2x                       8.2x
complex  standard     13,543      1,067      1,805           2,892                 7.5x                       4.7x
complex  complex      14,882      1,613      2,476           4,044                 6.0x                       3.7x

is_enabled, 8 threads, rollout flag + standard context
  current         p50    150,965 ns   p99      620,039 ns       45,358 ops/s
  reporter        p50      1,183 ns   p99        4,168 ns      487,508 ops/s
  pyo3_poc        p50      1,854 ns   p99        5,731 ns      396,336 ops/s
  pyo3_pythonize  p50      3,026 ns   p99        9,458 ns      254,740 ops/s
```