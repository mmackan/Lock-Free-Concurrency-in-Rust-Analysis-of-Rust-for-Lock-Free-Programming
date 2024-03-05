#!/usr/bin/env bash

PC_BENCH="$1/bench-prod-cons"
ED_BENCH="$1/bench-enq-deq"

if [[ -z "$MAX_THREADS" ]]; then
  MAX_THREADS=`nproc --all`
fi

deduce_nproc_pc() {
  SCRIPT="$(dirname "$(readlink -f "$0")")/deduce_nproc.py"
  python3 "$SCRIPT" pc $MAX_THREADS
}
deduce_nproc_sym() {
  SCRIPT="$(dirname "$(readlink -f "$0")")/deduce_nproc.py"
  python3 "$SCRIPT" sym $MAX_THREADS
}

shift

"$PC_BENCH" --fork -f cpp-res-p1c1b.csv -t $(echo 1 1 2 2 4 4 8 8 12 12 16 16 24 24 32 32 48 48 64 64 128 128 | deduce_nproc_pc) -b "$@"
"$PC_BENCH" --fork -f cpp-res-p2c1b.csv -t $(echo 2 1 4 2 8 4 12 6 16 8 20 10 32 16 42 21 64 32 84 42 144 72 | deduce_nproc_pc) -b "$@"
"$PC_BENCH" --fork -f cpp-res-p1c2b.csv -t $(echo 1 2 2 4 4 8 6 12 8 16 10 20 16 32 21 42 32 64 42 84 72 144 | deduce_nproc_pc) -b "$@"
"$ED_BENCH" --fork -f cpp-res-enq-deq.csv -t $(echo 1 2 4 8 16 24 32 48 64 96 128 256 | deduce_nproc_sym) "$@"
