#include <math.h>
#include <stdio.h>
#include <limits.h>
#include <stdlib.h>
#include <stdint.h>
#include <unistd.h>
#include <pthread.h>
#include <stdbool.h>
#include <string.h>
#include <sys/time.h>
#include "bits.h"
#include "cpumap.h"
#include "benchmark.h"

#ifndef NUM_ITERS
#define NUM_ITERS 5
#endif

#ifndef MAX_PROCS
#define MAX_PROCS 512
#endif

#ifndef MAX_ITERS
#define MAX_ITERS 20
#endif

#ifndef COV_THRESHOLD
#define COV_THRESHOLD 0.02
#endif

static pthread_barrier_t barrier;
static double times[MAX_ITERS];
static double means[MAX_ITERS];
static double covs[MAX_ITERS];
static volatile int target;

// Ugly global to store even_only and congestion_factor
static bool even_only = false;
static float congestion_factor = 0.0;

static void * thread(void * bits)
{
  int id = bits_hi(bits);
  int nprocs = bits_lo(bits);

  cpu_set_t set;
  CPU_ZERO(&set);

  int cpu;
  if (even_only) {
    int cpu = cpumap(id*2, nprocs);
  } else {
    int cpu = cpumap(id, nprocs);
  }
  CPU_SET(cpu, &set);
  sched_setaffinity(0, sizeof(set), &set);

  thread_init(id, nprocs);
  pthread_barrier_wait(&barrier);

  void * result = NULL;

  result = benchmark(id, nprocs, congestion_factor);
  pthread_barrier_wait(&barrier);
  thread_exit(id, nprocs);
  return result;
}

int main(int argc, const char *argv[])
{
  int nprocs = 0;
  int n = 0;

  /** The first argument is nprocs. */
  if (argc > 1) {
    nprocs = atoi(argv[1]);
  }

  /**
   * Use the number of processors online as nprocs if it is not
   * specified.
   */
  if (nprocs == 0) {
    nprocs = sysconf(_SC_NPROCESSORS_ONLN);
  }

  if (nprocs <= 0) return 1;
  else {
    /** Set concurrency level. */
    pthread_setconcurrency(nprocs);
  }

  /**
   * The second argument is input size n.
   */
  if (argc > 2) {
    n = atoi(argv[2]);
  }

  /**
   * The third argument is if only even cores should be used
  */
  if (argc > 3) {
    if (strcmp(argv[3], "true")) {
      even_only = true;
    }
  }

  /*
   * Fourth argument is the congestion factor, 0.0 means always delay, and 1.0 means never delay
   */
  if (argc > 4) {
     congestion_factor = strtof(argv[4], NULL);
  }

  pthread_barrier_init(&barrier, NULL, nprocs);
  printf("===========================================\n");
  printf("  Benchmark: %s\n", argv[0]);
  printf("  Number of processors: %d\n", nprocs);
  printf("  Even only: %s\n", even_only ? "true" : "false");
  printf("  Congestion factor: %f\n", congestion_factor);

  init(nprocs, n);

  pthread_t ths[nprocs];
  void * res[nprocs];

  int i;
  for (i = 1; i < nprocs; i++) {
    pthread_create(&ths[i], NULL, thread, bits_join(i, nprocs));
  }

  res[0] = thread(bits_join(0, nprocs));

  for (i = 1; i < nprocs; i++) {
    pthread_join(ths[i], &res[i]);
  }

  pthread_barrier_destroy(&barrier);
  printf("  Finished! \n");
  return verify(nprocs, res);
}

