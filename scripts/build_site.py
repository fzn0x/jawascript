"""Builds the documentation website from docs/*.md and site/.

    python3 scripts/build_site.py               # writes build/site
    python3 scripts/build_site.py --out _site

Needs the `markdown` package (pip install markdown). Code is highlighted
here, at build time, in the colours of the README image, so pages read
fine without JavaScript and load nothing from other hosts. Links between
the docs become links between pages; links to other files in the
repository go to GitHub. The Pages workflow (.github/workflows/pages.yml)
runs this and publishes the result.
"""

import argparse
import html
import os
import re
import shutil
import subprocess
import sys
from dataclasses import dataclass

import markdown

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SITE = os.path.join(ROOT, "site")
REPO = "https://github.com/fzn0x/jawascript"


@dataclass
class Page:
    out: str
    source: str  # a markdown file under docs/, or an HTML body under site/
    javanese: str
    english: str
    description: str


PAGES = [
    Page("index.html", "site/home.html", "Omah", "Home",
         "JawaScript: a typed, compiled programming language with the shape "
         "of JavaScript and the words of Javanese."),
    Page("guide.html", "docs/guide.md", "Pandhuan", "Guide",
         "A first tour of Jawa, from a first program to structs and linked "
         "lists, with runnable examples."),
    Page("vocabulary.html", "docs/vocabulary.md", "Kosakata", "Vocabulary",
         "Every Jawa word with its Javanese meaning and its JavaScript and "
         "TypeScript counterpart."),
    Page("reference.html", "docs/language.md", "Referensi", "Reference",
         "The Jawa language reference: types, expressions, statements, "
         "functions, structs and built-ins."),
    Page("architecture.html", "docs/architecture.md", "Arsitektur", "Architecture",
         "How jawac compiles Jawa: a Rust frontend, a custom MLIR dialect, "
         "LLVM and a garbage-collected runtime."),
]
DOCS = [page for page in PAGES if page.source.endswith(".md")]
PAGE_FOR_SOURCE = {os.path.basename(page.source): page.out for page in DOCS}

# ---------------------------------------------------------------------------
# Highlighting
# ---------------------------------------------------------------------------

JAWA = {
    "keyword": {"gawe", "ayo", "tetep", "yen", "liyane", "sajrone", "kanggo",
                "ing", "mandheg", "terusna", "bali", "wujud", "lan", "utawa",
                "ora"},
    "type": {"wilangan", "pecahan", "logika", "ukara", "suwung"},
    "constant": {"bener", "salah", "kosong"},
    "builtin": {"tulis", "dawane", "tambah", "gagal"},
}

JAVASCRIPT = {
    "keyword": {"function", "let", "const", "var", "if", "else", "while", "for",
                "of", "in", "return", "break", "continue", "interface", "class",
                "new", "throw", "type"},
    "type": {"number", "string", "boolean", "void"},
    "constant": {"true", "false", "null", "undefined"},
    "builtin": {"console"},
}

TOKEN = re.compile(r"""
    (?P<comment>//[^\n]*|/\*.*?\*/)
  | (?P<string>"(?:[^"\\\n]|\\.)*"?|'(?:[^'\\\n]|\\.)*'?)
  | (?P<number>0[xX][0-9a-fA-F_]+|0[bB][01_]+|\d[\d_]*(?:\.\d[\d_]*)?(?:[eE][+-]?\d+)?)
  | (?P<word>[^\W\d]\w*)
  | (?P<space>\s+)
  | (?P<other>.)
""", re.VERBOSE | re.DOTALL)


def span(kind, text):
    text = html.escape(text, quote=False)
    return f'<span class="t-{kind}">{text}</span>' if kind else text


