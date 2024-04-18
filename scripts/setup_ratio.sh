#!/bin/bash

FACTOR=$1
RATIO=$2

max_threads=$(($(nproc) / $FACTOR))

# Extract the multipliers from the ratio
IFS=':' read producer_multiplier consumer_multiplier <<< "$RATIO"

# Calculate maximum amount of producers & consumers for the ratio
min_threads=$((producer_multiplier + consumer_multiplier))
base=$(($max_threads / $min_threads))

producers=$(($base * producer_multiplier))
consumers=$(($base * consumer_multiplier))

export PROD=$producers
export CONS=$consumers



