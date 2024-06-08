# MS Production Traces

Download `BuildServer00` and `BuildServer01` from `http://iotta.snia.org/traces/block-io/158` and put it here (`data/ms_prod/`). Then, run the following command to extract the trace:

```bash
unzip -n BuildServer00.zip
rm -f BuildServer00.zip
unzip -n BuildServer01.zip
rm -f BuildServer01.zip
```