def highlight_code(code, words):
    """Highlights Jawa, or JavaScript with the JavaScript word list. Template
    strings may nest code inside `${…}`, so the scanner keeps a stack."""
    out = []
    pos = 0
    # Each entry is the brace depth of a `${` we are inside.
    stack = []
    in_template = False
    while pos < len(code):
        if in_template:
            if code.startswith("${", pos):
                out.append(span("interp", "${"))
                stack.append(0)
                in_template = False
                pos += 2
            elif code[pos] == "`":
                out.append(span("string", "`"))
                in_template = False
                pos += 1
            else:
                match = re.compile(r"(?:\\.|[^`$\\]|\$(?!\{))+", re.DOTALL).match(code, pos)
                out.append(span("string", match.group(0)))
                pos = match.end()
            continue
        char = code[pos]
        if char == "`":
            out.append(span("string", "`"))
            in_template = True
            pos += 1
            continue
        if stack and char == "{":
            stack[-1] += 1
        elif stack and char == "}":
            if stack[-1] == 0:
                stack.pop()
                out.append(span("interp", "}"))
                in_template = True
                pos += 1
                continue
            stack[-1] -= 1
        match = TOKEN.match(code, pos)
        kind = match.lastgroup
        text = match.group(0)
        pos = match.end()
        if kind == "word":
            following = code[pos:pos + 1]
            if text in words["keyword"]:
                kind = "keyword"
            elif text in words["type"] or text[0].isupper():
                kind = "type"
            elif text in words["constant"]:
                kind = "constant"
            elif text in words["builtin"] or following == "(":
                kind = "function"
            else:
                kind = None
        elif kind == "other":
            kind = "punct" if text in "{}()[];,.:" else "operator"
        elif kind == "space":
            kind = None
        out.append(span(kind, text))
    return "".join(out)


def highlight_console(code):
    """A terminal session: `$ ` lines are commands, the rest is output, with
    compiler diagnostics picked out."""
    lines = []
    for line in code.split("\n"):
        if line.startswith("$ "):
            lines.append(span("prompt", "$") + " " + span("command", line[2:]))
        elif line.startswith(("error:", "gagal:")):
            head, _, rest = line.partition(":")
            lines.append(span("error", head + ":") + html.escape(rest, quote=False))
        elif line.startswith("warning:"):
            lines.append(span("warning", "warning:") + html.escape(line[8:], quote=False))
        elif re.match(r"\s*= note:", line):
            lines.append(span("note", line))
        elif re.match(r"\s*(-->|\d*\s*\|)", line):
            lines.append(span("gutter", line))
        else:
            lines.append(html.escape(line, quote=False))
    return "\n".join(lines)


def highlight_shell(code):
    lines = []
    for line in code.split("\n"):
        command, hash_, comment = line.partition("#")
        # A `#` inside a word, like a URL fragment, is not a comment.
        if hash_ and command and not command[-1].isspace():
            command, comment, hash_ = line, "", ""
        lines.append(html.escape(command, quote=False)
                     + (span("comment", hash_ + comment) if hash_ else ""))
    return "\n".join(lines)


def highlight_ir(code, comment):
    pattern = re.compile(rf"""
        (?P<comment>{comment}[^\n]*)
      | (?P<string>"[^"\n]*")
      | (?P<value>[%^@][\w.$-]+)
      | (?P<type>![\w.]+(?:<[^\n>]*>)?|\bi(?:1|8|32|64)\b|\bptr\b|\bvoid\b|<\d+ x i64>)
      | (?P<keyword>\b[a-z_]+\.[a-z_.]+\b|\b(?:define|call|phi|load|store|add|ret|br|align|zeroinitializer)\b)
      | (?P<number>\b\d+\b)
    """, re.VERBOSE)
    out = []
    pos = 0
    for match in pattern.finditer(code):
        out.append(html.escape(code[pos:match.start()], quote=False))
        kind = {"value": "function"}.get(match.lastgroup, match.lastgroup)
        out.append(span(kind, match.group(0)))
        pos = match.end()
    out.append(html.escape(code[pos:], quote=False))
    return "".join(out)


LABELS = {"ts": "jawa", "jawa": "jawa", "js": "typescript", "text": "output",
          "console": "terminal", "sh": "shell", "powershell": "powershell",
          "mlir": "mlir", "llvm": "llvm ir"}


