import json
import matplotlib.pyplot as plt
import statistics
import argparse
import sys
import numpy as np

# Ugly parameter for fixed number of logn operations, should be argument
LOG_OPS = 8

# Values are not greater than this, used for correct y-scale
Y_MAX = 0.2*(10**LOG_OPS)

# Used for scaling x-axis, perhpas for future use
# MAX_THREADS = 36

def calculate_throughput(json_file, scan_category):
    with open(json_file, 'r') as file:
        benchmark = json.load(file)
        
    throughputs = []
    stddevs = []

    # Fixed operations for thread scan
    if scan_category in ["Threads", "Congestion"]:
        operations = (10**LOG_OPS)  

    for entry in benchmark['results']:

        # This is old code, kept in case we need throughput for operations also
        # Operations vary for each entry in operation scan
        if scan_category == "Operations":
            log_ops = int(entry['parameters']['Operations'])
            operations = (10**log_ops)

        # Each iteration's runtime
        times = entry['times']

        # Average runtime
        mean = statistics.mean(times)

        # Each iteration's throughput (Operations/s)
        tp = [operations / x for x in times]

        # Mean throughput 
        mean_throughput = statistics.mean(tp)

        # Standard deviation throughput
        throughput_stddev = statistics.stdev(tp)

        throughputs.append(mean_throughput)
        stddevs.append(throughput_stddev)
    
    return throughputs, stddevs

def die(msg):
    sys.stderr.write("fatal: %s\n" % (msg,))
    sys.exit(1)

def extract_parameters(results):
    """Return `(scan_category: str, parameter_values: List[float])`."""
    if not results:
        die("no benchmark data to plot")
    (names, values) = zip(*(unique_parameter(b) for b in results))
    names = frozenset(names)
    if len(names) != 1:
        die(
            "benchmarks must all have the same parameter name, but found: %s"
            % sorted(names)
        )
    return (next(iter(names)), list(values))

def unique_parameter(benchmark):
    """Return the unique parameter `(name: str, value: float)`, or die."""
    params_dict = benchmark.get("parameters", {})
    if not params_dict:
        die("benchmarks must have exactly one parameter, but found none")
    if len(params_dict) > 1:
        die(
            "benchmarks must have exactly one parameter, but found multiple: %s"
            % sorted(params_dict)
        )
    [(name, value)] = params_dict.items()
    return (name, float(value))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "file", help="JSON file with benchmark results", nargs="+"
    )
    parser.add_argument(
        "--parameter-name",
        metavar="name",
        type=str,
        help="Deprecated; parameter names are now inferred from benchmark files",
    )

    parser.add_argument(
        "--log-x", help="Use a logarithmic x (parameter) axis", action="store_true"
    )

    parser.add_argument(
        "--log-time", help="Use a logarithmic time axis", action="store_true"
    )

    parser.add_argument(
        "--titles", help="Comma-separated list of titles for the plot legend"
    )

    parser.add_argument(
        "-o", "--output", help="Save image to the given filename."
    )
    # parser.add_argument(
    #     "--threads", help="Specify if operation scan, otherwise operation scan"
    # )

    args = parser.parse_args()
    if args.parameter_name is not None:
        sys.stderr.write(
            "warning: --parameter-name is deprecated; names are inferred from "
            "benchmark results\n"
        )

    parameter_name = None

    for filename in args.file:
        with open(filename) as f:
            results = json.load(f)["results"]

        (this_parameter_name, parameter_values) = extract_parameters(results)
        if parameter_name is not None and this_parameter_name != parameter_name:
            die(
                "files must all have the same parameter name, but found %r vs. %r"
                % (parameter_name, this_parameter_name)
            )
        parameter_name = this_parameter_name

        if parameter_name in ["Threads", "Congestion"]:

            # Convert to throughput instead of runtime
            mean, stddev = calculate_throughput(filename, parameter_name)
        else:
            mean = [b["mean"] for b in results]
            stddev = [b["stddev"] for b in results]

        # Set different styles for each language
        if "rust" in filename:
            color = 'orange'
            fmt = '-o'
        else:
            color = 'blue'
            fmt = '-v'

        plt.errorbar(
            x=parameter_values, 
            y=mean, 
            yerr=stddev, 
            color = color,
            fmt = fmt,
            elinewidth=1,
            capsize=2,
            markersize=5,
            )
    
    plt.xlabel(parameter_name)
    plt.grid(True)
    plt.ylabel('Throughput [Operations/s]')

    if args.log_time:
        plt.yscale("log")
        plt.ylabel("Time [s]")
    else:
        # Make y-scale same for all plots
        plt.ylim(0, Y_MAX)

        # Scaling options for the x-axis
        # x = np.arange(0, MAX_THREADS + 1)
        # plt.xticks(np.arange(min(x), max(x)+1, 2))
        # plt.xlim(0, MAX_THREADS + 1)

    if args.log_x:
        plt.xscale("log")
    
    if args.titles:
        languages = args.titles.split(",")
        title = f"Comparison between {languages[0]} and {languages[1]}"
        plt.legend(languages)
        plt.title(title)

    if args.output:
        plt.savefig(args.output)
    else:
        plt.show()

if __name__ == "__main__":
    main()