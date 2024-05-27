# usage: download_traces.py data/net-traces/trace_name.toml
# the binary will download the traces specified in the TOML file and put them in the data/net-traces/trace_name/raw directory.

import os
import toml
import argparse

def download_traces(toml_filepath, user, password):
    # Load the TOML file
    with open(toml_filepath, 'r') as toml_file:
        config = toml.load(toml_file)
        output_dir = toml_filepath[:-5] + "/raw"  # Replace removesuffix with string slicing
        url = config['url']
        prefix = config['prefix']
        pcaps = [url + prefix + trace + ".UTC.anon.pcap" for trace in config['traces']]
        times = [url + prefix + trace + ".UTC.anon.times" for trace in config['traces']]

        for path in pcaps + times:
            os.system("mkdir -p {}".format(output_dir))
            if not os.path.exists("{}/{}".format(output_dir, os.path.basename(path))):
                if not os.path.exists("{}/{}.gz".format(output_dir, os.path.basename(path))):
                    os.system("aria2c -x 8 -d {} {}.gz --http-user={} --http-passwd={}".format(output_dir, path, user, password))
                # unzip the file
                os.system("gunzip {}/{}.gz".format(output_dir, os.path.basename(path)))
            else:
                print("File {} already exists".format(path))

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Download traces specified in a TOML file")
    parser.add_argument("toml_filepath", type=str, help="Path to the TOML file")
    parser.add_argument("--user", type=str, help="Username for the traces", required=True)
    parser.add_argument("--password", type=str, help="Password for the traces", required=True)
    args = parser.parse_args()

    download_traces(args.toml_filepath, args.user, args.password)
