#!/usr/bin/env python3
"""Render the suite's perf charts from criterion's own results.

Usage: plot_times.py <criterion_dir> <orientation> <scale> <kind> <norm> <out_dir>
  criterion_dir — `target/criterion` produced by `cargo bench --bench compsuite`.
  orientation   — "v" (groups on the x-axis) or "h" (groups on the y-axis).
  scale         — "log" (value axis log-scaled) or "lin" (linear).
  kind          — "bar" (median grouped bars) or "box" (box-and-whisker over raw samples).
  norm          — "abs" (raw ns), "byte" (ns per input byte) or "pkt" (ns per decoded element).
  out_dir       — directory the two `bench_*.jpg` files are written to.

Two charts are emitted, one per paradigm, because comparing tinyklv against KLV/TLV libraries and
against protobuf stacks are different stories:
  * `bench_klv.jpg`   — tinyklv vs the hand-written `manual` baseline and the KLV/TLV crates.
  * `bench_proto.jpg` — tinyklv vs `manual` and the four protobuf crates.
`tinyklv` and `manual` appear in both as the shared reference points.

Numbers come straight from criterion — no second timing path. `bar` reads the median
point-estimate from each `new/estimates.json`; `box` reads the raw per-iteration samples from each
`new/sample.json`. Per-bar byte counts come from criterion's persisted `Throughput` in
`new/benchmark.json`; per-tier element counts come from the `suite_meta.json` sidecar the bench
writes (criterion can hold only one throughput metric per bar). lower is faster.
"""

from __future__ import annotations

import json
import math
import sys
from abc import ABC, abstractmethod
from dataclasses import dataclass, field
from enum import Enum
from pathlib import Path

import matplotlib.patches as mpatches
import matplotlib.pyplot as plt
import numpy as np

# --------------------------------------------------
# constants
# --------------------------------------------------
# static pixel gap between a bar's tip and its value label (points), so a long bar and a short bar
# get the same label spacing instead of a value-proportional one
LABEL_OFFSET_PT = 3.0

# value-axis headroom above the tallest bar, per scale
LOG_HEADROOM = 1.5
LIN_HEADROOM = 1.12


# --------------------------------------------------
# approaches
# --------------------------------------------------
class Approach(Enum):
    """A benchmarked implementation; the member value is its criterion function id."""

    TINYKLV = "tinyklv"
    MANUAL = "manual"
    SERDE_KLV = "serde_klv"
    TLV_PARSER = "tlv_parser"
    PROST = "prost"
    QUICK_PROTOBUF = "quick_protobuf"
    RUST_PROTOBUF = "rust_protobuf"
    MICROPB = "micropb"

    @property
    def criterion_id(self) -> str:
        """The directory / function id criterion stores this approach's results under."""
        return self.value

    @property
    def label(self) -> str:
        """The human-facing legend label (currently the criterion id)."""
        return self.value

    @property
    def color(self) -> str:
        """The fill color for this approach's bars / boxes."""
        return _COLORS[self]


# pastel palette: tinyklv bold green, the other KLV approaches warm, the protobuf crates cool
_COLORS: dict[Approach, str] = {
    Approach.TINYKLV: "#00b81d",
    Approach.MANUAL: "#f2e2a6",
    Approach.SERDE_KLV: "#f6b26b",
    Approach.TLV_PARSER: "#ea9999",
    Approach.PROST: "#9fc5e8",
    Approach.QUICK_PROTOBUF: "#6fa8dc",
    Approach.RUST_PROTOBUF: "#b4a7d6",
    Approach.MICROPB: "#a2c4c9",
}

# the two paradigm rosters; tinyklv + manual anchor both
KLV_ROSTER = [Approach.TINYKLV, Approach.MANUAL, Approach.SERDE_KLV, Approach.TLV_PARSER]
PROTO_ROSTER = [
    Approach.TINYKLV,
    Approach.MANUAL,
    Approach.PROST,
    Approach.QUICK_PROTOBUF,
    Approach.RUST_PROTOBUF,
    Approach.MICROPB,
]

