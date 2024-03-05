import numpy as np
import matplotlib.pyplot as plt
import postprocess as pp
import pp_helpers as pph


def get_chart_style(index):
    markers = ['o', 'v', 's', '*', 'D']

    if "Service Name" not in index:
        return None
    queue_type = pph.get_service_name_for_label(index["Service Name"])
    h1 = hash(queue_type + "$1")

    marker = markers[h1 % len(markers)]
    markersize = 8
    linestyle = '-'
    color = None

    if "cc-queue" in queue_type:
        linestyle = '-.'
        marker = 'D'
        color = 'C5'

    if "faa" in queue_type:
        linestyle = '--'
        marker = 'v'
        color = 'C0'

    if "m&s" in queue_type:
        linestyle = ':'

    if "lscq" in queue_type:
        marker = 's'
        color = 'C1'

    if "lcrq" in queue_type:
        marker = '*'
        color = 'C2'

    if "lprq" in queue_type:
        marker = 'o'
        color = 'C3'

    if marker == 'v':
        markersize=9
    if marker == '*':
        markersize=11.6

    return {
        'linestyle' : linestyle,
        'marker' : marker,
        'markersize' : markersize,
        'color' : color
    }


pp.service_naming(pph.get_service_name_for_label)
pp.inject_plot_args(get_chart_style)

with pph.paper_throughput_charts():
    pp.subplots(ncols=2, nrows=2, figsize=(16, 11))
    pph.plot_symmetric_results("cpp-res-enq-deq.csv", "Enqueue-dequeue pairs benchmark", pph.paper_queues())
    plt.xticks(2**np.arange(8))
    pph.plot_asymmetric_results("cpp-res-p1c1b.csv", "1:1 producer-consumer benchmark", pph.paper_queues())
    plt.xticks(2**np.arange(1, 8))
    pph.plot_asymmetric_results("cpp-res-p2c1b.csv", "2:1 producer-consumer benchmark", pph.paper_queues())
    plt.xticks(np.concatenate([2**np.arange(6) * 3, [126]]))
    pph.plot_asymmetric_results("cpp-res-p1c2b.csv", "1:2 producer-consumer benchmark", pph.paper_queues())
    plt.xticks(2**np.arange(6) * 3)
    plt.xticks(np.concatenate([2**np.arange(6) * 3, [126]]))

plt.savefig('charts.pdf', bbox_inches='tight')
