"""Generates the README image: an editor showing fib.jw next to a Command
Prompt window running `jawac run fib.jw`.

The command types itself out, the output appears line by line and the
cursor blinks (SMIL, which GitHub renders in README images). Every
animated element's base value is its final state, so a viewer without
animation shows the finished frame. `--static` leaves the animation out.

    python3 scripts/readme_image.py assets/jawac-cmd.svg
"""

import re
import sys
from xml.sax.saxutils import escape

OUT = sys.argv[1]
STATIC = "--static" in sys.argv

FONT = ("'Cascadia Mono', 'Cascadia Code', Consolas, 'SF Mono', Menlo, "
        "'DejaVu Sans Mono', monospace")
UI_FONT = "'Segoe UI', system-ui, -apple-system, sans-serif"

CODE = """gawe fib(n: wilangan): wilangan {
    yen (n < 2) {
        bali n
    }
    bali fib(n - 1) + fib(n - 2)
}

gawe utama() {
    kanggo (ayo i = 0; i < 10; i++) {
        tulis(`fib(${i}) = ${fib(i)}`)
    }
}""".split("\n")

OUTPUT = [f"fib({i}) = {v}" for i, v in enumerate([0, 1, 1, 2, 3, 5, 8, 13, 21, 34])]

# Warm darks, with gold for keywords: a nod to batik sogan.
C = {
    "keyword": "#E8B04B",
    "type": "#63C5B1",
    "function": "#8FB8F0",
    "number": "#F2A07B",
    "string": "#A6CC70",
    "interp": "#E88AA0",
    "plain": "#DAD3C6",
    "punct": "#A9A193",
    "lineno": "#6B6359",
}

KEYWORDS = {"gawe", "yen", "bali", "kanggo", "ayo", "liyane", "tetep"}
TYPES = {"wilangan", "pecahan", "ukara", "logika"}
FUNCTIONS = {"fib", "utama", "tulis"}

NBSP = chr(0xA0)
CURSOR = chr(0x2588)


def text(content):
    """Escapes text for SVG and keeps every space: browsers drop leading
    spaces in SVG text, so spaces become no-break spaces."""
    return escape(content).replace(" ", NBSP)


def tokens(line):
    """Splits a line of Jawa into (text, colour) pieces."""
    out = []
    pos = 0
    in_template = False
    while pos < len(line):
        rest = line[pos:]
        if in_template:
            if rest.startswith("${"):
                out.append(("${", C["interp"]))
                end = rest.index("}")
                out.extend(tokens(rest[2:end]))
                out.append(("}", C["interp"]))
                pos += end + 1
            elif rest[0] == "`":
                out.append(("`", C["string"]))
                in_template = False
                pos += 1
            else:
                piece = re.match(r"[^`$]+", rest).group(0)
                out.append((piece, C["string"]))
                pos += len(piece)
            continue
        if rest[0] == "`":
            out.append(("`", C["string"]))
            in_template = True
            pos += 1
            continue
        word = re.match(r"[A-Za-z_][A-Za-z_0-9]*", rest)
        number = re.match(r"[0-9]+", rest)
        space = re.match(r"\s+", rest)
        if word:
            w = word.group(0)
            colour = (C["keyword"] if w in KEYWORDS
                      else C["type"] if w in TYPES
                      else C["function"] if w in FUNCTIONS
                      else C["plain"])
            out.append((w, colour))
            pos += len(w)
        elif number:
            out.append((number.group(0), C["number"]))
            pos += len(number.group(0))
        elif space:
            out.append((space.group(0), C["plain"]))
            pos += len(space.group(0))
        else:
            out.append((rest[0], C["punct"]))
            pos += 1
    return out


def tspans(pieces):
    return "".join(f'<tspan fill="{colour}">{text(piece)}</tspan>'
                   for piece, colour in pieces)


def hidden_until(at, attribute="opacity"):
    """Keeps an element hidden until `at` seconds, then shows it for good."""
    if STATIC:
        return ""
    return (f'<animate attributeName="{attribute}" values="0;1" keyTimes="0;1" '
            f'calcMode="discrete" dur="{at:.3f}s" fill="freeze"/>')


W, H = 920, 404

# The editor window, behind.
EX, EY, EW, EH = 16, 16, 434, 318
E_TITLE = 36
E_LINE = 21
E_CODE_Y = EY + E_TITLE + 30

# The Command Prompt window, in front.
CX, CY, CW, CH = 418, 74, 486, 312
C_TITLE = 34
C_LINE = 19.5
C_TEXT_X = CX + 18
C_TEXT_Y = CY + C_TITLE + 28

PROMPT = "D:\\jawascript> "
COMMAND = "jawac run fib.jw"
TYPE_START = 0.7
TYPE_STEP = 0.075
OUTPUT_START = TYPE_START + len(COMMAND) * TYPE_STEP + 0.45
OUTPUT_STEP = 0.11
FINAL_PROMPT = OUTPUT_START + len(OUTPUT) * OUTPUT_STEP + 0.35

svg = []
add = svg.append
add(f'<svg xmlns="http://www.w3.org/2000/svg" width="{W}" height="{H}" '
    f'viewBox="0 0 {W} {H}" role="img" aria-labelledby="title desc">')
add('<title id="title">JawaScript: jawac run fib.jw</title>')
add('<desc id="desc">A code editor showing fib.jw, a Fibonacci program written in '
    'Jawa with Javanese keywords, next to a Command Prompt window in which '
    '`jawac run fib.jw` prints fib(0) = 0 through fib(9) = 34.</desc>')
