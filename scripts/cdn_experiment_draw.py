import matplotlib.pyplot as plt
import numpy as np
import pandas as pd
import argparse

def draw_cdn_experiment(path: str):
    """
    example csv:
    LRU,LRU-MAD
21687.45,19234.46
21522.81,18996.12
21628.2,19259.95
21590.52,19271.07
21612.07,19331.76
    """

    df = pd.read_csv(path)

    # convert to ms (from ns)
    df = df / 1e3

    categories = ["LRU", "LRU-MAD"]
    lru_latency = np.mean(df["LRU"])
    lru_mad_latency = np.mean(df["LRU-MAD"])
    lru_std = np.std(df["LRU"])
    lru_mad_std = np.std(df["LRU-MAD"])

    bar_width = 0.1
    x = np.arange(1)

    fig, ax = plt.subplots(figsize=(3.7, 4))


    rects1 = ax.bar(x, [lru_latency], bar_width, label="LRU", yerr=lru_std, zorder=3)
    rects2 = ax.bar(x + bar_width, [lru_mad_latency], bar_width, label="LRU-MAD", yerr=lru_mad_std, zorder=3)

    # label with values and std deviation, like value (± std deviation)
    ax.bar_label(rects1, padding=3, labels=[f"{lru_latency:.2f}\n(±{lru_std:.2f})"])
    ax.bar_label(rects2, padding=3, labels=[f"{lru_mad_latency:.2f}\n(±{lru_mad_std:.2f})"])


    ax.set_ylabel("Average Latency (ms)")
    ax.set_ylim(0, 30)
    ax.set_title("CDN Experiment Results")
    ax.set_xticks(x + bar_width / 2, ["RTT=130ms"])
    ax.legend()

    # add grid
    ax.grid(True, which='both', linestyle='--', linewidth=0.5, zorder=0)

    plt.savefig("cdn_experiment.png")
    

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Draw CDN experiment results")
    parser.add_argument("path", type=str, help="Path to the CSV file")
    args = parser.parse_args()

    draw_cdn_experiment(args.path)