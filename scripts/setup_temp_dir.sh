
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