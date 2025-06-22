import argparse
import dataclasses
import os
import shutil
import subprocess
import time

@dataclasses.dataclass(frozen=True, kw_only=True)
class TestSet:
    features: frozenset[str]
    ret_code: int
    stdout: str
    stderr: str

def run_test(params: TestSet, is_release: bool, lto: str, use_asan: bool):
    build_args = "cargo build --verbose --workspace --exclude benches --bin duplicate_type_ids_handling"
    env = None
    if params.features:
        build_args += f" --features={','.join(params.features)}"
    if is_release:
        build_args += f" --release"
    if use_asan:
        build_args += " -Zbuild-std"
        env = { **os.environ, "RUSTFLAGS" : "-Zsanitizer=address" }

    if params.stdout:
        out = open(params.stdout, "rb").read()
    else:
        out = b""

    if params.stderr:
        err = open(params.stderr, "rb").read()
    else:
        err = b""

    workspace_cargo = "../Cargo.toml"
    if lto:
        shutil.copy2(workspace_cargo, workspace_cargo + ".orig")
        with open(workspace_cargo, "a") as f:
            f.write(f'\n[profile.release]\nlto = "{lto}"\n')
    try:
        if lto:
            print(f'Running with lto="{lto}"')
        else:
            print("Running without lto")
        print(f"    > {build_args}")
        print(build_args.split())
        subprocess.run(build_args.split(), check=True, env=env)
        if is_release:
            executable = "../target/release/duplicate_type_ids_handling"
        else:
            executable = "../target/debug/duplicate_type_ids_handling"
        run_res = subprocess.run(executable, capture_output=True)
        assert run_res.returncode == params.ret_code, f"Return code doesn't match: {run_res.returncode} != {params.ret_code}"
        assert run_res.stdout == out, f"stdout doesn't match: {repr(out)} != {repr(params.stdout)}"
        assert run_res.stderr == err, f"stderr doesn't match: {repr(err)} != {repr(params.stderr)}"
    finally:
        if lto:
            shutil.move(workspace_cargo + ".orig", workspace_cargo)

def set_current_dir():
    abspath = os.path.abspath(__file__)
    dname = os.path.dirname(abspath)
    os.chdir(dname)

if __name__ != "__main__":
    raise Exception("Please, just execute script")

start_time = time.time()

if os.name == "nt":
    error_code = 2
else:
    # SIGABRT
    error_code = -6
fs = frozenset
set_current_dir()
tests = (
    TestSet(features=fs(), ret_code=error_code, stdout="", stderr="etalons/auto_no_names.txt"),
    TestSet(features=fs({"debug_type_name"}), ret_code=error_code,
        stdout="", stderr="etalons/auto_with_names.txt"),
    TestSet(features=fs({"unsafe_remove_duplicate_checks"}), ret_code=0,
        stdout="etalons/m_stdout_no_names.txt", stderr="etalons/m_stderr.txt"),
    TestSet(features=fs({"unsafe_remove_duplicate_checks", "debug_type_name"}),
        ret_code=0, stdout="etalons/m_stdout_with_names.txt", stderr="etalons/m_stderr.txt"),
)

parser = argparse.ArgumentParser()
parser.add_argument("--use-asan", action='store_true')
args = parser.parse_args()

for t in tests:
    run_test(t, False, "", args.use_asan)
    for lto in ["off", "thin", "fat"]:
        run_test(t, True, lto, args.use_asan)

end_time = time.time()
print(f"Running tests took {end_time - start_time:.3} seconds")
