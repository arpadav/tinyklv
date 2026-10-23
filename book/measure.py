#!/usr/bin/env python3
# /// script
# requires-python = ">=3.11"
# dependencies = []
# ///
"""Page checker for the tinyklv book, run on the rendered html.

Run as `python3 book/measure.py <book-root>`, and with `-h` for the flags. `<book-root>` holds
`book.toml` and `src/SUMMARY.md`. The script runs `mdbook build` into a temp dir, parses each SUMMARY
page's `<main>` into prose, code and table blocks, and checks that model. The markdown is searched
only to turn a hit's first rendered line into `file:line`. Every hit prints as `where: message`, then
one summary line per check closes the output. Stdlib only.
"""

from __future__ import annotations

import argparse
import os
import re
import shutil
import subprocess
import sys
import tempfile
from dataclasses import dataclass, field
from enum import IntEnum, StrEnum, auto
from functools import cached_property
from html.parser import HTMLParser
from pathlib import Path
from typing import TYPE_CHECKING, ClassVar, Final, NamedTuple, NoReturn, Protocol, final

if TYPE_CHECKING:
    from collections.abc import Sequence


class Exit(IntEnum):
    """Process exit codes."""

    OK = 0
    FAILED = 1
    USAGE = 2
    NO_MDBOOK = 3


class Flag(StrEnum):
    """Command-line flags."""

    FINAL = "--final"

    @property
    def help(self) -> str:
        """The flag's help line."""
        match self:
            case Flag.FINAL:
                return "also fail on stubs, orphan examples and a wrong page count"


class Section(StrEnum):
    """Which part of the book a page belongs to."""

    TUTORIAL = "tutorial"
    REFERENCE = "reference"
    OTHER = "other"

    @classmethod
    def of(cls, rel: Path) -> Section:
        """The part of the book the page at `rel` (relative to `src`) belongs to."""
        match rel.parts[0]:
            case cls.TUTORIAL:
                return cls.TUTORIAL
            case cls.REFERENCE:
                return cls.REFERENCE
            case _:
                return cls.OTHER


class PageName(StrEnum):
    """File names that mdbook treats specially."""

    README = "README.md"
    INDEX = "index.html"


class Tag(StrEnum):
    """The html tags the parser acts on."""

    P = "p"
    LI = "li"
    H1 = "h1"
    H2 = "h2"
    H3 = "h3"
    H4 = "h4"
    H5 = "h5"
    H6 = "h6"
    PRE = "pre"
    CODE = "code"
    TABLE = "table"
    SUMMARY = "summary"
    BLOCKQUOTE = "blockquote"
    MAIN = "main"

    @classmethod
    def headings(cls) -> frozenset[Tag]:
        """`h1` to `h6`."""
        return frozenset({cls.H1, cls.H2, cls.H3, cls.H4, cls.H5, cls.H6})

    @classmethod
    def prose(cls) -> frozenset[Tag]:
        """The elements whose text is prose."""
        return cls.headings() | {cls.P, cls.LI}


class CssClass(StrEnum):
    """The css classes the parser acts on."""

    CALLOUT = "blockquote-tag"
    CALLOUT_TITLE = "blockquote-tag-title"
    LANGUAGE = "language-"


class CheckName(StrEnum):
    """The checks."""

    BUILD = "build"
    PROSE = "prose words"
    RUN_ON = "run-on sentences"
    CODE = "code in prose"
    STUBS = "stubs"
    ORPHANS = "orphan examples"
    PAGES = "page count"


class Verdict(StrEnum):
    """A check's outcome."""

    PASS = "PASS"  # noqa: S105
    FAIL = "FAIL"
    INFO = "INFO"


class Kind(StrEnum):
    """What an open html element is for while parsing."""

    PROSE = auto()
    SKIPPED = auto()
    PRE = auto()
    BLOCK = auto()
    INLINE = auto()
    TABLE = auto()
    OTHER = auto()


@final
class Limits:
    """Hard limits: a page, entry or sentence over one fails."""

    TUTORIAL_PAGE_WORDS: Final = 150
    REFERENCE_ENTRY_WORDS: Final = 80
    MAX_CODE_SPANS: Final = 2


@final
class Targets:
    """Plan targets for the whole book; the word total counts the teaching pages, the reference is held per entry."""

    TOTAL_WORDS: Final = 4800
    TUTORIAL_PAGES: Final = 34


@final
class UsageError(Exception):
    """A bad command line."""


