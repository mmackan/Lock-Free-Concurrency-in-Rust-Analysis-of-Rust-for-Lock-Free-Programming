#!/bin/bash

# Create temporary directory to store intermediate hyperfine results
source "${SCRIPT_DIR}/setup_temp_dir.sh"

workload="$1"
merged_file_name="$2"

# Makes sure intermediate files are ordered correctly
index=1

handle_pairwise() {
    local c="$1"
    local i="$2"

    echo "Running pairwise benchmark with congestion level: $c"
    
    hyperfine "$BINARY $THREADS $LOGN $EVEN_CORES $c" --export-json "$temp_dir/result$i.json"

    # Add the "parameter" field to the JSON
    python3 "${SCRIPT_DIR}"/add_params.py "$temp_dir/result$i.json" $c "Congestion"
}

handle_mpmc() {
    local c="$1"
    local i="$2"
    local ratio="$3"

    # Temporary fix for 2:1 ratio benchmarks until bug is fixed
    if [[ $ratio == "2:1" ]]; then
        producers="20"
        consumers="10"

    # Calculate maximum amount of producers & consumers for the ratio
    else
        max_threads=$(($(nproc) / $FACTOR))

        # Extract the multipliers from the ratio
        IFS=':' read producer_multiplier consumer_multiplier <<< "$ratio"

        min_threads=$((producer_multiplier + consumer_multiplier))
        base=$(($max_threads / $min_threads))

        producers=$(($base * producer_multiplier))
        consumers=$(($base * consumer_multiplier))
    fi 

    echo "Running mpmc $ratio benchmark with congestion level: $c"

    hyperfine "$BINARY $producers $consumers $LOGN $EVEN_CORES $c" --export-json "$temp_dir/result$i.json"

    # Add the "parameter" field to the JSON
    python3 "${SCRIPT_DIR}"/add_params.py "$temp_dir/result$i.json" $c "Congestion"

}

for congestion in 0.0 0.25 0.5 1.0; do
    if [[ "$workload" == "pairwise" ]]; then
        handle_pairwise "$congestion" "$index"
    elif [[ "$workload" =~ ^mpmc ]]; then
        # Extract the ratio from string
        ratio="${workload#mpmc-}"
        handle_mpmc "$congestion" "$index" "$ratio" 
    else
        echo "Invalid workload type specified"
    fi
    ((index++))
done

# Merge the individual hyperfine commands into single JSON
python3 "${SCRIPT_DIR}"/merge_ratios.py "$temp_dir/*.json" "$temp_dir/$merged_file_name"

# Moves the resulting JSON to the result directory
mv "$temp_dir/$merged_file_name" "${RESULT_DIR}/"