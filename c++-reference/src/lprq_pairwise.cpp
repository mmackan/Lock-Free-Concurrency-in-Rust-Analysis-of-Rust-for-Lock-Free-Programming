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

void delay_exec() {
    for (int i = 0; i < 100; i++) {
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

    float congestion_factor = 0.0;
    app.add_option("congestion_factor", congestion_factor, "Congestion factor, 0.0-1.0, 1.0 meaning full congestion");

    CLI11_PARSE(app, argc, argv);

    int nops = pow(10, numOps);
    int tops = nops / numThreads;

    std::vector<std::thread> handles(numThreads);

    // Initialize the queue
    auto queue = new LPRQueue<int, false, 1024, true>(numThreads);

    int core = 0;
    for (int i = 0; i < numThreads; i++) {
        handles[i] = std::thread([&tops, &nops, i, &queue, core, &congestion_factor](){
            // Thread rng
            auto engine = std::mt19937(std::random_device{}());
            auto distribution = std::uniform_real_distribution<float>(0.0,1.0);

            // Cpu affinity
            cpu_set_t cpuset;
            CPU_ZERO(&cpuset);
            CPU_SET(core, &cpuset);
            int result = pthread_setaffinity_np(pthread_self(), sizeof(cpu_set_t), &cpuset);

            if (result != 0) {
                std::cerr << "Failed to set affinity to cpu  " << i << std::endl;
            }

            for (int j = 0; j < tops; j++) {
                int* val = new int;
                *val = j;
                queue->enqueue(val, i);
                if(distribution(engine) > congestion_factor){
                    delay_exec();
                }
                delete (queue->dequeue(i));
                if(distribution(engine) > congestion_factor){
                    delay_exec();
                }
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