@final
class Text:
    """Shared text helpers."""

    COMMENTS: Final = re.compile(r"//[^\n]*|/\*.*?\*/", re.DOTALL)

    @staticmethod
    def read(path: Path) -> str:
        """Read a UTF-8 file."""
        return path.read_text(encoding="utf-8")

    @staticmethod
    def normalize(text: str) -> str:
        """Collapse whitespace runs and trim."""
        return " ".join(text.split())

    @classmethod
    def strip_comments(cls, source: str) -> str:
        """Rust `source` without its comments."""
        return cls.COMMENTS.sub("", source)

    @classmethod
    def preview(cls, text: str, width: int = 60) -> str:
        """The first `width` characters of `text`, marked when cut."""
        flat = cls.normalize(text)
        return flat if len(flat) <= width else flat[: width - 3] + "..."

    @staticmethod
    def noun(count: int, one: str, many: str | None = None) -> str:
        """`count` with `one` when it is 1, else with `many` (default: `one` + "s")."""
        return f"{count} {one if count == 1 else many or one + 's'}"


@final
@dataclass(frozen=True)
class BookPaths:
    """Every path the script touches; all absolute, because `root` is resolved once."""

    root: Path
    cwd: Path

    @classmethod
    def at(cls, raw: Path) -> BookPaths:
        """The paths of the book rooted at `raw`, resolved against the current directory."""
        return cls(root=raw.resolve(), cwd=Path.cwd().resolve())

    @property
    def src(self) -> Path:
        """The book's markdown sources."""
        return self.root / "src"

    @property
    def summary(self) -> Path:
        """The table of contents."""
        return self.src / "SUMMARY.md"

    @property
    def book_toml(self) -> Path:
        """The book's configuration."""
        return self.root / "book.toml"

    @property
    def examples(self) -> Path:
        """The book's runnable examples, a sibling `examples/<root name>`."""
        return self.root.parent / "examples" / self.root.name

    def example_files(self) -> list[Path]:
        """Every example source file, in path order."""
        return sorted(self.examples.rglob("*.rs"))

    def page_md(self, rel: Path) -> Path:
        """The markdown source of the page at `rel` (relative to `src`)."""
        return self.src / rel

    @staticmethod
    def page_html(out: Path, rel: Path) -> Path:
        """Where mdbook writes the page at `rel` under `out`: a `README.md` becomes `index.html`."""
        if rel.name == PageName.README:
            return out / rel.with_name(PageName.INDEX)
        return out / rel.with_suffix(".html")

    def problem(self) -> str | None:
        """Why `root` is not a book root, or `None`."""
        if not self.root.is_dir():
            return f"not a directory: {self.root}"
        if not (self.book_toml.is_file() and self.summary.is_file()):
            return f"not a book root (needs book.toml and src/SUMMARY.md): {self.root}"
        return None

    def shown(self, path: Path) -> str:
        """`path` relative to the current directory when it sits under it."""
        try:
            return str(path.relative_to(self.cwd))
        except ValueError:
            return str(path)


@final
@dataclass(frozen=True)
class Hit:
    """One finding, printed as `where: message`."""

    where: str
    message: str

    def __str__(self) -> str:
        """The hit as one output line."""
        return f"{self.where}: {self.message}"


@final
@dataclass(frozen=True)
class Finding:
    """What a check found, before the verdict."""

    summary: str
    hits: tuple[Hit, ...]


@final
@dataclass(frozen=True)
class CheckResult:
    """A check's finding with its verdict."""

    name: CheckName
    verdict: Verdict
    summary: str
    hits: tuple[Hit, ...]


class Span(NamedTuple):
    """Offsets of an inline code span in a prose block's text."""

    start: int
    end: int


class Worst(NamedTuple):
    """The largest word count seen and where."""

    words: int
    label: str


@final
class TextBuilder:
    """Accumulates one element's text with whitespace collapsed and inline-code spans recorded."""

    def __init__(self) -> None:
        """Start empty."""
        self.text = ""
        self.spans: list[Span] = []

    def add(self, data: str) -> None:
        """Append text, collapsing whitespace against what is already there."""
        if not data:
            return
        flat = " ".join(data.split())
        piece = flat
        if data[0].isspace() and self.text and not self.text.endswith(" "):
            piece = " " + piece
        if data[-1].isspace() and flat:
            piece += " "
        self.text += piece

    def add_code(self, code: str) -> None:
        """Append an inline code span."""
        flat = Text.normalize(code)
        self.spans.append(Span(len(self.text), len(self.text) + len(flat)))
        self.text += flat

    def done(self) -> str:
        """The text, trimmed."""
        return self.text.strip()