def highlight(lang, code):
    if lang in ("ts", "jawa"):
        return highlight_code(code, JAWA)
    if lang == "js":
        return highlight_code(code, JAVASCRIPT)
    if lang == "console":
        return highlight_console(code)
    if lang in ("sh", "powershell"):
        return highlight_shell(code)
    if lang == "mlir":
        return highlight_ir(code, "//")
    if lang == "llvm":
        return highlight_ir(code, ";")
    return html.escape(code, quote=False)


def code_block(lang, code):
    code = code.rstrip("\n")
    label = LABELS.get(lang, lang or "text")
    kind = label.replace(" ", "-")
    return (f'<div class="code code-{kind}" data-label="{label}">'
            f"<pre><code>{highlight(lang, code)}</code></pre></div>")


# ---------------------------------------------------------------------------
# Pages
# ---------------------------------------------------------------------------

PIPELINE = """<figure class="pipeline">
<ol>
<li class="stage-rust"><span class="stage-lang">Rust</span><b>Lexer</b><span>tokens, automatic semicolons</span></li>
<li class="stage-rust"><span class="stage-lang">Rust</span><b>Parser</b><span>syntax tree, every error reported</span></li>
<li class="stage-rust"><span class="stage-lang">Rust</span><b>Type checker</b><span>names, types, conversions &rarr; typed HIR</span></li>
<li class="stage-rust"><span class="stage-lang">Rust</span><b>Code generator</b><span>builder calls across the cxx bridge</span></li>
<li class="stage-cpp"><span class="stage-lang">C++</span><b>jawa dialect</b><span>structured MLIR ops</span></li>
<li class="stage-cpp"><span class="stage-lang">C++</span><b>Jawa and MLIR passes</b><span>canonicalize, jawa-fuse-concat, CSE, jawa-flatten-cfg, mem2reg</span></li>
<li class="stage-cpp"><span class="stage-lang">C++</span><b>convert-jawa-to-llvm</b><span>LLVM dialect, inline bounds and null checks</span></li>
<li class="stage-cpp"><span class="stage-lang">C++</span><b>LLVM</b><span>-O2 and an object file</span></li>
<li class="stage-c"><span class="stage-lang">C</span><b>Link</b><span>with the jawa_rt runtime and bdwgc &rarr; executable</span></li>
</ol>
<figcaption>From <code>.jw</code> source to a native executable.</figcaption>
</figure>"""

# Kawung, the four-petal batik motif, as the logo. It follows the theme's
# accent colour, unlike the fixed favicon.
MARK = """<svg class="mark" viewBox="0 0 32 32" aria-hidden="true"><g>
<ellipse cx="10.2" cy="10.2" rx="6.9" ry="4.5" transform="rotate(45 10.2 10.2)"/>
<ellipse cx="21.8" cy="10.2" rx="6.9" ry="4.5" transform="rotate(-45 21.8 10.2)"/>
<ellipse cx="10.2" cy="21.8" rx="6.9" ry="4.5" transform="rotate(-45 10.2 21.8)"/>
<ellipse cx="21.8" cy="21.8" rx="6.9" ry="4.5" transform="rotate(45 21.8 21.8)"/>
</g></svg>"""

LINK_ICON = ('<svg viewBox="0 0 16 16" aria-hidden="true"><path d="M6.6 9.4a3 3 0 0 0 4.2 0l2.4-2.4'
             'a3 3 0 0 0-4.2-4.2l-.9.9M9.4 6.6a3 3 0 0 0-4.2 0L2.8 9a3 3 0 0 0 4.2 4.2l.9-.9"/></svg>'
             '<span class="visually-hidden">Link to this section</span>')

FILTER = """<div class="filter" hidden>
<label for="filter-input">Golek tembung <span>&middot; find a word</span></label>
<input id="filter-input" type="search" placeholder="gawe, function, loop&hellip;" autocomplete="off" spellcheck="false">
<p class="filter-empty" hidden>Ora ketemu. No word matches.</p>
</div>"""


