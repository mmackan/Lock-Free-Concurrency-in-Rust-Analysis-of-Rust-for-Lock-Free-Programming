#include <CLI/App.hpp>
#include <CLI/Formatter.hpp>
#include <CLI/Config.hpp>

#include <string>
#include <cmath>
#include <thread>
#include <vector>
#include <random>

#include <pthread.h>

#include "LPRQueue.hpp"

void delay_exec(std::mt19937& gen) {
    auto distibution = std::uniform_int_distribution<int>(0, 100);
    int n = distibution(gen);
    int delay_end = 50 + n;

    for (int i = 50; i < delay_end; i++) {
        asm volatile("nop");
    }
    
}

int main(int argc, char *argv[]){
    CLI::App app{"Pairwise LPRQ benchmarks"};

    int numThreads = 8;
    app.add_option("numThreads", numThreads, "Number of threads")
        ->check(CLI::PositiveNumber);

    int numOps = 7;
    app.add_option("numOps", numOps, "Number of operations, 10^numOps")
        ->check(CLI::PositiveNumber);

    bool evenCores = false;
    app.add_option("evenCores", evenCores, "If true, use only even numbered cores");
    CLI11_PARSE(app, argc, argv);

    int nops = pow(10, numOps);
    int tops = nops / numThreads;

    std::vector<std::thread> handles(numThreads);

    // Initialize the queue
    auto queue = new LPRQueue<int, false, 1024, true>(numThreads);

    int core = 0;
    for (int i = 0; i < numThreads; i++) {
        handles[i] = std::thread([&tops, &nops, i, &queue, core](){
            // Thread rng
            auto engine = std::mt19937(std::random_device{}());

            // Cpu affinity
            cpu_set_t cpuset;
            CPU_ZERO(&cpuset);
            CPU_SET(core, &cpuset);
            int result = pthread_setaffinity_np(pthread_self(), sizeof(cpu_set_t), &cpuset);

            if (result != 0) {
                std::cerr << "Failed to set affinity to cpu  " << i << std::endl;
            }

            for (int j = 0; j < tops; j++) {
                queue->enqueue(&j, i);
                delay_exec(engine);
                queue->dequeue(i);
                delay_exec(engine);
            }
        });
        core++;
        if (evenCores) {
            core++;
        }
    }

    for (auto& handle : handles) {
        handle.join();
    }


    return 0;
}