@final
@dataclass(frozen=True)
class Sentence:
    """A sentence of a prose block and the inline code spans in it."""

    marked: str
    span_count: int


@final
@dataclass(frozen=True)
class Prose:
    """Text under one `p`, `li` or heading, with its inline code spans."""

    tag: str
    text: str
    spans: tuple[Span, ...]
    in_note: bool = False

    END: ClassVar[re.Pattern[str]] = re.compile(r"[.!?]\s+")
    ABBREVIATION: ClassVar[re.Pattern[str]] = re.compile(r"(?:^|\W)(?:e\.g|i\.e)$", re.IGNORECASE)

    @cached_property
    def words(self) -> int:
        """Whitespace-separated words, inline code included."""
        return len(self.text.split())

    def marked(self, begin: int = 0, end: int | None = None) -> str:
        """`text[begin:end]` with each inline code span re-wrapped in backticks."""
        stop = len(self.text) if end is None else end
        out: list[str] = []
        at = begin
        for span in self.spans:
            if span.end <= begin or span.start >= stop:
                continue
            first, last = max(span.start, begin), min(span.end, stop)
            out.append(self.text[at:first])
            out.append("`" + self.text[first:last] + "`")
            at = last
        out.append(self.text[at:stop])
        return "".join(out)

    @cached_property
    def sentences(self) -> tuple[Sentence, ...]:
        """Sentences ended by `.`, `!` or `?` and whitespace; `e.g.` and `i.e.` never end one."""
        flat = list(self.text)
        for span in self.spans:
            flat[span.start : span.end] = "x" * (span.end - span.start)
        masked = "".join(flat)
        bounds: list[Span] = []
        start = 0
        for match in self.END.finditer(masked):
            if self.ABBREVIATION.search(masked[: match.start()]):
                continue
            bounds.append(Span(start, match.end()))
            start = match.end()
        bounds.append(Span(start, len(masked)))
        return tuple(
            Sentence(self.marked(b.start, b.end).strip(), sum(1 for s in self.spans if b.start <= s.start < b.end))
            for b in bounds
            if self.text[b.start : b.end].strip()
        )


@final
@dataclass(frozen=True)
class Code:
    """A fenced code block as rendered, with its lowercase language."""

    lang: str
    text: str


@final
@dataclass(frozen=True)
class Table:
    """A table; it counts as page content, its cells are not prose."""


Block = Prose | Code | Table


@final
@dataclass(frozen=True)
class Entry:
    """A reference `h2`/`h3` section, or the text above the first `h2`."""

    heading: Prose | None
    parent: Prose | None
    first: int
    words: int

    @property
    def label(self) -> str:
        """The entry's heading, led by its `h2` for an `h3`, or a name for the page top."""
        if self.heading is None:
            return "(page top)"
        if self.parent is None:
            return self.heading.marked()
        return f"{self.parent.marked()} > {self.heading.marked()}"


