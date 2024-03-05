import matplotlib.pyplot as plt
import postprocess as pp
import pp_helpers as pph
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

usage = 'usage: python3 draw.py [-m <metric>] [<files>...]'
options, files = pph.parse_args(['-m'], usage)
if len(files) == 0:
    files = [f for f in os.listdir() if f.lower().endswith('.csv')]
n = len(files)

if n == 0:
    print(usage)
    print("Specify <files>")
    exit(1)

metric = options.get('-m')


def is_symmetric(f):
    q_pc = pp.read_thrpt_data(f, "producerConsumer")
    return len(q_pc) == 0


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
for f in files:
    if metric is None:
        if is_symmetric(f):
            pph.plot_symmetric_results(f, f[:-4])
        else:
            pph.plot_asymmetric_results(f, f[:-4])
    else:
        if is_symmetric(f):
            pph.plot_metric_results(f, "enqDeqPairs", metric, f[:-4])
        else:
            pph.plot_metric_results(f, "producerConsumer", metric, f[:-4])

plt.savefig('charts.pdf', bbox_inches='tight')
