import matplotlib.pyplot as plt
import pandas as pd
import postprocess as pp
import pp_helpers as pph
import sys
import os


def get_chart_style(index):
    markers = ['o', 'v', 's', '*', 'D']

    if "Service Name" not in index:
        return None
    queue_type = pph.get_service_name_for_label(index["Service Name"])
    h1 = hash(queue_type + "$1")

    marker = markers[h1 % len(markers)]
    markersize = 8
    linestyle = '-'

    if "cc-queue" in queue_type:
        linestyle = '-.'
        marker = 'D'

    if "faa" in queue_type:
        linestyle = '--'
        marker = 'v'

    if "m&s" in queue_type:
        linestyle = ':'

    if "lscq" in queue_type:
        marker = 's'

    if "lcrq" in queue_type:
        marker = '*'

    if "lprq" in queue_type:
        marker = 'o'

    if marker == 'v':
        markersize=9
    if marker == '*':
        markersize=11.6

    return {
        'linestyle' : linestyle,
        'marker' : marker,
        'markersize' : markersize
    }


pp.service_naming(pph.get_service_name_for_label_with_remap)
pp.inject_plot_args(get_chart_style)

usage = 'usage: python3 draw_compare.py [-m <metric>] [<files1> <file2>]'
options, files = pph.parse_args(['-m'], usage)

if len(files) == 0:
    files = [f for f in os.listdir() if f.lower().endswith('.csv')]

if len(files) != 2:
    print(usage)
    print("Specify two input CSV files")
    exit(1)

metric = options.get('-m')


def is_symmetric(f):
    q_pc = pp.read_thrpt_data(f, "producerConsumer")
    return len(q_pc) == 0

def get_names_diff(f1, f2):
    l = 0
    while l < len(f1) and l < len(f2) and f1[l] == f2[l]:
        l += 1
    f1 = f1[l:]
    f2 = f2[l:]
    r = 0
    while r < len(f1) and r < len(f2) and f1[-r-1] == f2[-r-1]:
        r += 1
    if r > 0:
        f1 = f1[:-r]
        f2 = f2[:-r]
    return f1, f2

def load_data(f):
    if metric is None:
        if is_symmetric(f):
            q = pp.read_thrpt_data(f, "enqDeqPairs")
            q["Score"] /= 2
            q["Score Error"] /= 2
        else:
            q = pp.read_thrpt_data(f, "producerConsumer")
    else:
        if is_symmetric(f):
            q = pp.read_metrics_data(f, "enqDeqPairs", metric, False)
        else:
            q = pp.read_metrics_data(f, "producerConsumer", metric, False)
    return q


n1, n2 = get_names_diff(files[0], files[1])
q1 = load_data(files[0])
q2 = load_data(files[1])
queues = set(q1["Service Name"]).intersection(set(q2["Service Name"]))
n = len(queues)

if n == 0:
    print("The specified CSV files contain data for different queues")
    exit(1)

plt.rc('font', size=13)
plt.rc('lines', linewidth=2.2)
if metric is None:
    pp.ylabel("Throughput, transfers / s")
else:
    pp.ylabel(metric)
if n == 1:
    plt.figure(figsize=(8, 6))
else:
    pp.subplots(ncols=2, nrows=(n + 1) // 2, figsize=(16, (n + 1) // 2 * 5.5))
for queue in queues:
    q_cur1 = q1[q1["Service Name"] == queue].copy()
    q_cur1["Service Name"] = n1
    q_cur2 = q2[q2["Service Name"] == queue].copy()
    q_cur2["Service Name"] = n2
    q_cur = pd.concat([q_cur1, q_cur2])
    pp.plot_throughput(q_cur)
    plt.title(queue)
    plt.xscale('log')
    pp.xticks(q_cur)

plt.savefig('comparison-charts.pdf', bbox_inches='tight')
