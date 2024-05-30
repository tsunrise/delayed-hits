import argparse
import pandas as pd
import matplotlib.pyplot as plt

def draw_latency(data_path, label:str):

    plt.figure(figsize=(10, 6))
    df = pd.read_csv(data_path, header=None)

    way = df[0][0]
    assoc = df[1][0]
    latencies = df[2] / 1e6
    improvements = df[6].str[:-1].astype(float)
    plt.plot(latencies, improvements, label='LRU-MAD')

    plt.xscale('log')
    plt.xlabel('Latency (ms)')
    plt.ylabel('\% Latency Improvement')
    plt.legend()
    plt.title(f"{label}: {way}-way {assoc}-associative")
    # use triangle as datapoint
    plt.grid(True, which='both', linestyle='--', linewidth=0.5)
    plt.savefig(f'{label}.png')
# Load the data


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Draw latency improvement")
    parser.add_argument("data_path", type=str, help="Path to the data file")
    parser.add_argument("label", type=str, help="Label for the plot")
    args = parser.parse_args()

    draw_latency(args.data_path, args.label)