#!/bin/bash

# Create temporary directory to store intermediate hyperfine results
source "${SCRIPT_DIR}/setup_temp_dir.sh"

# Multipliers
producer_multiplier="$1" 
consumer_multiplier="$2"

# Number of iterations for the ratio
iterations="$3"

# Resulting json file
merged_file_name="$4"

# Amount of congestion
congestion="$5"

for ((i=1; i<=iterations; i++)); do
  producers=$((i * producer_multiplier))
  consumers=$((i * consumer_multiplier))

  echo "Running benchmark with $producers producers and $consumers consumers"

  hyperfine "$BINARY $producers $consumers $LOGN $EVEN_CORES $congestion" --export-json "$temp_dir/result$i.json"

  threads=$((producers + consumers))
  # Add the "parameter" field to the JSON
  python3 "${SCRIPT_DIR}"/add_params.py "$temp_dir/result$i.json" $threads "Threads"
done

# Merge the individual hyperfine commands into single JSON
python3 "${SCRIPT_DIR}"/merge_ratios.py "$temp_dir/*.json" "$temp_dir/$merged_file_name"

# Moves the resulting JSON to the result directory
mv "$temp_dir/$merged_file_name" "${RESULT_DIR}/"