# fixed group ordering: three record shapes x five tiers (streamed is decode-only)
TESTS: list[tuple[str, str]] = [
    ("simple_decode_value", "simple\ndecode - value"),
    ("simple_decode_frame", "simple\ndecode - frame"),
    ("simple_decode_streamed", "simple\ndecode - streamed"),
    ("simple_encode_value", "simple\nencode - value"),
    ("simple_encode_frame", "simple\nencode - frame"),
    ("compound_decode_value", "compound\ndecode - value"),
    ("compound_decode_frame", "compound\ndecode - frame"),
    ("compound_decode_streamed", "compound\ndecode - streamed"),
    ("compound_encode_value", "compound\nencode - value"),
    ("compound_encode_frame", "compound\nencode - frame"),
    ("rich_decode_value", "rich\ndecode - value"),
    ("rich_decode_frame", "rich\ndecode - frame"),
    ("rich_decode_streamed", "rich\ndecode - streamed"),
    ("rich_encode_value", "rich\nencode - value"),
    ("rich_encode_frame", "rich\nencode - frame"),
]


# --------------------------------------------------
# criterion reader: the single source of every number the charts draw
# --------------------------------------------------
@dataclass
class CriterionReader:
    """Reads medians, raw samples, and throughput metadata out of a `target/criterion` tree."""

    #: root of the `target/criterion` tree produced by `cargo bench --bench compsuite`
    root: Path

    #: packets decoded per streamed-tier call; loaded from `suite_meta.json`, falling back to 1
    #: when the sidecar is absent (i.e. the streamed tier was not benched)
    stream_packets: int = field(init=False, default=1)

    def __post_init__(self) -> None:
        """Loads the bench-written sidecar holding the per-tier element count (clamped to >= 1)."""
        meta = self.root / "suite_meta.json"
        if meta.is_file():
            count = int(json.loads(meta.read_text()).get("stream_packets", 1))
            self.stream_packets = max(1, count)

    def median_ns(self, group: str, approach: Approach) -> float:
        """Returns the criterion median estimate in ns, or NaN if not measured."""
        path = self.root / group / approach.criterion_id / "new" / "estimates.json"
        if not path.is_file():
            return math.nan
        return float(json.loads(path.read_text())["median"]["point_estimate"])

    def sample_ns(self, group: str, approach: Approach) -> np.ndarray | None:
        """Returns criterion's per-iteration ns samples (times / iters), or None if not measured."""
        path = self.root / group / approach.criterion_id / "new" / "sample.json"
        if not path.is_file():
            return None
        data = json.loads(path.read_text())
        iters = np.asarray(data["iters"], dtype=float)
        times = np.asarray(data["times"], dtype=float)
        return times / iters

    def input_bytes(self, group: str, approach: Approach) -> float:
        """Returns the per-bar input byte count from criterion's `Throughput`, or NaN if absent."""
        path = self.root / group / approach.criterion_id / "new" / "benchmark.json"
        if not path.is_file():
            return math.nan
        throughput = json.loads(path.read_text()).get("throughput")
        if not isinstance(throughput, dict) or "Bytes" not in throughput:
            return math.nan
        return float(throughput["Bytes"])

    def elements(self, group: str) -> int:
        """Returns the number of decoded elements per call for `group` (streamed tier vs the rest)."""
        return self.stream_packets if group.endswith("_streamed") else 1


# --------------------------------------------------
# normalization: how a raw ns value is rescaled before plotting
# --------------------------------------------------
class Normalization(ABC):
    """Strategy that rescales raw ns into the chart's chosen unit."""

    #: value-axis unit suffix, e.g. "ns" or "ns/byte"
    unit: str

    @abstractmethod
    def divisor(self, reader: CriterionReader, group: str, approach: Approach) -> float:
        """Returns the per-(group, approach) divisor applied to every ns value."""

    def safe_divisor(self, reader: CriterionReader, group: str, approach: Approach) -> float:
        """Returns [`divisor`] when it is finite and non-zero, else `math.nan`

        Centralises the divide-by-zero / not-measured guard so the bar (scalar) and box (array)
        paths share one definition of "this bar cannot be normalized".
        """
        value = self.divisor(reader, group, approach)
        return value if math.isfinite(value) and value != 0 else math.nan


class Absolute(Normalization):
    """Raw per-call time; no rescaling."""

    unit = "ns"

    def divisor(self, reader: CriterionReader, group: str, approach: Approach) -> float:
        """Always 1 - values are left in raw ns."""
        return 1.0


class PerByte(Normalization):
    """Per-input-byte time, using criterion's persisted byte throughput."""

    unit = "ns/byte"

    def divisor(self, reader: CriterionReader, group: str, approach: Approach) -> float:
        """The per-bar input byte count."""
        return reader.input_bytes(group, approach)


class PerElement(Normalization):
    """Per-decoded-element time, dividing the streamed tier by its packet count."""

    unit = "ns/element"

    def divisor(self, reader: CriterionReader, group: str, approach: Approach) -> float:
        """The per-tier element count (packets for the streamed tier, 1 otherwise)."""
        return float(reader.elements(group))


