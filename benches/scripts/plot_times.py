#!/usr/bin/env python3
"""Render the combined grouped perf chart from criterion's own results.

Usage: plot_times.py <criterion_dir> <orientation> <scale> <kind> <out_jpg>
  criterion_dir — `target/criterion` produced by `cargo bench --bench suite`.
  orientation   — "v" (groups on the x-axis) or "h" (groups on the y-axis).
  scale         — "log" (value axis log-scaled) or "lin" (linear).
  kind          — "bar" (median grouped bars) or "box" (box-and-whisker over
                  criterion's raw per-iteration samples).

Numbers come straight from criterion — no second timing path. `bar` reads the
median point-estimate from each `new/estimates.json`; `box` reads the raw sample
timings from each `new/sample.json` (per-iteration ns = times[k] / iters[k]), so
the whiskers are the real measured distribution, not a summary.

The suite emits eight groups (flat/nested x decode/encode x clean/framed); each
becomes one cluster with one element per approach. tinyklv is drawn in a bold
color; the others are muted so the comparison reads at a glance. lower is faster.
"""

from __future__ import annotations

import json
import math
import sys
from pathlib import Path

import matplotlib.patches as mpatches
import matplotlib.pyplot as plt
import numpy as np

# --------------------------------------------------
# args
# --------------------------------------------------
CRITERION_DIR = Path(sys.argv[1])
ORIENTATION = sys.argv[2].lower()
SCALE = sys.argv[3].lower()
KIND = sys.argv[4].lower()
OUT_PATH = Path(sys.argv[5])

# --------------------------------------------------
# derived title (no CLI input: the legend already lists the frameworks)
# --------------------------------------------------
TITLE = "Median per-call time across KLV frameworks (lower is better)"

# --------------------------------------------------
# fixed group + approach ordering (and per-approach colors)
# --------------------------------------------------
TESTS = [
    ("flat_decode_clean", "flat\ndecode - clean"),
    ("flat_decode_framed", "flat\ndecode - framed"),
    ("flat_encode_clean", "flat\nencode - clean"),
    ("flat_encode_framed", "flat\nencode - framed"),
    ("nested_decode_clean", "nested\ndecode - clean"),
    ("nested_decode_framed", "nested\ndecode - framed"),
    ("nested_encode_clean", "nested\nencode - clean"),
    ("nested_encode_framed", "nested\nencode - framed"),
]
# # bright palette
# APPROACHES = [
#     ("tinyklv", "#00b81d"),
#     ("manual", "#ffcc00"),
#     ("serde_klv", "#ff8c00"),
#     ("tlv_parser", "#ff1f1f"),
# ]

# # grey palette
# APPROACHES = [
#     ("tinyklv", "#00b81d"),
#     ("manual", "#999999"),
#     ("serde_klv", "#999999"),
#     ("tlv_parser", "#999999"),
# ]

# pastel palette
APPROACHES = [
    ("tinyklv", "#00b81d"),
    ("manual", "#f2e2a6"),
    ("serde_klv", "#f6b26b"),
    ("tlv_parser", "#ea9999"),
]

# --------------------------------------------------
# criterion readers: median estimate (bar) and raw samples (box)
# --------------------------------------------------
def median_ns(group: str, approach: str) -> float:
    """Returns the criterion median estimate in ns, or NaN if not measured."""
    path = CRITERION_DIR / group / approach / "new" / "estimates.json"
    if not path.is_file():
        return math.nan
    data = json.loads(path.read_text())
    return float(data["median"]["point_estimate"])


def sample_ns(group: str, approach: str) -> np.ndarray | None:
    """Returns criterion's per-iteration ns samples, or `None` if not measured."""
    path = CRITERION_DIR / group / approach / "new" / "sample.json"
    if not path.is_file():
        return None
    data = json.loads(path.read_text())
    iters = np.asarray(data["iters"], dtype=float)
    times = np.asarray(data["times"], dtype=float)
    return times / iters

# --------------------------------------------------
# grouped geometry: one cluster per group, one element per approach
# --------------------------------------------------
group_pos = np.arange(len(TESTS))
bar_span = 0.8
slot = bar_span / len(APPROACHES)
offsets = (np.arange(len(APPROACHES)) - (len(APPROACHES) - 1) / 2) * slot
draw_w = slot * 0.9

