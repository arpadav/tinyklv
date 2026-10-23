// --------------------------------------------------
// tinyklv book theme: wire bytes and `#[klv(..)]` keywords in color
// --------------------------------------------------
(() => {
    "use strict";

    // --------------------------------------------------
    // shared
    // --------------------------------------------------
    // escapes text for innerHTML
    const esc = (text) => text.replace(/&/g, "&amp;").replace(/</g, "&lt;");

    // `text`, which starts at `start` in its block, with each [start, end, class] hit in it
    // wrapped in a span; a hit's own hits (its 4th field) nest inside that span
    function spans(text, start, hits) {
        let html = "";
        let last = 0;
        for (const [a, b, cls, inner = []] of hits) {
            const from = Math.max(a - start, last);
            const to = Math.min(b - start, text.length);
            if (from >= to) continue;
            const piece = spans(text.slice(from, to), start + from, inner);
            html += `${esc(text.slice(last, from))}<span class="${cls}">${piece}</span>`;
            last = to;
        }
        return html + esc(text.slice(last));
    }

    // wraps the hits (sorted, disjoint) of `block` text node by text node, so highlight.js
    // spans and search <mark>s stay; `plain(a, b)` drops a highlight.js token's own color
    function wrap(block, hits, plain) {
        const walker = document.createTreeWalker(block, NodeFilter.SHOW_TEXT);
        const nodes = [];
        while (walker.nextNode()) nodes.push(walker.currentNode);
        let offset = 0;
        let next = 0;
        for (const node of nodes) {
            const start = offset;
            const text = node.nodeValue;
            offset += text.length;
            while (next < hits.length && hits[next][1] <= start) next++;
            const token = node.parentElement;
            if (token.closest(".hljs-comment")) continue;
            if (token !== block && /^hljs-/.test(token.className) && plain(start, offset)) {
                token.classList.add("klv-plain");
            }
            let end = next;
            while (end < hits.length && hits[end][0] < offset) end++;
            if (end === next) continue;
            const html = spans(text, start, hits.slice(next, end));
            node.replaceWith(document.createRange().createContextualFragment(html));
        }
    }

    // --------------------------------------------------
    // wire bytes: ```klv blocks and span.klv cells
    // --------------------------------------------------
    // a role letter, its gap, then hex bytes split by spaces; the space after the last stays out
    const ITEM = /(?<=^|\s)([skljvu]) +([0-9A-Fa-f]{2}(?: +[0-9A-Fa-f]{2})*)(?=\s|$)/dgm;
    const COMMENT = /\/\/.*$/gm;

    // [start, end, class] of each `//` comment and each item in `text`; an item holds its
    // role letter and its byte run
    function wireHits(text) {
        const hits = [...text.matchAll(COMMENT)].map((m) => [
            m.index,
            m.index + m[0].length,
            "klv-comment",
        ]);
        const bare = text.replace(COMMENT, (note) => " ".repeat(note.length));
        for (const m of bare.matchAll(ITEM)) {
            const [role, bytes] = m.indices.slice(1);
            const inner = [[...role, "klv-role"], [...bytes, `klv-${m[1]}`]];
            hits.push([role[0], bytes[1], "klv-item", inner]);
        }
        return hits.sort((a, b) => a[0] - b[0]);
    }

    document.querySelectorAll("code.language-klv, span.klv").forEach((block) => {
        wrap(block, wireHits(block.textContent), () => false);
    });

    // --------------------------------------------------
    // attribute keywords: #[klv(..)] in rust and diff blocks
    // --------------------------------------------------
    // attribute word -> the wire role it shares a color with; `x` = no color
    const ROLE = {
        key: "k", keylen: "k", oneof: "k",
        len: "l", size: "l", exact: "l", lengthed: "l", implicit: "l", reserve: "l",
        dec: "v", enc: "v", typ: "v", latebind: "v", default: "v",
        sentinel: "s", unknown: "u", deny_unknown_keys: "u",
        stream: "x", break_on: "x", strict: "x", debug: "x", trait_fallback: "x",
        skip_if_default: "x", skip_if: "x",
        allow_unimplemented_encode: "x", allow_unimplemented_decode: "x",
    };
    // one identifier, char literal (`b'('`), raw string opener (`br#"`), each read at lastIndex
    const WORD = /\w+/y;
    const CHAR = /'(?:\\.|[^\\'\n])'/y;
    const RAW = /b?r(#*)"/y;
    // what follows an attribute word: `=`, `(`, `,` or `)`
    const AFTER = /\s*[=(,)]/y;
    // a `*` / `&` is a sigil only right after `enc =` or `latebind =`; `oneof` only after `key =`
    const SIGIL = /\b(?:enc|latebind)\s*=\s*$/;
    const ONEOF = /\bkey\s*=\s*$/;

    // the end of the `//` or `/* */` comment that starts at `i`, or -1
    function commentEnd(text, i) {
        if (text.startsWith("//", i)) {
            const end = text.indexOf("\n", i);
            return end < 0 ? text.length : end;
        }
        if (!text.startsWith("/*", i)) return -1;
        const end = text.indexOf("*/", i + 2);
        return end < 0 ? text.length : end + 2;
    }

    // the end of the string, raw string or char literal that starts at `i`, or -1
    function literalEnd(text, i) {
        RAW.lastIndex = i;
        const raw = /\w/.test(text[i - 1] ?? "") ? null : RAW.exec(text);
        if (raw) {
            const end = text.indexOf(`"${raw[1]}`, RAW.lastIndex);
            return end < 0 ? text.length : end + 1 + raw[1].length;
        }
        if (text[i] === '"') {
            let j = i + 1;
            while (j < text.length && text[j] !== '"') j += text[j] === "\\" ? 2 : 1;
            return j + 1;
        }
        CHAR.lastIndex = i;
        return CHAR.test(text) ? CHAR.lastIndex : -1;
    }

    // each closed `#[klv(..)]` of `text`, from `#` to past its `)`, and the [start, end, class]
    // hits: every comment, and in a closed range its `klv`, attribute words and sigils. a word
    // counts only where an attribute starts: after `(` or `,` of an attribute list, before `=(,)`
    function scan(text) {
        const ranges = [];
        const comments = [];
        let words = [];
        const lists = [];
        let prev = "";
        let attr = false;
        for (let i = 0; i < text.length; i++) {
            const c = text[i];
            const note = commentEnd(text, i);
            const literal = note < 0 ? literalEnd(text, i) : -1;
            if (note >= 0) {
                comments.push([i, note, "hljs-comment"]);
                i = note - 1;
            } else if (literal >= 0) {
                i = literal - 1;
                prev = '"';
                attr = false;
            } else if (!lists.length) {
                if (!text.startsWith("#[klv(", i)) continue;
                ranges.push([i, text.length]);
                words.push([i + 2, i + 5, "klv-name"]);
                lists.push(true);
                prev = "(";
                i += 5;
            } else if (c === "(") {
                lists.push(attr);
                prev = c;
                attr = false;
            } else if (c === ")") {
                lists.pop();
                if (!lists.length) ranges[ranges.length - 1][1] = i + 1;
                prev = c;
                attr = false;
            } else if (/[A-Za-z_]/.test(c)) {
                WORD.lastIndex = i;
                const word = WORD.exec(text)[0];
                AFTER.lastIndex = i + word.length;
                const at = lists.at(-1) && (prev === "(" || prev === ",") && AFTER.test(text);
                const oneof = word === "oneof" && ONEOF.test(text.slice(Math.max(0, i - 16), i));
                attr = Object.hasOwn(ROLE, word) && at;
                if (attr || oneof) words.push([i, i + word.length, `klv-attr-${ROLE[word]}`]);
                prev = "w";
                i += word.length - 1;
            } else if (/\S/.test(c)) {
                const before = text.slice(Math.max(0, i - 24), i);
                if ((c === "*" || c === "&") && SIGIL.test(before)) {
                    words.push([i, i + 1, "klv-attr-sigil"]);
                }
                prev = c;
                attr = false;
            }
        }
        if (lists.length) {
            const open = ranges.pop()[0];
            words = words.filter((hit) => hit[0] < open);
        }
        return { ranges, hits: [...comments, ...words].sort((a, b) => a[0] - b[0]) };
    }

    // `text` with each line that starts with `drop` blanked, and the `+` / `-` column of the
    // rest, offsets kept
    function side(text, drop) {
        const blank = (line) => (line[0] === drop ? " ".repeat(line.length) : ` ${line.slice(1)}`);
        return text.replace(/^[-+ ].*$/gm, blank);
    }

    // the scan of a rust block; for a diff, of each hunk's two sides (its context + `-` lines,
    // its context + `+` lines) on their own, merged
    function scanBlock(block) {
        const text = block.textContent;
        if (!block.classList.contains("language-diff")) return scan(text);
        const cuts = [0, ...[...text.matchAll(/^@@/gm)].map((m) => m.index), text.length];
        const ranges = [];
        const hits = [];
        for (let h = 0; h + 1 < cuts.length; h++) {
            const at = cuts[h];
            for (const drop of "+-") {
                const part = scan(side(text.slice(at, cuts[h + 1]), drop));
                ranges.push(...part.ranges.map(([a, b]) => [a + at, b + at]));
                hits.push(...part.hits.map(([a, b, cls]) => [a + at, b + at, cls]));
            }
        }
        hits.sort((a, b) => a[0] - b[0]);
        return { ranges, hits: hits.filter((hit, i) => i === 0 || hit[0] >= hits[i - 1][1]) };
    }

    document.querySelectorAll("code.language-rust, code.language-diff").forEach((block) => {
        const found = scanBlock(block);
        wrap(block, found.hits, (a, b) => found.ranges.some(([c, d]) => a < d && c < b));
    });

    // --------------------------------------------------
    // diff width
    // --------------------------------------------------
    // one box as wide as the longest line, so each line's tint spans the scroll width
    document.querySelectorAll("code.language-diff").forEach((block) => {
        const box = document.createElement("span");
        box.className = "klv-diff";
        box.append(...block.childNodes);
        block.append(box);
    });

    // --------------------------------------------------
    // print
    // --------------------------------------------------
    // a printed page shows every fold open; the ones that were closed close again after
    let opened = [];
    window.addEventListener("beforeprint", () => {
        opened = [...document.querySelectorAll("details:not([open])")];
        opened.forEach((fold) => {
            fold.open = true;
        });
    });
    window.addEventListener("afterprint", () => {
        opened.forEach((fold) => {
            fold.open = false;
        });
    });
})();