# --------------------------------------------------
# layout: everything that differs between a vertical and a horizontal chart
# --------------------------------------------------
class Layout(ABC):
    """Orientation strategy: bar drawing, label placement, and axis setup."""

    @abstractmethod
    def figsize(self) -> tuple[float, float]:
        """Returns the figure size tuned for this orientation."""

    @abstractmethod
    def draw_bars(self, ax, positions, values, width, color):
        """Draws the grouped bars and returns the bar container."""

    @abstractmethod
    def annotate(self, ax, bar, value: float, precision: int) -> None:
        """Places a value label (at `precision` decimals) a fixed pixel offset off the bar's tip."""

    @property
    @abstractmethod
    def boxplot_orientation(self) -> str:
        """The matplotlib boxplot orientation string for this layout."""

    @abstractmethod
    def setup_axes(self, ax, group_pos, labels, scale_name, cap, value_label) -> None:
        """Configures the category axis (groups) and value axis (scale, limit, label)."""


class Vertical(Layout):
    """Groups along the x-axis, values rising on the y-axis."""

    def figsize(self) -> tuple[float, float]:
        """Wide and short - many group clusters across the x-axis."""
        return (18, 6)

    def draw_bars(self, ax, positions, values, width, color):
        """Vertical bars via `ax.bar`."""
        return ax.bar(positions, values, width=width, color=color)

    def annotate(self, ax, bar, value: float, precision: int) -> None:
        """Label above the bar top, rotated, with a static vertical pixel gap."""
        ax.annotate(
            _fmt(value, precision),
            xy=(bar.get_x() + bar.get_width() / 2, value),
            xytext=(0, LABEL_OFFSET_PT),
            textcoords="offset points",
            va="bottom", ha="center", fontsize=6, rotation=90,
        )

    @property
    def boxplot_orientation(self) -> str:
        """Boxes stand vertically."""
        return "vertical"

    def setup_axes(self, ax, group_pos, labels, scale_name, cap, value_label) -> None:
        """Group ticks on x; scaled, capped, labelled value axis on y."""
        ax.set_xticks(group_pos)
        ax.set_xticklabels(labels)
        ax.set_xlim(group_pos[0] - 0.5, group_pos[-1] + 0.5)
        ax.set_yscale(scale_name)
        ax.set_ylim(top=cap)
        ax.set_ylabel(value_label)
        ax.grid(True, axis="y", alpha=0.3, which="both")


class Horizontal(Layout):
    """Groups along the y-axis, values extending on the x-axis."""

    def figsize(self) -> tuple[float, float]:
        """Tall and narrow - group clusters stacked down the y-axis."""
        return (12, 13)

    def draw_bars(self, ax, positions, values, width, color):
        """Horizontal bars via `ax.barh`."""
        return ax.barh(positions, values, height=width, color=color)

    def annotate(self, ax, bar, value: float, precision: int) -> None:
        """Label past the bar end with a static horizontal pixel gap."""
        ax.annotate(
            _fmt(value, precision),
            xy=(value, bar.get_y() + bar.get_height() / 2),
            xytext=(LABEL_OFFSET_PT, 0),
            textcoords="offset points",
            va="center", ha="left", fontsize=6,
        )

    @property
    def boxplot_orientation(self) -> str:
        """Boxes lie horizontally."""
        return "horizontal"

    def setup_axes(self, ax, group_pos, labels, scale_name, cap, value_label) -> None:
        """Group ticks on y (top-down); scaled, capped, labelled value axis on x."""
        ax.set_yticks(group_pos)
        ax.set_yticklabels([lbl.replace("\n", " ") for lbl in labels])
        ax.set_ylim(group_pos[0] - 0.5, group_pos[-1] + 0.5)
        ax.invert_yaxis()
        ax.set_xscale(scale_name)
        ax.set_xlim(right=cap)
        ax.set_xlabel(value_label)
        ax.grid(True, axis="x", alpha=0.3, which="both")


# --------------------------------------------------
# series renderer: how one approach's per-group data is drawn (median bars vs raw-sample boxes)
# --------------------------------------------------
class SeriesRenderer(ABC):
    """Strategy that draws one approach across every group and returns the values it plotted."""

    @abstractmethod
    def draw(self, ax, layout, reader, normalization, approach, positions, width, groups, precision) -> list[float]:
        """Draws this approach's series across `groups` and returns the finite plotted values

        `groups` is the ordered list of criterion group ids aligned with `positions`; the renderer
        reads its data through `reader` rather than knowing the global group roster. `precision` is
        the chart-wide label precision (ignored by renderers that draw no labels). The returned
        finite values are used by the caller to size the value-axis cap.
        """