horizontal = ORIENTATION == "h"
box_kind = KIND == "box"
fig, ax = plt.subplots(figsize=(12, 8.5) if horizontal else (15, 6))

all_vals: list[float] = []

for (approach, color), off in zip(APPROACHES, offsets):
    positions = group_pos + off
    if box_kind:
        # --------------------------------------------------
        # box-and-whisker over the raw per-iteration samples
        # --------------------------------------------------
        data, pos = [], []
        for gi, (group, _) in enumerate(TESTS):
            s = sample_ns(group, approach)
            if s is not None and s.size:
                data.append(s)
                pos.append(positions[gi])
                all_vals.extend((float(s.min()), float(s.max())))
        if not data:
            continue
        bp = ax.boxplot(
            data, positions=pos, widths=draw_w, patch_artist=True,
            manage_ticks=False, showfliers=False,
            orientation="horizontal" if horizontal else "vertical",
        )
        for box in bp["boxes"]:
            box.set(facecolor=color, edgecolor="#333333", linewidth=0.6)
        for whisker in bp["whiskers"]:
            whisker.set(color="#333333", linewidth=0.6)
        for cap in bp["caps"]:
            cap.set(color="#333333", linewidth=0.6)
        for median in bp["medians"]:
            median.set(color="#000000", linewidth=1.0)
    else:
        # --------------------------------------------------
        # median grouped bars (with a per-bar ns label)
        # --------------------------------------------------
        values = [median_ns(g, approach) for g, _ in TESTS]
        all_vals.extend(v for v in values if math.isfinite(v))
        if horizontal:
            bars = ax.barh(positions, values, height=draw_w, color=color)
            for bar, v in zip(bars, values):
                if math.isfinite(v):
                    ax.text(v * 1.05, bar.get_y() + bar.get_height() / 2, f"{v:.0f}",
                            va="center", ha="left", fontsize=6)
        else:
            bars = ax.bar(positions, values, width=draw_w, color=color)
            for bar, v in zip(bars, values):
                if math.isfinite(v):
                    ax.text(bar.get_x() + bar.get_width() / 2, v * 1.02, f"{v:.0f}",
                            va="bottom", ha="center", fontsize=6, rotation=90)

# --------------------------------------------------
# axes: log or linear value axis, groups on the category axis
# --------------------------------------------------
log_scale = SCALE != "lin"
value_cap = max(all_vals) * (1.5 if log_scale else 1.12)
value_label = (
    "Time per call (ns, log scale)"
    if log_scale
    else "Time per call (ns)"
)
axis_scale = "log" if log_scale else "linear"

if horizontal:
    ax.set_yticks(group_pos)
    ax.set_yticklabels([lbl.replace("\n", " ") for _, lbl in TESTS])
    ax.set_ylim(group_pos[0] - 0.5, group_pos[-1] + 0.5)
    ax.invert_yaxis()
    ax.set_xscale(axis_scale)
    ax.set_xlim(right=value_cap)
    ax.set_xlabel(value_label)
    ax.grid(True, axis="x", alpha=0.3, which="both")
else:
    ax.set_xticks(group_pos)
    ax.set_xticklabels([lbl for _, lbl in TESTS])
    ax.set_xlim(group_pos[0] - 0.5, group_pos[-1] + 0.5)
    ax.set_yscale(axis_scale)
    ax.set_ylim(top=value_cap)
    ax.set_ylabel(value_label)
    ax.grid(True, axis="y", alpha=0.3, which="both")

ax.set_title(TITLE, fontsize=13)
# custom patch legend (works for both bars and boxes)
handles = [mpatches.Patch(facecolor=c, edgecolor="#333333", label=a) for a, c in APPROACHES]
ax.legend(handles=handles, ncol=len(APPROACHES), loc="upper center",
          bbox_to_anchor=(0.5, -0.08), frameon=False)

plt.tight_layout()
fig.savefig(OUT_PATH, dpi=120, bbox_inches="tight")
print(f"wrote {OUT_PATH}")
