#!/bin/bash

# Create temporary directory to store intermediate hyperfine results
source "${SCRIPT_DIR}/setup_temp_dir.sh"

workload="$1"
merged_file_name="$2"


## TODO: For both workloads use LOGN operations and scan over [0.0, 0.25, 1.0]

## TODO: If pairwise -> Use max threads

## TODO: If MPMC workload -> Check if 1:1 or 2:1 ratio, then use max p:c slit for corresponding ratio