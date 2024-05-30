import matplotlib.pyplot as plt
import numpy as np

def analyze_latency_for(ax, request_timestamps, response_timestamps, label):


    mask = response_timestamps < request_timestamps
    response_timestamps[mask] = request_timestamps[mask]

    latency = response_timestamps - request_timestamps
    print(f"({label}) Average latency: {np.mean(latency)/1e6}ms")

    misses = latency[(latency >= 5e7) & (latency < 2e8)]/1e6
    ax.hist(misses, bins=100, alpha=0.5, label=f"{label} (cnt: {len(misses)})")

    

def analyze_latency(request_timestamps_lru, response_timestamps_lru, request_timestamps_lru_mad, response_timestamps_lru_mad):
    fig, ax = plt.subplots(1, 1, figsize=(10, 5))
    ax.set_title("Distribution of Missed + Delayed Hit Objects")
    ax.set_xlabel("Latency (ms)")
    ax.set_ylabel("Number of objects")
    ax.get_yaxis().get_major_formatter().set_scientific(False)

    analyze_latency_for(ax, request_timestamps_lru, response_timestamps_lru, "LRU")
    analyze_latency_for(ax, request_timestamps_lru_mad, response_timestamps_lru_mad, "LRU-MAD")
    
    ax.legend()
    plt.show()

def analyze_hit_rate(request_timestamps_lru, response_timestamps_lru, request_timestamps_lru_mad, response_timestamps_lru_mad):
    """
    Draw a bar chart for hit counts and non-hit counts for LRU and LRU-MAD
    use Grouped bar chart with labels

    direct hit if latency < 1ms, or not hit

    example:
    # data from https://allisonhorst.github.io/palmerpenguins/

import matplotlib.pyplot as plt
import numpy as np

species = ("Adelie", "Chinstrap", "Gentoo")
penguin_means = {
    'Bill Depth': (18.35, 18.43, 14.98),
    'Bill Length': (38.79, 48.83, 47.50),
    'Flipper Length': (189.95, 195.82, 217.19),
}

x = np.arange(len(species))  # the label locations
width = 0.25  # the width of the bars
multiplier = 0

fig, ax = plt.subplots(layout='constrained')

for attribute, measurement in penguin_means.items():
    offset = width * multiplier
    rects = ax.bar(x + offset, measurement, width, label=attribute)
    ax.bar_label(rects, padding=3)
    multiplier += 1

# Add some text for labels, title and custom x-axis tick labels, etc.
ax.set_ylabel('Length (mm)')
ax.set_title('Penguin attributes by species')
ax.set_xticks(x + width, species)
ax.legend(loc='upper left', ncols=3)
ax.set_ylim(0, 250)

plt.show()
    """
    fig, ax = plt.subplots(1, 1, figsize=(10, 5))
    ax.set_title("Hit Rate of LRU and LRU-MAD")
    ax.set_xlabel("Hit Rate")

    hit_lru = np.mean(response_timestamps_lru - request_timestamps_lru < 1e6)
    hit_lru_mad = np.mean(response_timestamps_lru_mad - request_timestamps_lru_mad < 1e6)

    miss_lru = 1 - hit_lru
    miss_lru_mad = 1 - hit_lru_mad

    x = np.arange(2)
    width = 0.25

    rects1 = ax.bar(x, [hit_lru, miss_lru], width, label="LRU")
    rects2 = ax.bar(x + width, [hit_lru_mad,miss_lru_mad], width, label="LRU-MAD")

    ax.bar_label(rects1, padding=3)
    ax.bar_label(rects2, padding=3)

    ax.set_xticks(x + width/2, ["Hit", "Misses"])
    ax.legend(loc='upper left', ncols=2)
    ax.set_ylim(0, 1)

    plt.show()


if __name__ == "__main__":
    # analyze_timestamps("request_timestamps.npy", "response_timestamps.npy")
    import argparse

    parser = argparse.ArgumentParser(description="Analyze latency from request and response timestamps, requires request[i] and response[i] to represent the same object")
    parser.add_argument("f", type=str, help="function to analyze the latency")
    args = parser.parse_args()

    # Load the binary request timestamps (u64 for each)
    request_timestamps_lru = np.load("request_starts_lru.npy")
    # Load the binary response timestamps (u64 for each)
    response_timestamps_lru = np.load("request_ends_lru.npy")

    request_timestamps_lru_mad = np.load("request_starts_lru-mad.npy")
    response_timestamps_lru_mad = np.load("request_ends_lru-mad.npy")

    if args.f == "analyze_latency":
        analyze_latency(request_timestamps_lru, response_timestamps_lru, request_timestamps_lru_mad, response_timestamps_lru_mad)
    elif args.f == "analyze_hit_rate":
        analyze_hit_rate(request_timestamps_lru, response_timestamps_lru, request_timestamps_lru_mad, response_timestamps_lru_mad)