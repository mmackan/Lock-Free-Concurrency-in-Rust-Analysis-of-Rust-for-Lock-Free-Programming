#!/usr/bin/env bash

PC_BENCH="$1/bench-prod-cons"
ED_BENCH="$1/bench-enq-deq"

shift

{
  "$PC_BENCH" --fork -f cpp-res-rs-p1c1b.csv -t 8 8 24 24 -r 32 64 256 512 1024 2048 4096 8192 -b "$@"
  "$PC_BENCH" --fork -f cpp-res-rs-p2c1b.csv -t 10 5 32 16 -r 32 64 256 512 1024 2048 4096 8192 -b "$@"
  "$PC_BENCH" --fork -f cpp-res-rs-p1c2b.csv -t 5 10 16 32 -r 32 64 256 512 1024 2048 4096 8192 -b "$@"
  "$ED_BENCH" --fork -f cpp-res-rs-enq-deq.csv -t 16 48 -r 32 64 256 512 1024 2048 4096 8192 "$@"
}
