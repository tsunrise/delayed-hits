# usage: download_traces.py data/net-traces/trace_name.toml
# the binary will download the traces specified in the TOML file and put them in the data/net-traces/trace_name/raw directory.

import os
import sys
import toml
import argparse

def download_traces(toml_filepath: str, user: str, password: str):
    # Load the TOML file
    with open(toml_filepath, 'r') as toml_file:
        config = toml.load(toml_file)
        output_dir = toml_filepath.removesuffix(".toml") + "/raw"
        prefix = config['prefix']
        pcaps = [prefix + trace + ".UTC.anon.pcap.gz" for trace in config['traces']]
        times = [prefix + trace + ".UTC.anon.times.gz" for trace in config['traces']]

        for path in pcaps + times:
            os.system(f"mkdir -p {output_dir}")
            if not os.path.exists(f"{output_dir}/{os.path.basename(path)}"):
                os.system(f"aria2c -x 8 -d {output_dir} {path} --http-user={user} --http-passwd={password}")
            else:
                print(f"File {path} already exists")

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Download traces specified in a TOML file")
    parser.add_argument("toml_filepath", type=str, help="Path to the TOML file")
    parser.add_argument("--user", type=str, help="Username for the traces", required=True)
    parser.add_argument("--password", type=str, help="Password for the traces", required=True)
    args = parser.parse_args()

    download_traces(args.toml_filepath, args.user, args.password)

    