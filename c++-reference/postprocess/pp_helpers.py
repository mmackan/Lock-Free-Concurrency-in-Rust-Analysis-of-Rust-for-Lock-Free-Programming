import sys

import matplotlib as mpl
import matplotlib.pyplot as plt
import os.path as path
import postprocess as pp
from contextlib import contextmanager


def plot_results(files, title, benchmark, mapper):
    q_pc = pp.read_thrpt_data(files, benchmark)
    q_pc = mapper(q_pc)
    pp.plot_throughput(q_pc)
    plt.title(title)
    plt.xscale('log')
    pp.xticks(q_pc)

def plot_asymmetric_results(files, title, mapper=lambda x: x):
    plot_results(files, title, "producerConsumer", mapper)

def plot_symmetric_results(files, title, mapper=lambda x: x):
    def f(q):
        q = mapper(q).copy()
        q["Score"] /= 2
        q["Score Error"] /= 2
        return q
    plot_results(files, title, "enqDeqPairs", f)

def plot_metric_results(files, bench, metric, title):
    q_m = pp.read_metrics_data(files, bench, metric, False)
    pp.plot_throughput(q_m)
    plt.title(title)
    plt.xscale('log')
    pp.xticks(q_m)

def filter_work(w, compose=lambda p: p):
    def f(q_pc):
        q_pc = compose(q_pc)
        if "Param: additionalWork" not in q_pc.columns:
            return q_pc
        return q_pc[q_pc["Param: additionalWork"] == w]
    return f

def filter_rs(rs, compose=lambda x: x):
    def f(q):
        q = compose(q)
        return q[q["Param: ringSize"] == rs].drop(["Param: ringSize"], axis="columns")
    return f

def filter_threads(tmin=0, tmax=1024, compose=lambda x: x):
    def f(q):
        q = compose(q)
        t = q["Threads"]
        return q[(t >= tmin) & (t <= tmax)]
    return f

@contextmanager
def paper_throughput_charts():
    rc = dict(mpl.rcParams)
    plt.rc('font', size=13)
    plt.rc('lines', linewidth=2.2)
    pp.ylabel("Throughput, transfers / s")
    pp.disable_legend()
    try:
        yield None
    finally:
        handles, labels = plt.gca().get_legend_handles_labels()
        plt.gcf().legend(handles, labels, loc='upper center', ncol=len(labels), borderaxespad=1.5, fontsize='large')
        plt.subplots_adjust(hspace=0.25)

        pp.enable_legend()
        pp.xscale(None)
        mpl.rcParams.update(rc)

def paper_queues(compose=lambda x: x):
    def f(q):
        q = compose(q)
        s = q["Service Name"]
        return q[s.str.startswith('CC') | (s.str.startswith('FAA') | s.str.startswith('LPRQ') | s.str.startswith('LSCQ') | s.str.startswith('LCRQ')) & s.str.contains('/remap')]
    return f

def get_service_name_for_label_with_remap(service):
    return service.replace("FAAArrayQueue", "faa-queue") \
        .replace("LCRQueue", "lcrq") \
        .replace("LSCQueue", "lscq") \
        .replace("LPRQueue", "lprq") \
        .replace("LFakeCRQueue", "fake-lcrq") \
        .replace("LModCRQueue", "lcrq-modified") \
        .replace("CCQueue", "cc-queue") \
        .replace("MichaelScottQueue", "m&s-queue")

def get_service_name_for_label(service):
    return get_service_name_for_label_with_remap(service) \
        .replace("/remap", "")

def parse_args(supported_options, usage):
    args = sys.argv[1:]
    options = {}
    others = []
    i = 0
    while i < len(args):
        a = args[i]
        if a.startswith("-"):
            if i == len(args) - 1 or a not in supported_options or a in options:
                print(usage)
                exit(1)
            options[a] = args[i + 1]
            i += 1
        else:
            others.append(a)
        i += 1
    return options, others
