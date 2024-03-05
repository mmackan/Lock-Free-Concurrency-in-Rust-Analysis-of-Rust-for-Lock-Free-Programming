import matplotlib.pyplot as plt
import pandas as pd


# -- globals

_service_naming = lambda s: s
_args_injector = lambda index: {}
_ylabel = "Throughput (ops/s)"
_subplots = []
_enable_legend = True
_xscale = lambda x: x

def service_naming(naming):
    global _service_naming
    _service_naming = naming

def inject_plot_args(injector):
    global _args_injector
    _args_injector = injector

def ylabel(label):
    global _ylabel
    _ylabel = label

def enable_legend():
    global _enable_legend
    _enable_legend = True

def disable_legend():
    global _enable_legend
    _enable_legend = False

def xscale(f):
    global _xscale
    if f is None:
        f = lambda x: x
    _xscale = f

def clear_subplots():
    global _subplots
    _subplots = []

def subplots(*args, **kwargs):
    global _subplots
    fig, ax = plt.subplots(*args, **kwargs)
    _subplots = list(ax.flat)


def _next_subplot():
    global _subplots
    if len(_subplots) > 0:
        ax = _subplots[0]
        _subplots = _subplots[1:]
        plt.sca(ax)


# globals --


def read_thrpt_data(file, bench):
    if type(file) == list:
        data = pd.concat(list(pd.read_csv(f, index_col=False) for f in file))
    else:        
        data = pd.read_csv(file, index_col=False)

    data = data.rename(columns={
        "Score Error (99.9%)": "Score Error",
        "Param: serviceName": "Service Name",
        "Param: queueType": "Service Name",
        "Param: recoveryMode": "Service Name"
    })
    if "Mode" not in data.columns:
        data["Mode"] = "thrpt"

    basic_col = pd.Index(["Threads", "Score", "Score Error", "Service Name"])

    thrpt = data[
        (data["Benchmark"].str.contains(bench)) & ~(data["Benchmark"].str.contains(":")) & (data["Mode"] == "thrpt")]
    col = thrpt.columns
    non_unique_params = col[col.str.startswith("Param:") & (thrpt.nunique() > 1)]
    thrpt = thrpt[basic_col.union(non_unique_params, False)]

    return thrpt

def read_asymmetric_thrpt_data(file, bench):
    if type(file) == list:
        if len(file) == 0:
            return pd.DataFrame()
        data = pd.concat(list(pd.read_csv(f, index_col=False) for f in file))
    else:        
        data = pd.read_csv(file, index_col=False)

    data = data.rename(columns={
        "Score Error (99.9%)": "Score Error",
        "Param: serviceName": "Service Name",
        "Param: queueType": "Service Name",
        "Param: recoveryMode": "Service Name"
    })
    if "Mode" not in data.columns:
        data["Mode"] = "thrpt"

    basic_col = pd.Index(["Threads", "Score", "Score Error", "Service Name"])
    basic_params = pd.Index(["Threads", "Service Name"])

    thrpt = data[
        (data["Benchmark"].str.contains(bench)) & data["Benchmark"].str.contains(":") & (data["Mode"] == "thrpt")]
    col = thrpt.columns
    non_unique_params = col[col.str.startswith("Param:") & (thrpt.nunique() > 1)]
    
    params = basic_params.union(non_unique_params, False).intersection(thrpt.columns, False)
    unique_index = thrpt[params].drop_duplicates()
    
    thrpt_res = []

    for _, index in unique_index.iterrows():
        thrpt_for_index = thrpt[(thrpt[unique_index.columns] == index).all(axis=1)]
        thrpt_for_index = thrpt_for_index.loc[thrpt_for_index["Score"].idxmin(axis=0)]
        thrpt_res.append(thrpt_for_index)

    thrpt = pd.DataFrame(thrpt_res)
    thrpt = thrpt[basic_col.union(non_unique_params, False)]

    return thrpt

def read_metrics_data(file, bench, metric, normalize=True):
    if type(file) == list:
        if len(file) == 0:
            return pd.DataFrame()
        data = pd.concat(list(pd.read_csv(f, index_col=False) for f in file))
    else:        
        data = pd.read_csv(file, index_col=False)
        
    if "Mode" not in data.columns:
        data["Mode"] = "thrpt"

    data = data.rename(columns={
        "Score Error (99.9%)": "Score Error",
        "Param: serviceName": "Service Name",
        "Param: queueType": "Service Name",
        "Param: recoveryMode": "Service Name"
    })

    basic_col = pd.Index(["Threads", "Score", "Score Error", "Service Name"])

    met = data[data["Benchmark"].str.contains(bench) & data["Benchmark"].str.contains(":get" + metric) & (data["Mode"] == "thrpt")]
    col = met.columns
    non_unique_params = col[col.str.startswith("Param:") & (met.nunique() > 1)]
    met = met[basic_col.union(non_unique_params, False)].reset_index(drop=True)

    if normalize:
        thrpt = read_thrpt_data(file, bench)["Score"].reset_index(drop=True)
        met["Score"] /= thrpt
        met["Score Error"] /= thrpt

    return met


def _get_label(index):
    res = _service_naming(index["Service Name"])
    for param in index.index[index.index != "Service Name"]:
        name = param[len("Param: "):] if param.startswith("Param: ") else param
        val = index[param]
        res += "; " + name + "=" + str(val)
    return res

def plot_throughput_by(thrpt, x_col, x_label=None, start_from_zero=True):
    _next_subplot()
    plt.gcf().patch.set_facecolor("white")

    if x_label == None:
        x_label = x_col
    plt.xlabel(x_label)
    plt.ylabel(_ylabel)

    params = thrpt.columns[thrpt.columns.str.startswith("Param:")]
    params = params.union(["Service Name", "Threads"], False).drop([x_col]).intersection(thrpt.columns, False)
    unique_index = thrpt[params].drop_duplicates()

    lines = []

    for _, index in unique_index.iterrows():
        thrpt_for_index = thrpt[(thrpt[unique_index.columns] == index).all(axis=1)]
        x = _xscale(thrpt_for_index[x_col])
        y = thrpt_for_index["Score"]
        e = thrpt_for_index["Score Error"]
        l = plt.errorbar(x, y, yerr=e, capsize=3, label=_get_label(index), **_args_injector(index))
        lines.append(l)

    if _enable_legend:
        plt.legend()
    plt.grid()
    if start_from_zero and (thrpt["Score"] > 0.0).all():
        plt.ylim(bottom=0.0)

    return lines

def plot_throughput(thrpt, start_from_zero=True):
    return plot_throughput_by(thrpt, "Threads", start_from_zero=start_from_zero)

def xticks(data, x_col="Threads"):
    ticks = data[x_col].unique()
    plt.xticks(_xscale(ticks), ticks)
