#include <CLI/App.hpp>
#include <CLI/Formatter.hpp>
#include <CLI/Config.hpp>

#include <string>


int main(int argc, char *argv[]){
    CLI::App app{"Queue benchmarks"};

    int numThreads = 8;
    app.add_option("numThreads", numThreads, "Number of threads")
        ->check(CLI::PositiveNumber);

    int numOps = 7;
    app.add_option("numOps", numOps, "Number of operations, 10^numOps")
        ->check(CLI::PositiveNumber);

    bool evenCores = false;
    app.add_option("evenCores", evenCores, "If true, use only even numbered cores");
    CLI11_PARSE(app, argc, argv);


    return 0;
}

