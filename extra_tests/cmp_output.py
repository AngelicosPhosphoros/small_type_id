import argparse
import dataclasses
import functools
import os
import platform
import re
import shutil
import subprocess
import time

print = functools.partial(print, flush=True)

@dataclasses.dataclass(frozen=True, kw_only=True)
class TestSet:
    features: frozenset[str]
    ret_code: int
    stdout: str
    stderr: str

def lookup_clang_asan():
    assert platform.system() == "Windows"
    paths = [
        "C:/Program Files/LLVM/lib/clang/",
        "C:/LLVM/lib/clang/",
    ]
    only_numbers = re.compile(r"^\d+$")
    highest_version = None
    found = None
    for p in paths:
        if not os.path.isdir(p): continue
        subdirs = [f for f in os.listdir(p) if os.path.isdir(p + f)]
        subdirs = [f for f in subdirs if only_numbers.fullmatch(f)]
        if not subdirs: continue
        selected = max(int(f) for f in subdirs)
        if highest_version is None or highest_version < selected:
            highest_version = selected
            found = f"{p}{selected}/lib/windows/"
    if not found:
        raise Exception("Failed to find Address Sanitizer libraries")
    return found

def create_symlink(src: str, dst: str):
    assert platform.system() == "Windows"
    import _winapi
    _winapi.CreateJunction(src, dst)

def get_toolchain()->str:
    res = subprocess.run(["rustup", "toolchain", "list"], check=True, capture_output=True)
    active = [x for x in res.stdout.decode('utf-8').splitlines() if "active" in x][0]
    active = active.split()[0]
    if "aarch64-apple-darwin" in active:
        return "aarch64-apple-darwin"
    toolchain = re.search(r"\w+\-\w+\-\w+\-\w+$", active)[0]
    return toolchain


def run_test(params: TestSet, is_release: bool, lto: str, use_asan: bool, target: str):
    build_args = "cargo build --verbose --workspace --exclude benches --bin duplicate_type_ids_handling"
    build_args += f" --target={target}"
    if params.features:
        build_args += f" --features={','.join(params.features)}"
    if is_release:
        build_args += f" --release"
    if use_asan:
        build_args += " -Zbuild-std"

    if is_release:
        mod_str = "release"
    else:
        mod_str = "debug"

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
        subprocess.run(build_args.split(), check=True)
        executable = f"../target/{target}/{mod_str}/duplicate_type_ids_handling"
        print(f"Running\n    {executable}")
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

set_current_dir()

if platform.system() == "Windows":
    error_code = 2
else:
    # SIGABRT
    error_code = -6

fs = frozenset
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

if args.use_asan:
    flags = os.environ.get("RUSTFLAGS", "")
    os.environ["RUSTFLAGS"] = flags + ' -Zsanitizer=address'

if args.use_asan and platform.system() == "Windows":
    llvm_junction = "../target/llvm"
    if os.path.isdir(llvm_junction):
        os.remove(llvm_junction)
    asan_dir = lookup_clang_asan()
    create_symlink(asan_dir, llvm_junction)
    llvm_junction = os.path.abspath(llvm_junction)
    print(f"Created junction to ASAN from {asan_dir} to {llvm_junction}")
    os.environ["RUSTFLAGS"] += " -l dylib=clang_rt.asan_dynamic-x86_64"
    os.environ["RUSTFLAGS"] += f' -L{llvm_junction}'
    os.environ["PATH"] += f";{llvm_junction};"

toolchain = get_toolchain()

for t in tests:
    run_test(t, False, "", args.use_asan, toolchain)
    for lto in ["off", "thin", "fat"]:
        run_test(t, True, lto, args.use_asan, toolchain)

end_time = time.time()
print(f"Running tests took {end_time - start_time:.3} seconds")