@final
@dataclass(frozen=True)
class RenderedPage:
    """One book page: its source md and the blocks of its rendered `<main>`."""

    rel: Path
    md: Path
    shown: str
    blocks: tuple[Block, ...]
    stray: tuple[str, ...]

    LINK: ClassVar[re.Pattern[str]] = re.compile(r"!?\[([^\]]*)\]\([^)]*\)")
    EMPHASIS: ClassVar[re.Pattern[str]] = re.compile(r"(`+[^`]*`+)|\*\*|__|(?<![\w`])[*_]|[*_](?![\w`])")
    KEY_WIDTHS: ClassVar[tuple[int, ...]] = (60, 30, 16)
    MIN_KEY: ClassVar[int] = 4

    @property
    def section(self) -> Section:
        """The part of the book the page is in."""
        return Section.of(self.rel)

    @property
    def is_part_readme(self) -> bool:
        """A tutorial part's `README.md`."""
        return self.section is Section.TUTORIAL and self.rel.name == PageName.README

    @cached_property
    def prose(self) -> tuple[Prose, ...]:
        """Every prose block, in document order."""
        return tuple(b for b in self.blocks if isinstance(b, Prose))

    @cached_property
    def words(self) -> int:
        """Prose words on the page; a note box's words are not counted, it is where English belongs."""
        return sum(p.words for p in self.prose if not p.in_note)

    @cached_property
    def rust(self) -> tuple[Code, ...]:
        """Every rust block on the page."""
        return tuple(b for b in self.blocks if isinstance(b, Code) and b.lang == "rust")

    @property
    def is_stub(self) -> bool:
        """No block beyond one heading."""
        if not self.blocks:
            return True
        only = self.blocks[0]
        return len(self.blocks) == 1 and isinstance(only, Prose) and only.tag in Tag.headings()

    @cached_property
    def entries(self) -> tuple[Entry, ...]:
        """Sections split at `h2`/`h3`; the text above the first `h2` is an entry too."""
        entries: list[Entry] = []
        heading: Prose | None = None
        parent: Prose | None = None
        section: Prose | None = None
        first = 0
        words = 0

        def close() -> None:
            if words:
                entries.append(Entry(heading, parent, first, words))

        for index, block in enumerate(self.prose):
            if block.tag in {Tag.H2, Tag.H3}:
                close()
                if block.tag == Tag.H2:
                    section = block
                parent = section if block.tag == Tag.H3 else None
                heading, first, words = block, index, 0
            if not block.in_note:
                words += block.words
        close()
        return tuple(entries)

    @cached_property
    def source_lines(self) -> tuple[str, ...]:
        """The page's markdown lines with emphasis and link syntax removed, whitespace-normalized."""
        return tuple(
            Text.normalize(self.EMPHASIS.sub(lambda m: m[1] or "", self.LINK.sub(r"\1", line)))
            for line in Text.read(self.md).splitlines()
        )

    @cached_property
    def prose_lines(self) -> tuple[int | None, ...]:
        """For each prose block, the md line it starts on, searching forward in document order."""
        found: list[int | None] = []
        cursor = 0
        for block in self.prose:
            line = self._find(block.marked(), cursor)
            found.append(line)
            if line is not None:
                cursor = line
        return tuple(found)

    def _find(self, anchor: str, start: int) -> int | None:
        """The first md line at or after 0-based index `start` holding the start of `anchor`."""
        flat = Text.normalize(anchor)
        for width in self.KEY_WIDTHS:
            key = flat[:width]
            if len(key) < self.MIN_KEY:
                return None
            for index in range(start, len(self.source_lines)):
                if key in self.source_lines[index]:
                    return index + 1
        return None

    def where(self, index: int, anchor: str | None = None) -> str:
        """`file:line` of `anchor` inside prose block `index` (default: the block), else the block's line."""
        block_line = self.prose_lines[index]
        text = anchor if anchor is not None else self.prose[index].marked()
        line = self._find(text, 0 if block_line is None else block_line - 1) or block_line
        return self._locate(line, text)

    def where_text(self, text: str) -> str:
        """`file:line` of `text` anywhere on the page."""
        return self._locate(self._find(text, 0), text)

    def _locate(self, line: int | None, text: str) -> str:
        if line is None:
            return f'{self.shown}: "{Text.preview(text, 40)}"'
        return f"{self.shown}:{line}"

    def page_where(self) -> str:
        """The location of the page as a whole."""
        return f"{self.shown}:1"


@final
@dataclass
class Frame:
    """An open element during parsing."""

    kind: Kind
    tag: str
    lang: str = ""
    slot: int = -1
    note: bool = False
    buf: TextBuilder = field(default_factory=TextBuilder)
    raw: list[str] = field(default_factory=list)


