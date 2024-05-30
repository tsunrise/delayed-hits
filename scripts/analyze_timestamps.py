import matplotlib.pyplot as plt
import numpy as np

def analyze_timestamps(request_timestamps_path, response_timestamps_path):
    # Load the binary request timestamps (u64 for each)
    request_timestamps = np.load(request_timestamps_path)
    # Load the binary response timestamps (u64 for each)
    response_timestamps = np.load(response_timestamps_path)

    print("Number of request timestamps: {}".format(len(request_timestamps)))
    print("Number of response timestamps: {}".format(len(response_timestamps)))
    print("Average delay {}".format(np.mean(response_timestamps - request_timestamps)))
    print("Request timestamps: {}".format(request_timestamps[12:]))
    print("Response timestamps: {}".format(response_timestamps[12:]))
    # plot both in the same histogram
    fig, ax = plt.subplots()
    ax.hist(request_timestamps, bins=100, alpha=0.5, label='Request Timestamps')
    ax.hist(response_timestamps, bins=100, alpha=0.5, label='Response Timestamps')
    ax.legend()
    plt.show()


if __name__ == "__main__":
    # analyze_timestamps("request_timestamps.npy", "response_timestamps.npy")
    import argparse

    parser = argparse.ArgumentParser(description="Analyze request and response timestamps")
    parser.add_argument("request_timestamps_path", type=str, help="Path to the binary file containing the request timestamps")
    parser.add_argument("response_timestamps_path", type=str, help="Path to the binary file containing the response timestamps")
    args = parser.parse_args()

    analyze_timestamps(args.request_timestamps_path, args.response_timestamps_path)