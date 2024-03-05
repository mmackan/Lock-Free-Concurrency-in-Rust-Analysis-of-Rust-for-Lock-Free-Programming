#pragma once

extern "C" {
#include <queue.h>
}
#include <Metrics.hpp>


// ring size is not used by this queue
// it's provided for consistency of the benchmarks
template <class T>
class CCQueue : public MetricsAwareBase {
private:
    static constexpr int MAX_THREADS = 128;

    // alignof(handle_t) > sizeof(handle_t), so the compiler disallows arrays of handle_t
    // HandleT fixes this issue
    struct HandleT : public handle_t {
        [[no_unique_address]] char pad[std::max((ssize_t)0, (ssize_t)alignof(handle_t) - (ssize_t)sizeof(handle_t))];
    };

    queue_t queue;
    HandleT handles[MAX_THREADS];
    int maxThreads;

public:
    CCQueue(int maxThreads=MAX_THREADS)
            : MetricsAwareBase(maxThreads), maxThreads(maxThreads) {
        queue_init(&queue, maxThreads);
        if (maxThreads > MAX_THREADS)
            std::abort();
        for (int i = 0; i < maxThreads; ++i) {
            // that's a bit insane
            // because register must be called by the corresponding thread
            queue_register(&queue, &handles[i], i);
        }
    }

    ~CCQueue() {
        queue_free(&queue, &handles[0]);
        for (int i = 0; i < maxThreads; ++i) {
            handle_free(&handles[i]);
        }
    }

    static std::string className() {
        return "CCQueue";
    }

    size_t estimateSize(int tid) {
        return ::queue_size(&queue, static_cast<handle_t*>(&handles[tid]));
    }

    void enqueue(T* item, const int tid) {
        ::enqueue(&queue, static_cast<handle_t*>(&handles[tid]), item);
    }

    T* dequeue(const int tid) {
        void* r = ::dequeue(&queue, static_cast<handle_t*>(&handles[tid]));
        if (r == EMPTY)
            return nullptr;
        return static_cast<T*>(r);
    }
};
