#include <CLI/App.hpp>
#include <CLI/Formatter.hpp>
#include <CLI/Config.hpp>

#include <string>
#include <cmath>
#include <thread>
#include <vector>
#include <random>
#include <atomic>

#include <pthread.h>

#include "LPRQueue.hpp"

void delay_exec() {

    for (int i = 0; i < 100; i++) {
        asm volatile("nop");
    }
    
}

int main(int argc, char *argv[]){
    CLI::App app{"MPMC LPRQ benchmarks"};

    int numProducers = 8;
    app.add_option("numProducers", numProducers, "Number of producer threads")
        ->check(CLI::PositiveNumber);

    int numConsumers = 8;
    app.add_option("numComsumers", numConsumers, "Number of consumer threads")
        ->check(CLI::PositiveNumber);

    int numOps = 7;
    app.add_option("numOps", numOps, "Number of operations, 10^numOps")
        ->check(CLI::PositiveNumber);

    bool evenCores = false;
    app.add_option("evenCores", evenCores, "If true, use only even numbered cores");
    float congestion_factor = 1.0;
    app.add_option("congestion_factor", congestion_factor, "Congestion factor, 0.0-1.0, 1.0 meaning full congestion");
    CLI11_PARSE(app, argc, argv);

    int nops = pow(10, numOps);
    int tops = nops / numProducers;

    std::vector<std::thread> producer_handles(numProducers);
    std::vector<std::thread> consumer_handles(numConsumers);

    // Initialize the queue
    auto queue = new LPRQueue<int, false, 1024, true>(numProducers + numConsumers); 

    int core = 0;

    for (int i = 0; i < numProducers; i++) {
        producer_handles[i] = std::thread([&tops, &nops, i, &queue, core, &congestion_factor](){
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
                queue->enqueue(&j, core);
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
    std::atomic<bool> done = false;
    for (int i = 0; i < numConsumers; i++) {
        consumer_handles[i] = std::thread([i, &queue, core, &done, &congestion_factor](){
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
            while (true) {
                auto val = queue->dequeue(core);
                if (val == nullptr) {
                    if (done.load() == true) {
                        break;
                    }
                }
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

    for (auto& handle : producer_handles) {
        handle.join();
    }

    done.store(true);

    for (auto& handle : consumer_handles) {
        handle.join();
    }


    return 0;
}
