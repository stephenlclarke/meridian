#!/usr/bin/env bash
set -euo pipefail

# The extra checkout isolates tests that create fixtures or inspect Git state.
# Relative source paths in .coveragerc let us combine both coverage databases.
work=$(mktemp -d)
mkdir -p coverage
run_python_suite() {
  python -B -m coverage run --parallel-mode -m unittest discover -s tools -p 'test_*.py' -v
}

case "${COVERAGE_SCHEDULE:?}" in
  parallel)
    git worktree add --quiet --detach "$work/python" HEAD
    trap 'git worktree remove --force "$work/python"' EXIT
    (cd "$work/python" && COVERAGE_FILE="$work/.coverage" run_python_suite) > coverage/python-tests.log 2>&1 &
    python_pid=$!
    rust_status=0
    bash .github/scripts/collect_rust_coverage.sh || rust_status=$?
    python_status=0
    wait "$python_pid" || python_status=$?
    tail -n 8 coverage/python-tests.log
    if (( rust_status || python_status )); then
      exit 1
    fi
    mv "$work"/.coverage.* .
    ;;
  serial)
    bash .github/scripts/collect_rust_coverage.sh
    run_python_suite 2>&1 | tee coverage/python-tests.log
    ;;
  *)
    echo "Unknown coverage schedule" >&2
    exit 2
    ;;
esac

python -m coverage combine
python -m coverage xml -o coverage/python.xml
python -m coverage report tools/measure_goshawk_surface_fit.py
