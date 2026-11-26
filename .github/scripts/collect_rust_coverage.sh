#!/usr/bin/env bash
set -euo pipefail

mkdir -p coverage
coverage_env=$(cargo llvm-cov show-env --sh)
eval "$coverage_env"
cargo llvm-cov clean --workspace
cargo test --workspace --locked 2>&1 | tee coverage/rust-tests.log
cargo build --locked -p meridian-desktop --bin meridian-desktop --example inspect_commander
target_dir="$CARGO_LLVM_COV_TARGET_DIR"
test "$target_dir" = "$PWD/target"
mapfile -t lavapipe < <(dpkg -L mesa-vulkan-drivers | grep -E '/lvp_icd(\.[^.]+)?\.json$')
test "${#lavapipe[@]}" -eq 1
test -f "${lavapipe[0]}"
VK_DRIVER_FILES="${lavapipe[0]}" VK_ICD_FILENAMES="${lavapipe[0]}" WINIT_X11_SCALE_FACTOR=1 \
  xvfb-run --auto-servernum --error-file=coverage/xvfb.log \
  --server-args="-screen 0 1600x900x24 -dpi 96 -nolisten tcp -noreset" \
  python -B .github/scripts/native_x11_session.py
llvm_bin="$(rustc --print target-libdir)/../bin"
"$llvm_bin/llvm-profdata" merge -sparse coverage/native/desktop-[0-9]*.profraw \
  -o coverage/native/desktop.profdata
"$llvm_bin/llvm-cov" export "$target_dir/debug/meridian-desktop" \
  -instr-profile=coverage/native/desktop.profdata -format=lcov > coverage/native/desktop.lcov
mule_profiles=(coverage/native/desktop-mule-*.profraw)
test "${#mule_profiles[@]}" -eq 1
test -s "${mule_profiles[0]}"
"$llvm_bin/llvm-profdata" merge -sparse "${mule_profiles[0]}" \
  -o coverage/native/mule-desktop.profdata
"$llvm_bin/llvm-cov" export "$target_dir/debug/meridian-desktop" \
  -instr-profile=coverage/native/mule-desktop.profdata -format=lcov > coverage/native/mule-desktop.lcov
set -o pipefail
python -B -m coverage run --parallel-mode .github/scripts/check_native_coverage.py \
  | tee coverage/native/native-line-gates.txt
cp coverage/native/desktop-*.profraw "$(dirname "$LLVM_PROFILE_FILE")/"
cargo llvm-cov report --workspace --lcov --output-path coverage/lcov.info
