"""Process-paired log-ratio estimator and deterministic bootstrap intervals."""
import math
import random
import statistics

def summarize(pairs, bootstrap=10000):
    if not pairs or any(a <= 0 or b <= 0 for a,b in pairs):
        raise ValueError('positive paired timings required')
    logs = [math.log(b/a) for a,b in pairs]
    ratio = math.exp(statistics.mean(logs))
    rng = random.Random(20260924)
    draws = sorted(math.exp(statistics.mean(rng.choices(logs,k=len(logs)))) for _ in range(bootstrap))
    low = draws[int(.025*bootstrap)]
    high = draws[min(bootstrap-1,int(.975*bootstrap))]
    return {
        'pairs':len(pairs),
        'neon_median':statistics.median(a for a,b in pairs),
        'sve_median':statistics.median(b for a,b in pairs),
        'paired_geomean_time_ratio':ratio,
        'time_reduction_pct':100*(1-ratio),
        'time_reduction_ci95_pct':[100*(1-high),100*(1-low)],
        'throughput_gain_pct':100*(1/ratio-1),
    }
