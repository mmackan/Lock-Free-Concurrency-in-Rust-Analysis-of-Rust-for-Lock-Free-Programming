import json
import sys

def main():

    # Read the arguments
    filename = sys.argv[1]
    amount = sys.argv[2]
    field = sys.argv[3]

    # Open JSON file to append parameter to
    with open(filename, 'r') as file:
        data = json.load(file)

    # Add the parameter
    data['results'][0]['parameters'] = {
        field: amount
    }

    # Write the modified JSON data back to the file
    with open(filename, 'w') as file:
        json.dump(data, file, indent=4)

if __name__ == "__main__":
    main()