add('<defs>'
    '<filter id="shadow" x="-10%" y="-10%" width="120%" height="130%">'
    '<feDropShadow dx="0" dy="10" stdDeviation="12" flood-color="#000" '
    'flood-opacity="0.35"/></filter>'
    f'<clipPath id="editor-clip"><rect x="{EX}" y="{EY}" width="{EW}" '
    f'height="{EH}" rx="10"/></clipPath>'
    f'<clipPath id="cmd-clip"><rect x="{CX}" y="{CY}" width="{CW}" '
    f'height="{CH}" rx="8"/></clipPath>'
    '</defs>')
add(f'<g font-family="{FONT}" font-size="14">')

# --- The editor -------------------------------------------------------------
add(f'<g filter="url(#shadow)"><rect x="{EX}" y="{EY}" width="{EW}" height="{EH}" '
    'rx="10" fill="#1C1A17"/></g>')
add('<g clip-path="url(#editor-clip)">')
add(f'<rect x="{EX}" y="{EY}" width="{EW}" height="{E_TITLE}" fill="#2B2823"/>')
for i, colour in enumerate(["#E2685F", "#E8B04B", "#7FBF6A"]):
    add(f'<circle cx="{EX + 20 + i * 18}" cy="{EY + E_TITLE / 2}" r="5.5" '
        f'fill="{colour}"/>')
add(f'<text x="{EX + EW / 2}" y="{EY + E_TITLE / 2 + 4.5}" text-anchor="middle" '
    f'font-size="13" fill="#B7AE9F">fib.jw</text>')
for i, line in enumerate(CODE):
    y = E_CODE_Y + i * E_LINE
    add(f'<text x="{EX + 36}" y="{y}" text-anchor="end" fill="{C["lineno"]}">'
        f'{i + 1}</text>')
    if line:
        add(f'<text x="{EX + 52}" y="{y}">{tspans(tokens(line))}</text>')
add('</g>')

# --- The Command Prompt -----------------------------------------------------
add(f'<g filter="url(#shadow)"><rect x="{CX}" y="{CY}" width="{CW}" height="{CH}" '
    'rx="8" fill="#0C0C0C"/></g>')
add('<g clip-path="url(#cmd-clip)">')
add(f'<rect x="{CX}" y="{CY}" width="{CW}" height="{C_TITLE}" fill="#1F1F1F"/>')
add(f'<rect x="{CX + 12}" y="{CY + 9}" width="16" height="16" rx="3" fill="#3A3A3A"/>')
add(f'<text x="{CX + 14.5}" y="{CY + 21.5}" font-size="10" font-weight="bold" '
    'fill="#F2F2F2">&gt;_</text>')
add(f'<text x="{CX + 38}" y="{CY + 21.5}" font-family="{UI_FONT}" font-size="12.5" '
    'fill="#E6E6E6">Command Prompt</text>')
mid = CY + C_TITLE / 2
x = CX + CW - 120
add(f'<line x1="{x}" y1="{mid}" x2="{x + 10}" y2="{mid}" stroke="#CFCFCF"/>')
x += 46
add(f'<rect x="{x}" y="{mid - 5}" width="10" height="10" fill="none" stroke="#CFCFCF"/>')
x += 46
add(f'<path d="M{x} {mid - 5}l10 10M{x + 10} {mid - 5}l-10 10" stroke="#CFCFCF"/>')

# The prompt, and the command typed one character at a time.
parts = [f'<tspan fill="#CCCCCC">{text(PROMPT)}</tspan>']
for i, ch in enumerate(COMMAND):
    colour = "#E8B04B" if i < len("jawac") else "#F2F2F2"
    at = TYPE_START + i * TYPE_STEP
    parts.append(f'<tspan fill="{colour}">{text(ch)}'
                 f'{hidden_until(at, "fill-opacity")}</tspan>')
add(f'<text x="{C_TEXT_X}" y="{C_TEXT_Y}">{"".join(parts)}</text>')

# The program's output, a line at a time.
for i, line in enumerate(OUTPUT):
    y = C_TEXT_Y + (i + 1) * C_LINE
    add(f'<text x="{C_TEXT_X}" y="{y}" fill="#CCCCCC">{text(line)}'
        f'{hidden_until(OUTPUT_START + i * OUTPUT_STEP)}</text>')

# A fresh prompt with a blinking cursor.
blink = "" if STATIC else (
    '<animate attributeName="fill-opacity" values="1;1;0;0" '
    f'keyTimes="0;0.5;0.5;1" dur="1.1s" begin="{FINAL_PROMPT:.2f}s" '
    'repeatCount="indefinite"/>')
y = C_TEXT_Y + (len(OUTPUT) + 2) * C_LINE
add(f'<g>{hidden_until(FINAL_PROMPT)}<text x="{C_TEXT_X}" y="{y}" fill="#CCCCCC">'
    f'{text(PROMPT)}<tspan>{CURSOR}{blink}</tspan></text></g>')
add('</g>')

add('</g>')
add('</svg>')

with open(OUT, "w", encoding="utf-8", newline="\n") as f:
    f.write("\n".join(svg) + "\n")
print(f"wrote {OUT}: {W}x{H}, final prompt at {FINAL_PROMPT:.2f}s")
