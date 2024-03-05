#!/usr/bin/env bash

set -e
cd "$(dirname "$(readlink -f "$0")")"

ARGS=("$@")
if [[ ! " ${ARGS[*]} " =~ " -w " ]]; then
  ARGS+=("-w" "8")
fi

if [[ ! " ${ARGS[*]} " =~ " -r " ]]; then
  ARGS+=("-r" "1024")
fi

TARGET=../build
if [[ -n "$DISABLE_HP" ]]; then
  TARGET=../build-no-hp
fi


mkdir -p results && cd results
../benchmarks.sh "$TARGET" -q "CC.+|(LPR|LCR|LSC|FAA).+/remap" "${ARGS[@]}"
python3 ../postprocess/draw_main.py
