#!/usr/bin/env python3
"""Produce bench.csv (median times: absolute, per element/packet, per byte) from target/criterion.

Usage: csv_times.py [criterion_dir] [out_csv]
  criterion_dir — the `target/criterion` tree from `cargo bench --bench compsuite` (default:
                  target/criterion).
  out_csv       — output csv path (default: benches/bench.csv).

Reads the same criterion files as plot_times.py (no second timing path):
  median ns         <criterion>/<group>/<approach>/new/estimates.json  ["median"]["point_estimate"]
  input bytes/call  <criterion>/<group>/<approach>/new/benchmark.json   ["throughput"]["Bytes"]
  elements/call     the streamed tier decodes `stream_packets` per call (from the suite_meta.json
                    sidecar, fallback 1); every other tier is 1 element/call.

Each criterion group id is "<record>_<operation>_<framing>" (e.g. simple_encode_value). Columns:
  record, operation, framing, approach, median_ns, ns_per_element, ns_per_byte
`ns_per_element` divides the absolute median by the per-call element count (only the streamed tier
differs from 1); `ns_per_byte` divides by criterion's persisted input-byte throughput. Rows whose
median is absent (the tier/approach was not benched, e.g. a filtered run) are skipped, so the csv
reflects exactly what is currently in `target/criterion` - run a full `gencharts.sh -f` first for a
complete table. Stdlib only; no venv needed.

Author: aav
"""

from __future__ import annotations

import csv
import json
import sys
from pathlib import Path

# --------------------------------------------------
# constants
# --------------------------------------------------
# the eight benchmarked approaches (criterion function ids), mirroring plot_times.py
APPROACHES = [
    "tinyklv",
    "manual",
    "serde_klv",
    "tlv_parser",
    "prost",
    "quick_protobuf",
    "rust_protobuf",
    "micropb",
]

# the record shapes, operations, and tiers that compose each criterion group id
RECORDS = ["simple", "compound", "rich"]
OPERATIONS = ["decode", "encode"]
FRAMINGS = ["value", "frame", "streamed"]

# csv column order (the first five match the prior bench.csv; the last two are the new metrics)
FIELDNAMES = [
    "record",
    "operation",
    "framing",
    "approach",
    "median_ns",
    "ns_per_element",
    "ns_per_byte",
]


# --------------------------------------------------
# criterion readers (same files plot_times.py uses)
# --------------------------------------------------
def stream_packets(croot: Path) -> int:
    """Returns the per-call element count for the streamed tier, from the suite_meta.json sidecar.

    # Arguments

    * `croot` - the `target/criterion` root directory.

    # Returns

    The persisted `stream_packets` count clamped to at least 1, or 1 when the sidecar is absent.
    """
    meta = croot / "suite_meta.json"
    if not meta.is_file():
        return 1
    return max(1, int(json.loads(meta.read_text()).get("stream_packets", 1)))


def median_ns(croot: Path, group: str, approach: str) -> float | None:
    """Returns the criterion median estimate in ns for one bar, or `None` if it was not measured.

    # Arguments

    * `croot` - the `target/criterion` root directory.
    * `group` - the criterion group id (e.g. `simple_encode_value`).
    * `approach` - the approach's criterion function id.

    # Returns

    The median point-estimate in nanoseconds, or `None` when the estimates file is absent.
    """
    path = croot / group / approach / "new" / "estimates.json"
    if not path.is_file():
        return None
    return float(json.loads(path.read_text())["median"]["point_estimate"])


def input_bytes(croot: Path, group: str, approach: str) -> float | None:
    """Returns the per-call input byte count for one bar from criterion's persisted `Throughput`.

    # Arguments

    * `croot` - the `target/criterion` root directory.
    * `group` - the criterion group id.
    * `approach` - the approach's criterion function id.

    # Returns

    The byte count, or `None` when the benchmark file or its `Bytes` throughput is absent.
    """
    path = croot / group / approach / "new" / "benchmark.json"
    if not path.is_file():
        return None
    throughput = json.loads(path.read_text()).get("throughput")
    if not isinstance(throughput, dict) or "Bytes" not in throughput:
        return None
    return float(throughput["Bytes"])


# --------------------------------------------------
# csv assembly
# --------------------------------------------------
def rows(croot: Path) -> list[dict[str, object]]:
    """Builds one csv row per measured (record, operation, framing, approach) combination.

    # Arguments

    * `croot` - the `target/criterion` root directory.

    # Returns

    The list of row dicts (absolute median plus per-element and per-byte normalizations), skipping
    any bar whose median estimate is absent.
    """
    # --------------------------------------------------
    # the streamed tier is the only one that decodes more than one element per call
    # --------------------------------------------------
    packets = stream_packets(croot)
    out: list[dict[str, object]] = []
    # --------------------------------------------------
    # walk the fixed (record x operation x framing) group grid, then each approach within it
    # --------------------------------------------------
    for record in RECORDS:
        for operation in OPERATIONS:
            for framing in FRAMINGS:
                group = f"{record}_{operation}_{framing}"
                elements = packets if framing == "streamed" else 1
                for approach in APPROACHES:
                    median = median_ns(croot, group, approach)
                    if median is None:
                        continue
                    byte_count = input_bytes(croot, group, approach)
                    out.append(
                        {
                            "record": record,
                            "operation": operation,
                            "framing": framing,
                            "approach": approach,
                            "median_ns": round(median, 1),
                            "ns_per_element": round(median / elements, 2),
                            "ns_per_byte": round(median / byte_count, 4) if byte_count else "",
                        }
                    )
    return out


def main() -> None:
    """Parses the optional cli paths, reads criterion, and writes the bench csv."""
    # --------------------------------------------------
    # resolve the criterion dir and output path (both optional, with sensible defaults)
    # --------------------------------------------------
    croot = Path(sys.argv[1]) if len(sys.argv) > 1 else Path("target/criterion")
    out_csv = Path(sys.argv[2]) if len(sys.argv) > 2 else Path("benches/bench.csv")
    if not croot.is_dir():
        sys.exit(f"criterion dir not found: {croot} (run `cargo bench --bench compsuite` first)")
    # --------------------------------------------------
    # assemble and write the rows
    # --------------------------------------------------
    data = rows(croot)
    out_csv.parent.mkdir(parents=True, exist_ok=True)
    with out_csv.open("w", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=FIELDNAMES)
        writer.writeheader()
        writer.writerows(data)
    print(f"wrote {out_csv} ({len(data)} rows)")


if __name__ == "__main__":
    main()
