#include <boost/thread/thread.hpp>
#include <boost/lockfree/queue.hpp>
#include <iostream>

#include <boost/atomic.hpp>
#include <boost/program_options.hpp>

boost::atomic_int producer_count(0);
boost::atomic_int consumer_count(0);

boost::lockfree::queue<int> queue(128);

const int iterations = 10000000;

void producer(void)
{
    for (int i = 0; i != iterations; ++i) {
        int value = ++producer_count;
        while (!queue.push(value))
            ;
    }
}

boost::atomic<bool> done (false);
void consumer(void)
{
    int value;
    while (!done) {
        while (queue.pop(value))
            ++consumer_count;
    }

    while (queue.pop(value))
        ++consumer_count;
}

int main(int argc, char* argv[])
{
    namespace po = boost::program_options;
    using namespace std;

    boost::thread_group producer_threads, consumer_threads;

    // Declare the supported options.
    po::options_description desc("Allowed options");
    desc.add_options()
        ("help", "produce help message")
        ("producers,p", po::value<int>(), "# producer threads")
        ("consumers,c", po::value<int>(), "# consumer threads")
    ;

    po::variables_map vm;
    po::store(po::parse_command_line(argc, argv, desc), vm);
    po::notify(vm);    

    if (vm.count("help")) {
        cout << desc << "\n";
        return 1;
    }

    int producer_thread_count = 4;
    int consumer_thread_count = 4;

    if (vm.count("producers")) {
        cout << "Number of producers set to " 
     << vm["producers"].as<int>() << ".\n";
        producer_thread_count = vm["producers"].as<int>();
    } else {
        cout << "Number of producer threads not set, defaulting to 4\n";
    }
    if (vm.count("consumers")) {
        cout << "Number of consumers set to " 
     << vm["consumers"].as<int>() << ".\n";
        consumer_thread_count = vm["consumers"].as<int>();
    } else {
        cout << "Number of consumer threads not set, defaulting to 4\n";
    }

    for (int i = 0; i != producer_thread_count; ++i)
        producer_threads.create_thread(producer);

    for (int i = 0; i != consumer_thread_count; ++i)
        consumer_threads.create_thread(consumer);

    producer_threads.join_all();
    done = true;

    consumer_threads.join_all();

    cout << "produced " << producer_count << " objects." << endl;
    cout << "consumed " << consumer_count << " objects." << endl;
}