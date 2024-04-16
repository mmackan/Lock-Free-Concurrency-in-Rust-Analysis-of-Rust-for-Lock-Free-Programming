import json
import glob
import sys

def main():

    # Read the arguments
    json_results = sys.argv[1]
    output_file = sys.argv[2]
    
    merged_results = {"results": []}

    # Guarantee files will be merged in same order as their name
    sorted_files = sorted(
        glob.glob(json_results), 
        key=lambda x: int(x.split('result')[-1].split('.')[0]))

    # Merge results from each ratio's benchmark
    for json_file in sorted_files:
        with open(json_file, 'r') as file:
            data = json.load(file)
            merged_results["results"].extend(data["results"])

    # Write to output file
    with open(output_file, 'w') as file:
        json.dump(merged_results, file, indent=4)

if __name__ == "__main__":
    main()
