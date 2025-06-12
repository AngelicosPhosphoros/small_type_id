import argparse
import os
import subprocess

if __name__ != "__main__":
    raise Exception("Please, just execute script")

parser = argparse.ArgumentParser(
    prog="python cmp_output.py",
    description="Compares program output against expected values",
)

parser.add_argument("executable")
parser.add_argument("--exp_code", required=True, type=int)
parser.add_argument("--exp_stdout", required=False)
parser.add_argument("--exp_stderr", required=False)

args = parser.parse_args()

if os.name == "nt" or args.exp_code == 0:
    ret_code = args.exp_code
else:
    # We terminate process using signals on Unix
    # so use -SIGABRT instead.
    ret_code = -6

if args.exp_stdout:
    expected_stdout = open(args.exp_stdout, "rb").read()
else:
    expected_stdout = b""

if args.exp_stderr:
    expected_stderr = open(args.exp_stderr, "rb").read()
else:
    expected_stderr = b""

run_res = subprocess.run(
    args.executable,
    capture_output=True
)

assert run_res.returncode == ret_code, f"Return code doesn't match: {run_res.returncode} != {ret_code}"
assert run_res.stdout == expected_stdout, f"stdout doesn't match: {repr(run_res.stdout)} != {repr(expected_stdout)}"
assert run_res.stderr == expected_stderr, f"stderr doesn't match: {repr(run_res.stderr)} != {repr(expected_stderr)}"