@final
class PageParser(HTMLParser):
    """Turns the `<main>` of a rendered page into blocks."""

    VOID: ClassVar[frozenset[str]] = frozenset(
        {"area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "source", "track", "wbr"}
    )

    def __init__(self) -> None:
        """Start with no open elements."""
        super().__init__(convert_charrefs=True)
        self.in_main = False
        self.frames: list[Frame] = []
        self.slots: list[Block | None] = []
        self.stray: list[str] = []

    @property
    def blocks(self) -> tuple[Block, ...]:
        """The page's blocks in document order."""
        return tuple(b for b in self.slots if b is not None)

    @staticmethod
    def _classes(attrs: list[tuple[str, str | None]]) -> list[str]:
        default: list[str] = []
        return next((value.split() for name, value in attrs if name == "class" and value), default)

    def _classify(self, tag: str, attrs: list[tuple[str, str | None]]) -> Frame:
        classes = self._classes(attrs)
        if tag == Tag.CODE:
            if self.frames and self.frames[-1].kind is Kind.PRE:
                lang = next((c.removeprefix(CssClass.LANGUAGE) for c in classes if c.startswith(CssClass.LANGUAGE)), "")
                return Frame(Kind.BLOCK, tag, lang.lower())
            return Frame(Kind.INLINE, tag)
        if tag == Tag.SUMMARY or (tag == Tag.P and CssClass.CALLOUT_TITLE in classes):
            return Frame(Kind.SKIPPED, tag)
        if tag == Tag.BLOCKQUOTE and CssClass.CALLOUT in classes:
            return Frame(Kind.OTHER, tag, note=True)
        if tag in Tag.prose():
            return Frame(Kind.PROSE, tag)
        if tag == Tag.PRE:
            return Frame(Kind.PRE, tag)
        if tag == Tag.TABLE:
            return Frame(Kind.TABLE, tag)
        return Frame(Kind.OTHER, tag)

    def handle_starttag(self, tag: str, attrs: list[tuple[str, str | None]]) -> None:
        """Open an element; only `<main>` content is modelled."""
        if tag == Tag.MAIN:
            self.in_main = True
            return
        if not self.in_main or tag in self.VOID:
            return
        frame = self._classify(tag, attrs)
        if frame.kind in {Kind.PROSE, Kind.BLOCK, Kind.TABLE}:
            frame.slot = len(self.slots)
            self.slots.append(None)
        self.frames.append(frame)

    def handle_endtag(self, tag: str) -> None:
        """Close an element, and any element left open inside it."""
        if tag == Tag.MAIN:
            self.in_main = False
            return
        if not self.in_main:
            return
        for index in range(len(self.frames) - 1, -1, -1):
            if self.frames[index].tag == tag:
                while len(self.frames) > index:
                    self._close(self.frames.pop())
                return

    def handle_data(self, data: str) -> None:
        """Route text to the innermost element that owns it; text nobody owns is stray."""
        if not self.in_main:
            return
        for frame in reversed(self.frames):
            match frame.kind:
                case Kind.SKIPPED | Kind.TABLE | Kind.PRE:
                    return
                case Kind.BLOCK:
                    frame.raw.append(data)
                    return
                case Kind.INLINE | Kind.PROSE:
                    frame.buf.add(data)
                    return
                case Kind.OTHER:
                    continue
        if data.strip():
            self.stray.append(Text.normalize(data))

    def _close(self, frame: Frame) -> None:
        match frame.kind:
            case Kind.PROSE:
                text = frame.buf.done()
                if text:
                    in_note = any(outer.note for outer in self.frames)
                    self.slots[frame.slot] = Prose(frame.tag, text, tuple(frame.buf.spans), in_note)
            case Kind.BLOCK:
                self.slots[frame.slot] = Code(frame.lang, "".join(frame.raw))
            case Kind.TABLE:
                self.slots[frame.slot] = Table()
            case Kind.INLINE:
                self._close_inline(frame)
            case _:
                pass

    def _close_inline(self, frame: Frame) -> None:
        """Hand an inline code span to the prose block around it; spans elsewhere are dropped."""
        for outer in reversed(self.frames):
            match outer.kind:
                case Kind.PROSE:
                    outer.buf.add_code(frame.buf.done())
                    return
                case Kind.SKIPPED | Kind.TABLE:
                    return
                case _:
                    continue


@final
@dataclass(frozen=True)
class Example:
    """An example source file."""

    path: Path
    text: str

    STUB_MAIN: ClassVar[re.Pattern[str]] = re.compile(r"fn\s+main\s*\(\s*\)\s*\{\s*\}")
    ANCHOR: ClassVar[re.Pattern[str]] = re.compile(r"^.*\bANCHOR(?:_END)?:.*\n?", re.MULTILINE)

    @property
    def rendered(self) -> str:
        """The text mdbook shows for the whole file: its `ANCHOR` lines dropped, no trailing newline."""
        return self.ANCHOR.sub("", self.text).rstrip("\n")

    @property
    def is_stub(self) -> bool:
        """Whether the code is still an empty `fn main() {}`."""
        return bool(self.STUB_MAIN.search(Text.strip_comments(self.text)))


@final
@dataclass(frozen=True)
class Book:
    """The rendered pages in SUMMARY order, mdbook's diagnostics, and the example files."""

    paths: BookPaths
    pages: tuple[RenderedPage, ...]
    build_log: tuple[Hit, ...]

    def pages_in(self, section: Section) -> tuple[RenderedPage, ...]:
        """Pages of one section."""
        return tuple(p for p in self.pages if p.section is section)

    @cached_property
    def examples(self) -> tuple[Example, ...]:
        """Every example file, read once."""
        return tuple(Example(path, Text.read(path)) for path in self.paths.example_files())

    @cached_property
    def rendered_rust(self) -> frozenset[str]:
        """The text of every rust block of every page, trailing newlines trimmed."""
        return frozenset(c.text.rstrip("\n") for p in self.pages for c in p.rust)


@final
class BookLoader:
    """Runs `mdbook build` and reads the pages SUMMARY lists."""

    PINNED_ENV: ClassVar[dict[str, str]] = {
        "MDBOOK_LOG": "warn",
        "MDBOOK_BUILD__CREATE_MISSING": "false",
        "MDBOOK_OUTPUT__HTML__SMART_PUNCTUATION": "false",
    }
    ENV_PREFIX: ClassVar[str] = "MDBOOK_"
    DIAGNOSTIC: ClassVar[re.Pattern[str]] = re.compile(r"^\s*(ERROR|WARN)\s+(.*)$")
    TARGET: ClassVar[re.Pattern[str]] = re.compile(r"^\w+(?:::\w+)+:\s+")
    CAUSE: ClassVar[re.Pattern[str]] = re.compile(r"^Caused [Bb]y:?\s*(.*)$")
    LINK: ClassVar[re.Pattern[str]] = re.compile(r"\]\(\s*([^)\s]+\.md)\s*\)")
    TOOL: ClassVar[str] = "mdbook"
    SELF: ClassVar[str] = "measure.py"

    def __init__(self, paths: BookPaths, mdbook: str, out: Path) -> None:
        """Build the book at `paths` with the `mdbook` executable into `out`."""
        self.paths = paths
        self.mdbook = mdbook
        self.out = out

    def environment(self) -> dict[str, str]:
        """The caller's environment without its `MDBOOK_*` variables, plus the pinned ones."""
        kept = {k: v for k, v in os.environ.items() if not k.startswith(self.ENV_PREFIX)}
        return kept | self.PINNED_ENV

    def diagnostics(self, run: subprocess.CompletedProcess[str]) -> list[Hit]:
        """One hit per ERROR or WARN of mdbook's stderr, its `Caused by` lines folded in; a bare failure too."""
        messages: list[str] = []
        failed = False
        for line in run.stderr.splitlines():
            found = self.DIAGNOSTIC.match(line)
            if not found:
                continue
            body = self.TARGET.sub("", found[2])
            cause = self.CAUSE.match(body)
            if cause and messages:
                messages[-1] += f" (caused by: {cause[1]})"
            else:
                messages.append(f"{found[1]} {body}")
                failed = failed or found[1] == "ERROR"
        hits = [Hit(self.TOOL, message) for message in messages]
        if run.returncode != 0 and not failed:
            tail = run.stderr.strip().splitlines()[-1:] or [""]
            hits.append(Hit(self.TOOL, f"exited with status {run.returncode} {tail[0]}".strip()))
        return hits

    def build(self) -> Book:
        """Build, then parse every SUMMARY page that rendered."""
        run = subprocess.run(  # noqa: S603
            [self.mdbook, "build", str(self.paths.root), "-d", str(self.out)],
            env=self.environment(),
            capture_output=True,
            encoding="utf-8",
            check=False,
        )
        log = self.diagnostics(run)
        pages: list[RenderedPage] = []
        for rel in self.listed():
            html = self.paths.page_html(self.out, rel)
            if not html.is_file():
                log.append(Hit(self.SELF, f"page not rendered: {rel}"))
                continue
            parser = PageParser()
            parser.feed(Text.read(html))
            parser.close()
            md = self.paths.page_md(rel)
            pages.append(RenderedPage(rel, md, self.paths.shown(md), parser.blocks, tuple(parser.stray)))
        return Book(self.paths, tuple(pages), tuple(log))

    def listed(self) -> list[Path]:
        """The pages in SUMMARY.md, in order, relative to `src`."""
        seen: dict[Path, None] = {}
        for target in self.LINK.findall(Text.read(self.paths.summary)):
            if "://" not in target:
                seen.setdefault(Path(target), None)
        return list(seen)


class Check(Protocol):
    """One mechanical check."""

    name: CheckName
    final_only: bool

    def run(self, book: Book) -> Finding:
        """Run the check over the book."""
        ...


@final
class BuildLog:
    """Every ERROR or WARN line mdbook printed, and a failed exit."""

    name: CheckName = CheckName.BUILD
    final_only: bool = False

    def run(self, book: Book) -> Finding:
        """List mdbook's diagnostics."""
        return Finding(Text.noun(len(book.build_log), "build diagnostic"), book.build_log)


@final
class ProseWords:
    """Prose size per tutorial page, per reference entry, and in total."""

    name: CheckName = CheckName.PROSE
    final_only: bool = False

    def run(self, book: Book) -> Finding:
        """Check the word limits and report text that sits outside any paragraph."""
        hits: list[Hit] = []
        pages_over = entries_over = 0
        worst_page = worst_entry = Worst(0, "-")
        for page in book.pages:
            hits.extend(
                Hit(page.where_text(text), "text sits outside any paragraph, list item or heading")
                for text in page.stray
            )
            if page.section is Section.TUTORIAL:
                worst_page = max(worst_page, Worst(page.words, page.shown))
                if page.words > Limits.TUTORIAL_PAGE_WORDS:
                    pages_over += 1
                    message = f"tutorial page has {page.words} prose words (limit {Limits.TUTORIAL_PAGE_WORDS})"
                    hits.append(Hit(page.page_where(), message))
            elif page.section is Section.REFERENCE:
                for entry in page.entries:
                    worst_entry = max(worst_entry, Worst(entry.words, f"{page.shown} {entry.label}"))
                    if entry.words > Limits.REFERENCE_ENTRY_WORDS:
                        entries_over += 1
                        message = (
                            f"reference entry {entry.label} has {entry.words} prose words "
                            f"(limit {Limits.REFERENCE_ENTRY_WORDS})"
                        )
                        hits.append(Hit(page.where(entry.first), message))
        total = sum(page.words for page in book.pages if page.section is not Section.REFERENCE)
        if total > Targets.TOTAL_WORDS:
            hits.append(Hit(book.paths.shown(book.paths.root), f"{total} prose words (target {Targets.TOTAL_WORDS})"))
        summary = (
            f"total {total}/{Targets.TOTAL_WORDS}; {Text.noun(pages_over, 'tutorial page')} over "
            f"{Limits.TUTORIAL_PAGE_WORDS} (worst {worst_page.words}, {worst_page.label}); "
            f"{Text.noun(entries_over, 'reference entry', 'reference entries')} over "
            f"{Limits.REFERENCE_ENTRY_WORDS} (worst {worst_entry.words}, {worst_entry.label})"
        )
        return Finding(summary, tuple(hits))


@final
class RunOnSentences:
    """Sentences holding too many inline code spans."""

    name: CheckName = CheckName.RUN_ON
    final_only: bool = False

    def run(self, book: Book) -> Finding:
        """List every sentence over the span limit."""
        hits: list[Hit] = []
        total = 0
        for page in book.pages:
            for index, prose in enumerate(page.prose):
                for sentence in prose.sentences:
                    total += 1
                    if sentence.span_count > Limits.MAX_CODE_SPANS:
                        message = (
                            f"sentence holds {sentence.span_count} inline code spans "
                            f"(limit {Limits.MAX_CODE_SPANS}): {Text.preview(sentence.marked)!r}"
                        )
                        hits.append(Hit(page.where(index, sentence.marked), message))
        summary = f"sentences over {Limits.MAX_CODE_SPANS} code spans: {len(hits)} of {total}"
        return Finding(summary, tuple(hits))


@final
class CodeInProse:
    """Inline code in prose that is an expression or a signature, which belongs in a code block."""

    name: CheckName = CheckName.CODE
    final_only: bool = False

    CODE_LIKE: ClassVar[re.Pattern[str]] = re.compile(r"->|=>|\bfn\s*\(|\bimpl\s|[{}]|=.*::|::.*=")

    def run(self, book: Book) -> Finding:
        """List every inline code span in prose that reads as code, not as a name."""
        hits: list[Hit] = []
        total = 0
        for page in book.pages:
            for index, prose in enumerate(page.prose):
                for span in prose.spans:
                    total += 1
                    code = prose.text[span.start : span.end]
                    if self.CODE_LIKE.search(code):
                        message = f"inline code is an expression or signature, use a code block: {code!r}"
                        hits.append(Hit(page.where(index, prose.marked()), message))
        return Finding(f"code-like inline spans: {len(hits)} of {total}", tuple(hits))


@final
class Stubs:
    """Pages with no block beyond one heading, and `fn main() {}` examples."""

    name: CheckName = CheckName.STUBS
    final_only: bool = True

    def run(self, book: Book) -> Finding:
        """List stub pages and stub examples."""
        hits = [Hit(page.page_where(), "page has nothing but a heading") for page in book.pages if page.is_stub]
        hits.extend(
            Hit(book.paths.shown(example.path), "example is still `fn main() {}`")
            for example in book.examples
            if example.is_stub
        )
        return Finding(Text.noun(len(hits), "stub"), tuple(hits))


@final
class OrphanExamples:
    """Every example file is some page's rendered rust block."""

    name: CheckName = CheckName.ORPHANS
    final_only: bool = True

    def run(self, book: Book) -> Finding:
        """List example files whose text no page renders."""
        hits = [
            Hit(book.paths.shown(example.path), "example is included by no page")
            for example in book.examples
            if example.rendered not in book.rendered_rust
        ]
        return Finding(Text.noun(len(hits), "orphan example"), tuple(hits))


@final
class PageCount:
    """The tutorial has the planned number of pages."""

    name: CheckName = CheckName.PAGES
    final_only: bool = True

    def run(self, book: Book) -> Finding:
        """Count tutorial pages, part READMEs aside."""
        count = sum(1 for page in book.pages_in(Section.TUTORIAL) if not page.is_part_readme)
        hits: list[Hit] = []
        if count != Targets.TUTORIAL_PAGES:
            hits.append(
                Hit(book.paths.shown(book.paths.root), f"{count} tutorial pages (target {Targets.TUTORIAL_PAGES})")
            )
        return Finding(f"{Text.noun(count, 'tutorial page')} (target {Targets.TUTORIAL_PAGES})", tuple(hits))


@final
class Parser(argparse.ArgumentParser):
    """An argument parser that raises instead of exiting."""

    def error(self, message: str) -> NoReturn:
        """Raise the usage error."""
        raise UsageError(message)


@final
@dataclass(frozen=True)
class Args:
    """Parsed command line."""

    root: Path
    flags: frozenset[Flag] = field(default_factory=frozenset)

    def has(self, flag: Flag) -> bool:
        """Whether `flag` was passed."""
        return flag in self.flags

    @classmethod
    def parse(cls, argv: Sequence[str]) -> Args:
        """Parse `argv`; an unknown flag or a missing root raises `UsageError`."""
        parser = Parser(prog="measure.py", description="Check the tinyklv book against the plan's limits.")
        parser.add_argument("root", type=Path, help="book root holding book.toml and src/SUMMARY.md")
        for flag in Flag:
            parser.add_argument(flag.value, dest=flag.name, action="store_true", help=flag.help)
        ns = vars(parser.parse_args(argv))
        return cls(ns["root"], frozenset(flag for flag in Flag if ns[flag.name]))


@final
class Report:
    """The results of every check, printed in the order given."""

    def __init__(self, results: Sequence[CheckResult]) -> None:
        """Hold the results."""
        self.results = results

    @staticmethod
    def judge(check: Check, finding: Finding, *, at_final: bool) -> CheckResult:
        """The verdict: a final-only check blocks only at `--final`; its summary says so while it only lists."""
        blocking = at_final or not check.final_only
        verdict = (Verdict.FAIL if blocking else Verdict.INFO) if finding.hits else Verdict.PASS
        summary = finding.summary
        if finding.hits and not blocking:
            summary += f" (listed only; {Flag.FINAL} makes it fail)"
        return CheckResult(check.name, verdict, summary, finding.hits)

    def show(self) -> None:
        """Print every hit, then one summary line per check."""
        hits = [(r.name, hit) for r in self.results for hit in r.hits]
        for name, hit in hits:
            print(f"{hit}  [{name}]")
        if hits:
            print()
        for result in self.results:
            print(f"{result.verdict}  {result.name}: {result.summary}")

    @property
    def exit(self) -> Exit:
        """`Exit.FAILED` when any check failed."""
        failed = any(r.verdict is Verdict.FAIL for r in self.results)
        return Exit.FAILED if failed else Exit.OK


@final
class Main:
    """Entry point."""

    @staticmethod
    def run(argv: Sequence[str]) -> Exit:
        """Build the book, run every check in order, print the report."""
        try:
            args = Args.parse(argv)
        except UsageError as error:
            print(f"measure.py: {error}", file=sys.stderr)
            return Exit.USAGE
        paths = BookPaths.at(args.root)
        if problem := paths.problem():
            print(f"measure.py: {problem}", file=sys.stderr)
            return Exit.USAGE
        mdbook = shutil.which("mdbook")
        if mdbook is None:
            print("measure.py: mdbook is not on PATH", file=sys.stderr)
            return Exit.NO_MDBOOK
        checks: list[Check] = [
            BuildLog(),
            ProseWords(),
            RunOnSentences(),
            CodeInProse(),
            Stubs(),
            OrphanExamples(),
            PageCount(),
        ]
        at_final = args.has(Flag.FINAL)
        with tempfile.TemporaryDirectory(prefix="measure-") as tmp:
            book = BookLoader(paths, mdbook, Path(tmp)).build()
            report = Report([Report.judge(c, c.run(book), at_final=at_final) for c in checks])
        report.show()
        return report.exit


if __name__ == "__main__":
    sys.exit(Main.run(sys.argv[1:]))