def rewrite_link(href, source):
    """Points a link in docs/<source> at the page or GitHub file it names."""
    if re.match(r"^[a-z]+:|^#", href):
        return href
    path, _, fragment = href.partition("#")
    fragment = "#" + fragment if fragment else ""
    if path in PAGE_FOR_SOURCE:
        return PAGE_FOR_SOURCE[path] + fragment
    target = os.path.normpath(os.path.join(os.path.dirname(source), path)).replace("\\", "/")
    if target.startswith("assets/"):
        return target
    kind = "blob" if os.path.isfile(os.path.join(ROOT, target)) else "tree"
    return f"{REPO}/{kind}/main/{target}{fragment}"


def label_cells(match):
    """Gives each table cell its column's heading as `data-label`, so narrow
    screens can show a row as a stack of labelled lines."""
    table = match.group(0)
    headings = [html.escape(re.sub(r"<[^>]+>", "", cell).strip())
                for cell in re.findall(r"<th[^>]*>(.*?)</th>", table, re.DOTALL)]

    def row(row_match):
        columns = iter(headings)
        return re.sub(r"<td(?=[\s>])",
                      lambda cell: f'<td data-label="{next(columns, "")}"', row_match.group(0))

    return re.sub(r"<tr>.*?</tr>", row, table, flags=re.DOTALL)


def render_markdown(page):
    with open(os.path.join(ROOT, page.source), encoding="utf-8") as f:
        text = f.read()
    md = markdown.Markdown(
        extensions=["tables", "fenced_code", "toc", "sane_lists", "smarty"],
        extension_configs={"toc": {"permalink": "#", "permalink_class": "anchor",
                                   "permalink_title": "Link to this section"}},
    )
    body = md.convert(text)

    def code(match):
        lang = match.group(1) or ""
        source = html.unescape(match.group(2))
        if lang == "mermaid":
            return PIPELINE
        return code_block(lang, source)

    body = re.sub(r'<pre><code(?: class="language-(\w+)")?>(.*?)</code></pre>',
                  code, body, flags=re.DOTALL)
    body = re.sub(r'(href|src)="([^"]+)"',
                  lambda m: f'{m.group(1)}="{rewrite_link(html.unescape(m.group(2)), page.source)}"',
                  body)
    body = body.replace("<!-- site: filter -->", FILTER)
    body = re.sub(r'(<a class="anchor" href="#[^"]*" title="[^"]*">)#(</a>)',
                  lambda m: m.group(1) + LINK_ICON + m.group(2), body)
    # A `|` in a table cell is written `\|` in the markdown; GitHub drops the
    # backslash, and so do we.
    body = re.sub(r"<table>.*?</table>", lambda m: m.group(0).replace("\\|", "|"),
                  body, flags=re.DOTALL)
    body = re.sub(r"<table>.*?</table>", label_cells, body, flags=re.DOTALL)
    body = re.sub(r"<table>", '<div class="table-wrap"><table>', body)
    body = re.sub(r"</table>", "</table></div>", body)
    # The reference opens with a list of its sections; the page has its own.
    body = re.sub(r'<ul>\s*(?:<li><a href="#[\w-]+">[^<]*</a></li>\s*)+</ul>\s*',
                  "", body, count=1)

    title_match = re.search(r"<h1[^>]*>(.*?)</h1>", body, re.DOTALL)
    title = re.sub(r"<[^>]+>", "", title_match.group(1)).rstrip("#").strip()
    # Sections, so the vocabulary filter can hide a whole one.
    parts = re.split(r"(?=<h2)", body)
    body = parts[0] + "".join(f"<section>{part}</section>" for part in parts[1:])

    toc = [(token["id"], token["name"]) for token in md.toc_tokens
           for token in (token["children"] if token["level"] == 1 else [token])
           if token["level"] in (1, 2)]
    toc = [(id_, html.unescape(name)) for id_, name in toc if id_]
    return title, body, toc


def nav(current):
    items = []
    for page in PAGES[1:]:
        attr = ' aria-current="page"' if page is current else ""
        items.append(f'<a href="{page.out}"{attr}><span class="jv">{page.javanese}</span>'
                     f'<span class="en">{page.english}</span></a>')
    return "\n".join(items)


def toc_list(toc):
    return "".join(f'<li><a href="#{id_}">{html.escape(name)}</a></li>' for id_, name in toc)