class BarRenderer(SeriesRenderer):
    """Median grouped bars with a per-bar value label."""

    def draw(self, ax, layout, reader, normalization, approach, positions, width, groups, precision) -> list[float]:
        """Draws one normalized median bar per group and labels each at `precision` decimals."""
        # --------------------------------------------------
        # normalize each group's median for this approach
        # --------------------------------------------------
        values = [
            _normalize(reader.median_ns(g, approach), reader, normalization, g, approach)
            for g in groups
        ]
        # --------------------------------------------------
        # draw bars and annotate the measured ones
        # --------------------------------------------------
        bars = layout.draw_bars(ax, positions, values, width, approach.color)
        for bar, value in zip(bars, values):
            if math.isfinite(value):
                layout.annotate(ax, bar, value, precision)
        return [v for v in values if math.isfinite(v)]


class BoxRenderer(SeriesRenderer):
    """Box-and-whisker over criterion's raw per-iteration samples (draws no value labels)."""

    def draw(self, ax, layout, reader, normalization, approach, positions, width, groups, precision) -> list[float]:
        """Draws one box per group from the normalized sample distribution (`precision` unused)."""
        # --------------------------------------------------
        # collect normalized sample arrays for the measured groups
        # --------------------------------------------------
        data, pos, collected = [], [], []
        for index, group in enumerate(groups):
            samples = reader.sample_ns(group, approach)
            if samples is None or not samples.size:
                continue
            divisor = normalization.safe_divisor(reader, group, approach)
            if not math.isfinite(divisor):
                continue
            scaled = samples / divisor
            data.append(scaled)
            pos.append(positions[index])
            collected.extend((float(scaled.min()), float(scaled.max())))
        # --------------------------------------------------
        # draw the boxes and apply styling
        # --------------------------------------------------
        if data:
            self._style(ax.boxplot(
                data, positions=pos, widths=width, patch_artist=True,
                manage_ticks=False, showfliers=False,
                orientation=layout.boxplot_orientation,
            ), approach.color)
        return collected

    @staticmethod
    def _style(bp, color: str) -> None:
        """Applies the shared styling to a matplotlib boxplot return dict

        `bp` is the dict returned by `ax.boxplot` (keys `boxes`, `whiskers`, `caps`, `medians`);
        `color` fills the boxes, edges/whiskers/caps are dark grey, and the median line is black.
        """
        for box in bp["boxes"]:
            box.set(facecolor=color, edgecolor="#333333", linewidth=0.6)
        for part in ("whiskers", "caps"):
            for artist in bp[part]:
                artist.set(color="#333333", linewidth=0.6)
        for median in bp["medians"]:
            median.set(color="#000000", linewidth=1.0)


# total category width one group's bars share, and the fraction of a slot a bar actually fills
_BAR_SPAN = 0.8
_SLOT_FILL = 0.9


def _normalize(
    raw_ns: float, reader: CriterionReader, normalization: Normalization, group: str, approach: Approach
) -> float:
    """Applies a normalization divisor to a single raw ns value

    Returns `raw_ns / divisor` when `raw_ns` is finite and the guarded divisor is valid; returns
    `math.nan` otherwise - i.e. when this approach was not measured for this group, or the divisor
    is non-finite or zero (see [`Normalization.safe_divisor`]).
    """
    divisor = normalization.safe_divisor(reader, group, approach)
    if not math.isfinite(raw_ns) or not math.isfinite(divisor):
        return math.nan
    return raw_ns / divisor


def _precision(values: list[float]) -> int:
    """Chooses one label precision for a whole chart from its set of displayed values

    The precision is uniform across the chart (not per-bar) so the labels read consistently:
    2 decimals when any value is below 5, otherwise 1 decimal when any is below 15, otherwise
    whole numbers. Empty input yields 0.
    """
    if not values:
        return 0
    smallest = min(values)
    if smallest < 5:
        return 2
    if smallest < 15:
        return 1
    return 0


def _fmt(value: float, precision: int) -> str:
    """Formats a bar value label at the chart-wide `precision`."""
    return f"{value:.{precision}f}"


