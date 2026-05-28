#!/usr/bin/env bash
# regenerate the README perf chart: run the criterion suite bench, then render
# the combined grouped-bar chart from criterion's own results in target/criterion
#
# the matplotlib venv (benches/.venv, gitignored) is created automatically on
# first run; `uv` must be on PATH
#
# usage: benches/scripts/charts.sh [-f|--force]
#   -f, --force - re-run the criterion bench even if results already exist;
#                 otherwise existing target/criterion results are reused and the
#                 script just re-renders the chart
#
# author: aav
set -euo pipefail
cd "$(dirname "$0")/../.."

# --------------------------------------------------
# args
# --------------------------------------------------
FORCE=0
case "${1:-}" in
    -f | --force) FORCE=1 ;;
    "") ;;
    *) echo "error: unknown arg '$1' (expected -f|--force)" >&2; exit 1 ;;
esac

# --------------------------------------------------
# constants
# --------------------------------------------------
CRITERION_DIR="target/criterion"
SENTINEL="$CRITERION_DIR/flat_decode_clean/tinyklv/new/estimates.json"
VENV="benches/.venv"
PY="$VENV/bin/python"

# --------------------------------------------------
# bootstrap the plotting venv if it is missing
# --------------------------------------------------
if [[ ! -x "$PY" ]]; then
    echo "==> creating $VENV (matplotlib + numpy)"
    command -v uv >/dev/null || { echo "error: uv not found on PATH" >&2; exit 1; }
    uv venv "$VENV"
    uv pip install --python "$PY" matplotlib numpy
fi

# --------------------------------------------------
# run the criterion suite bench (skip if results exist and not forced)
# --------------------------------------------------
if [[ "$FORCE" -eq 1 || ! -f "$SENTINEL" ]]; then
    echo "==> running criterion suite bench"
    RUSTFLAGS='-C target-cpu=native' cargo bench --bench suite
else
    echo "==> reusing existing criterion results (pass -f to re-run)"
fi

# --------------------------------------------------
# render the chart from criterion's medians
# --------------------------------------------------
echo "==> rendering chart"
"$PY" benches/scripts/plot_times.py "$CRITERION_DIR" v lin bar benches/bench.jpg
echo "==> done - image written to benches/bench.jpg"
