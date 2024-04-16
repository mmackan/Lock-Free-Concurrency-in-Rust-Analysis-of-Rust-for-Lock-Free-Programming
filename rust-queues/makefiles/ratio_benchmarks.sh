#!/bin/bash

# Directory of the script
dir="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"

# Temporary directory within $dir to store intermediate hyperfine results
temp_dir=`mktemp -d -p "$dir"`

# Check if temp dir was created
if [[ ! "$temp_dir" || ! -d "$temp_dir" ]]; then
  echo "Could not create temp dir"
  exit 1
fi

# Deletes the temp directory
function cleanup {      
  rm -rf "$temp_dir"
  echo "Deleted temp working directory $temp_dir"
}

# Register the cleanup function to be called on the EXIT signal
trap cleanup EXIT

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

    # Temporary fix for 2:1 ratio benchmarks until bug is fixed
    if [[ $ratio == "2:1" ]] && [[ $producers -gt 20 ]]; then
      break
    fi  

    echo "Running benchmark with $producers producers and $consumers consumers"

    hyperfine "$BINARY $producers $consumers $LOGN $EVEN_CORES $congestion" --export-json "$temp_dir/result$i.json"

    # Add the "parameter" field to the JSON
    python3 "${SCRIPT_DIR}"/add_params.py "$temp_dir/result$i.json" $producers $consumers $LOGN
done

# Merge the individual hyperfine commands into single JSON
python3 "${SCRIPT_DIR}"/merge_ratios.py "$temp_dir/*.json" "$temp_dir/$merged_file_name"

# Moves the resulting JSON to the result directory
mv "$temp_dir/$merged_file_name" "${RESULT_DIR}/"
