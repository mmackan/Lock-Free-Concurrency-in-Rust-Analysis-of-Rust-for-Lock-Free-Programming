import sys
import numpy as np

if len(sys.argv) != 3:
    print("Invalid usage")
    exit(1)

mode = sys.argv[1]

if mode != "sym" and mode != "pc":
    print("Invalid mode")
    exit(1)

nproc = int(sys.argv[2])
possible_num_threads = sys.stdin.readline()
possible_num_threads = possible_num_threads.split()
possible_num_threads = list(map(int, possible_num_threads))

if mode == "sym":
    res = list(filter(lambda n: n <= nproc, possible_num_threads))
else:
    res = []
    for i in range(len(possible_num_threads) // 2):
        a = possible_num_threads[i * 2]
        b = possible_num_threads[i * 2 + 1]
        if a + b <= nproc:
            res += [a, b]

if mode == "sym":
    maxt = np.max(res)
    if maxt < nproc:
        res.append(nproc)
else:
    res_pairs = list(zip(res[::2], res[1::2]))
    res_sums = list(map(np.sum, res_pairs))
    res_ratios = list(map(
        lambda r: (r[0] // np.gcd(*r), r[1] // np.gcd(*r)),
        filter(
            lambda r: r[0] * r[1] != 0,
            res_pairs
        )
    ))
    if len(np.unique(res_ratios, axis=0)) == 1:
        ratio = res_ratios[0]
        maxt = np.max(res_sums)
        mult = nproc // (ratio[0] + ratio[1])
        a = mult * ratio[0]
        b = mult * ratio[1]
        if a + b > maxt:
            res += [a, b]

print(' '.join(map(str, res)))
