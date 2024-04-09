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

# Ratio in the format of "producer:consumer"
ratio="$1" 

# Number of iterations for the ratio
iterations="$2" 

result_dir="${RESULT_DIR}"
script_dir="${SCRIPT_DIR}"
merged_file_name="${JSON}"

# Extract the multipliers from the ratio
IFS=':' read producer_multiplier consumer_multiplier <<< "$ratio"

for ((i=1; i<=iterations; i++)); do
    producer_threads=$((i * producer_multiplier))
    consumer_threads=$((i * consumer_multiplier))

    echo "Running benchmark with $producer_threads producers and $consumer_threads consumers"

    hyperfine "$BINARY $producer_threads $consumer_threads $LOGN $EVEN_CORES" --export-json "$temp_dir/result$i.json"

    # Add the parameter field in the JSON
    python3 "$script_dir"/add_params.py "$temp_dir/result$i.json" $producer_threads $consumer_threads $LOGN
done

# Merge all the results from the individual hyperfine commands
python3 "$script_dir"/merge_ratios.py "$temp_dir/*.json" "$temp_dir/$merged_file_name"

# Ensure result directory exist, and 
mkdir -p "$result_dir" 

# Moves the final results to the result directory
mv "$temp_dir/$merged_file_name" "$result_dir/"