# --------------------------------------------------
# chart: one figure for one roster of approaches
# --------------------------------------------------
@dataclass
class Chart:
    """A single grouped chart over one roster of approaches."""

    reader: CriterionReader
    approaches: list[Approach]
    tests: list[tuple[str, str]]
    layout: Layout
    renderer: SeriesRenderer
    normalization: Normalization
    log_scale: bool
    title: str
    out_path: Path

    def render(self) -> None:
        """Draws every approach, configures the axes and legend, and writes the image."""
        # --------------------------------------------------
        # grouped geometry: one cluster per group, one slot per approach
        # --------------------------------------------------
        group_ids = [group for group, _ in self.tests]
        group_pos = np.arange(len(self.tests))
        slot = _BAR_SPAN / len(self.approaches)
        offsets = (np.arange(len(self.approaches)) - (len(self.approaches) - 1) / 2) * slot
        draw_width = slot * _SLOT_FILL
        # --------------------------------------------------
        # one label precision for the whole chart, from every approach's normalized median
        # --------------------------------------------------
        medians = [
            _normalize(self.reader.median_ns(g, a), self.reader, self.normalization, g, a)
            for a in self.approaches for g in group_ids
        ]
        precision = _precision([v for v in medians if math.isfinite(v)])
        # --------------------------------------------------
        # draw each approach's series, collecting plotted values for axis sizing
        # --------------------------------------------------
        fig, ax = plt.subplots(figsize=self.layout.figsize())
        all_vals: list[float] = []
        for approach, off in zip(self.approaches, offsets):
            all_vals.extend(
                self.renderer.draw(ax, self.layout, self.reader, self.normalization,
                                   approach, group_pos + off, draw_width, group_ids, precision)
            )
        # --------------------------------------------------
        # axes: scaled value axis with a little headroom over the tallest value
        # --------------------------------------------------
        headroom = LOG_HEADROOM if self.log_scale else LIN_HEADROOM
        cap = (max(all_vals) if all_vals else 1.0) * headroom
        scale_name = "log" if self.log_scale else "linear"
        suffix = f"{self.normalization.unit}, log scale" if self.log_scale else self.normalization.unit
        self.layout.setup_axes(
            ax, group_pos, [lbl for _, lbl in self.tests], scale_name, cap, f"Time per call ({suffix})"
        )
        # --------------------------------------------------
        # title, legend, save
        # --------------------------------------------------
        ax.set_title(self.title, fontsize=13)
        handles = [
            mpatches.Patch(facecolor=a.color, edgecolor="#333333", label=a.label)
            for a in self.approaches
        ]
        ax.legend(handles=handles, ncol=len(self.approaches), loc="upper center",
                  bbox_to_anchor=(0.5, -0.08), frameon=False)
        fig.tight_layout()
        fig.savefig(self.out_path, dpi=120, bbox_inches="tight")
        plt.close(fig)
        print(f"wrote {self.out_path}")


# --------------------------------------------------
# cli wiring: select strategies by name, then render both paradigm charts
# --------------------------------------------------
_LAYOUTS: dict[str, Layout] = {"v": Vertical(), "h": Horizontal()}
_RENDERERS: dict[str, SeriesRenderer] = {"bar": BarRenderer(), "box": BoxRenderer()}
_NORMALIZATIONS: dict[str, Normalization] = {"abs": Absolute(), "byte": PerByte(), "pkt": PerElement()}


def main() -> None:
    """Parses CLI knobs and renders `bench_klv.jpg` and `bench_proto.jpg`."""
    # --------------------------------------------------
    # parse + validate knobs (print usage on a bad arg count or unknown knob)
    # --------------------------------------------------
    if len(sys.argv) < 7:
        sys.exit(__doc__)
    criterion_dir, orientation, scale, kind, norm, out_dir = sys.argv[1:7]
    try:
        layout = _LAYOUTS[orientation.lower()]
        renderer = _RENDERERS[kind.lower()]
        normalization = _NORMALIZATIONS[norm.lower()]
    except KeyError as bad:
        sys.exit(f"unknown knob {bad}; see usage:\n{__doc__}")
    reader = CriterionReader(Path(criterion_dir))
    log_scale = scale.lower() != "lin"
    out = Path(out_dir)
    out.mkdir(parents=True, exist_ok=True)
    # --------------------------------------------------
    # render one chart per paradigm roster
    # --------------------------------------------------
    for roster, stem, paradigm in (
        (KLV_ROSTER, "bench_klv", "KLV / TLV frameworks"),
        (PROTO_ROSTER, "bench_proto", "tinyklv vs protobuf stacks"),
    ):
        Chart(
            reader=reader,
            approaches=roster,
            tests=TESTS,
            layout=layout,
            renderer=renderer,
            normalization=normalization,
            log_scale=log_scale,
            title=f"{paradigm} - per-call time ({normalization.unit}, lower is better)",
            out_path=out / f"{stem}.jpg",
        ).render()


if __name__ == "__main__":
    main()
