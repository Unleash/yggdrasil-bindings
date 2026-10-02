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