def doc_main(page, title, body, toc):
    index = DOCS.index(page)
    pager = []
    if index > 0:
        prev = DOCS[index - 1]
        pager.append(f'<a class="prev" href="{prev.out}"><span>Sadurunge &middot; previous</span>'
                     f"{prev.javanese}: {prev.english}</a>")
    if index + 1 < len(DOCS):
        nxt = DOCS[index + 1]
        pager.append(f'<a class="next" href="{nxt.out}"><span>Sabanjure &middot; next</span>'
                     f"{nxt.javanese}: {nxt.english}</a>")
    toc_items = toc_list(toc)
    return f"""<div class="doc">
<aside class="toc" aria-labelledby="toc-title">
<p class="toc-title" id="toc-title">Ing kaca iki <span>&middot; on this page</span></p>
<ol>{toc_items}</ol>
</aside>
<main id="main" class="article">
<details class="toc-mobile"><summary>Ing kaca iki &middot; on this page</summary><ol>{toc_items}</ol></details>
{body}
<nav class="pager" aria-label="Pages">{"".join(pager)}</nav>
<p class="edit"><a href="{REPO}/edit/main/{page.source}">Edit this page on GitHub</a></p>
</main>
</div>"""


def home_main(page):
    with open(os.path.join(ROOT, page.source), encoding="utf-8") as f:
        body = f.read()

    def code(match):
        return code_block(match.group(1), html.unescape(match.group(2)))

    body = body.replace("{{pipeline}}", PIPELINE)
    return re.sub(r'<pre><code class="language-(\w+)">\n?(.*?)</code></pre>',
                  code, body, flags=re.DOTALL)


def render(page, layout):
    if page.source.endswith(".md"):
        title, body, toc = render_markdown(page)
        main = doc_main(page, title, body, toc)
        full_title = f"{title} · JawaScript"
        body_class = "page-doc page-" + page.out.removesuffix(".html")
    else:
        main = home_main(page)
        full_title = "JawaScript · a programming language in Javanese"
        body_class = "page-home"
    values = {
        "title": html.escape(full_title),
        "description": html.escape(page.description),
        "nav": nav(page),
        "main": main,
        "body_class": body_class,
        "mark": MARK,
    }
    return re.sub(r"\{\{(\w+)\}\}", lambda m: values[m.group(1)], layout)


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("--out", default=os.path.join(ROOT, "build", "site"),
                        help="output directory (default: build/site)")
    args = parser.parse_args()

    out = os.path.abspath(args.out)
    # Empty the directory rather than deleting it, so a preview server
    # running inside it keeps working.
    os.makedirs(out, exist_ok=True)
    for name in os.listdir(out):
        path = os.path.join(out, name)
        if os.path.isdir(path):
            shutil.rmtree(path)
        else:
            os.remove(path)
    os.makedirs(os.path.join(out, "assets"))

    with open(os.path.join(SITE, "layout.html"), encoding="utf-8") as f:
        layout = f.read()
    for page in PAGES:
        with open(os.path.join(out, page.out), "w", encoding="utf-8", newline="\n") as f:
            f.write(render(page, layout))

    for name in ("style.css", "site.js", "favicon.svg"):
        shutil.copy(os.path.join(SITE, name), os.path.join(out, name))
    shutil.copytree(os.path.join(SITE, "fonts"), os.path.join(out, "fonts"))
    shutil.copy(os.path.join(ROOT, "assets", "jawac-cmd.svg"), os.path.join(out, "assets"))
    # A still frame of the hero image, for readers who prefer less motion.
    subprocess.run([sys.executable, os.path.join(ROOT, "scripts", "readme_image.py"),
                    os.path.join(out, "assets", "jawac-cmd-static.svg"), "--static"],
                   check=True, stdout=subprocess.DEVNULL)
    # GitHub Pages would otherwise run the files through Jekyll.
    open(os.path.join(out, ".nojekyll"), "w").close()
    print(f"wrote {len(PAGES)} pages to {out}")


if __name__ == "__main__":
    main()
