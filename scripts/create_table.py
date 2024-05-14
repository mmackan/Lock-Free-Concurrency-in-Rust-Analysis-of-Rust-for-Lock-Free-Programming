import os
import re
import argparse
import csv
from pathlib import Path
import json
from collections import defaultdict

# The relevant metrics for report
METRICS = [
    'cache-misses', 
    'cache-references', 
    'faults', 
    'energy (J)', 
    'time (s)', 
    'heap total',
    'heap peak', 
    'stack peak',
    # 'instructions', 
    # 'branches', 
    # 'cycles'
    ] 

# All the implementations
IMPLEMENTATIONS = {
    'arc': 'Aarc',
    'epoch': 'Epoch',
    'rust': 'Hazp',
    # 'leak': 'Leak',
    'c': 'Cpp'
}

BENCHMARKS = {
    'perf_mem': 'perf_mem',
    'memusage': 'memusage',
    'energy': 'energy'
}

def contains_impl(input_string):
    return any(key in input_string for key in IMPLEMENTATIONS.keys())

def get_files(dir, pattern):
    files = []
    
    for file_name in os.listdir(dir):        
        if file_name.endswith(pattern) and contains_impl(file_name):
            file_path = os.path.join(dir, file_name)
            files.append(file_path)
    
    return files

# Process text file from memory usage results
def process_perf_memory(file_path):
    results = {}
    keywords = [
        'cache-references', 'cache-misses', 'cycles', 'instructions',
        'branches', 'faults', 'migrations'
    ]
    with open(file_path, 'r') as file:
        for line in file:
            for keyword in keywords:
                if keyword in line:
                    value = int(line.strip().split()[0])
                    results[keyword] = {'value': value}
    return results

# Process text file from energy consumption results
def process_energy(file_path):
    with open(file_path, 'r') as file:
        for line in file:
            if 'Joules' in line:
                # Extract the Joules
                joules = float(re.findall(r'\d+\.\d+', line)[0])
                return {'energy (J)': {'value': joules}}
    return {}

def process_memusage(file_path):
    pattern = re.compile(r'heap total: (\d+), heap peak: (\d+), stack peak: (\d+)')

    with open(file_path, 'r') as file:
        for line in file:
            match = re.search(pattern, line)
            if match:
                ht, hp, sp = match.groups()
                return {
                    'heap total': {'value': int(ht)},
                    'heap peak': {'value': int(hp)},
                    'stack peak': {'value': int(sp)}
                }
    return {}

# Process JSON file from throughput benchmark results
def process_time_file(file_path):
    with open(file_path, 'r') as file:
        data = json.load(file)

        max_threads = 0

        # Take time from the iteration with max amount of threads
        for iteration in data["results"]:
            threads = int(iteration['parameters']['Threads'])
            if threads > max_threads:
                max_threads = threads
                max_threads_mean = iteration['mean']

    return {'time (s)': {'value': round(max_threads_mean, 2)}}

def process_files(dir):
    data = defaultdict(lambda: defaultdict(lambda: defaultdict(lambda: defaultdict(dict))))

    # Energy and Memory benchmark files
    files = get_files(dir, '.txt')

    # Files that contain mean time for ratio benchmarks
    files.extend(get_files(dir, 'pc_1_1.json'))
    files.extend(get_files(dir, 'pc_2_1.json'))

    for file_path in files:
        file_name = os.path.basename(file_path)
        file_type = 'TXT' if '.txt' in file_name else 'JSON'
        ratio = '1_1' if '1_1' in file_name else '2_1'

        implementation = ''
        benchmark = ''

        # Determine implementation from file name
        for key, impl in IMPLEMENTATIONS.items():
            if key in file_name:
                implementation = impl
                break
        
        # Determine benchmark from file name
        for key, bm in BENCHMARKS.items():
            if key in file_name:
                benchmark = bm
                break

        # Process the file depending on what benchmark
        if benchmark == 'perf_mem':
            mem_data = process_perf_memory(file_path)
            data[ratio][implementation].update(mem_data)
        elif benchmark == 'energy':
            energy_data = process_energy(file_path)
            data[ratio][implementation].update(energy_data)
        elif benchmark == 'memusage':
            mem_data = process_memusage(file_path)
            data[ratio][implementation].update(mem_data)
        elif file_type == 'JSON':
            time_data = process_time_file(file_path)
            data[ratio][implementation].update(time_data)
    
    normalize_data(data)
    
    return data

def find_minimums(data):
    min_values = {}
    for ratio in data:
        min_values[ratio] = {}
        for implementation in data[ratio]:
            for metric, values in data[ratio][implementation].items():

                if metric not in min_values[ratio]:
                    min_values[ratio][metric] = float('inf')

                # Something is wrong with benchmark
                if (values['value'] == 0):
                    values['value'] = float('inf')
                
                min_values[ratio][metric] = min(min_values[ratio][metric], values['value'])
    return min_values

# Normalize each metric to the most efficient implementation
def normalize_data(data):
    min_values = find_minimums(data)
    for ratio in data:
        for implementation in data[ratio]:
            for metric, values in data[ratio][implementation].items():
                normalized_value = values['value'] / min_values[ratio][metric]
                values['normalized'] = round(normalized_value, 2)

def print_data(data):
    for ratio in data:
        print(f'Ratio: {ratio}')
        for implementation in data[ratio]:
            print(f'  Implementation: {implementation}')
            for metric, values in data[ratio][implementation].items():
                print(f'      {metric}: {values}')

def create_csv(data, value, dir):
    for ratio in data:
        file_path = f'{dir}/ratio_{ratio}_{value}.csv'

        fields = ['Implementation'] + list(data[ratio]['Hazp'].keys())

        with open(file_path, 'w', newline='') as csvfile:
            writer = csv.DictWriter(csvfile, fieldnames=fields)
            writer.writeheader()

            for implementation in data[ratio]:
                row = {'Implementation': implementation}
                for metric in data[ratio][implementation]:
                    row[metric] = data[ratio][implementation][metric][value]

                writer.writerow(row)

        if value == 'normalized':
            split_and_sort_csv(ratio, file_path, f'{dir}/tables')

def split_and_sort_csv(ratio, file_path, output_dir):
    with open(file_path, mode='r', newline='') as file:
        reader = csv.DictReader(file)

        # Dictionary to hold a lists for each metric
        metric_data = {metric: [] for metric in METRICS}

        for row in reader:
            for metric in metric_data:
                metric_data[metric].append((row['Implementation'], row[metric]))

        # Create an output directory if it doesn't exist
        Path(output_dir).mkdir(parents=True, exist_ok=True)

        # For each metric, sort by the metric's value and write to a new CSV file
        for metric, tuples in metric_data.items():
            # Sort normalized values with 1.0 the top
            sorted_tuples = sorted(tuples, key=lambda x: (float(x[1]), x[0]) if x[1] != '1.0' else (0, x[0]))
            
            metric_csv_path = Path(output_dir) / f'{ratio}_{metric}.csv'

            with open(metric_csv_path, mode='w', newline='') as outfile:
                writer = csv.writer(outfile)
                writer.writerow([f'{ratio.replace("_", ":")}', metric])
                writer.writerows(sorted_tuples)

def main():
    parser = argparse.ArgumentParser(description='Process memory and energy files from results.')
    parser.add_argument('directory', type=str, help='The directory containing the becnhmarking results.')
    args = parser.parse_args()

    data = process_files(args.directory)

    print_data(data)

    # Tables for the Appendix
    create_csv(data, 'value', args.directory)

    # Tables for the report
    create_csv(data, 'normalized', args.directory)

if __name__ == "__main__":
    main()