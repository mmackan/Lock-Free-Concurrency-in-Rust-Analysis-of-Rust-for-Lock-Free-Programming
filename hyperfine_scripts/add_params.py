import json
import sys

def main():

    # Read the arguments
    filename = sys.argv[1]
    producer_threads = int(sys.argv[2])
    consumer_threads = int(sys.argv[3])

    # Open JSON file to append parameter to
    with open(filename, 'r') as file:
        data = json.load(file)

    # Add the parameter
    data['results'][0]['parameters'] = {
        'Threads': producer_threads + consumer_threads
    }

    # Write the modified JSON data back to the file
    with open(filename, 'w') as file:
        json.dump(data, file, indent=4)

if __name__ == "__main__":
    main()
