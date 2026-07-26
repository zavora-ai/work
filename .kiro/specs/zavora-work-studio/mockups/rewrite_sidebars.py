#!/usr/bin/env python3
"""Rewrite every narrow-frame sidebar in mockups.html to the thread-list navigation model."""
import re
import sys
from pathlib import Path

SRC = Path(__file__).with_name("mockups.html")

# Per-frame: which sidebar row is active. 'new' | 'dash' | 'set' | thread index 1-6
ACTIVE = {
    "s1": "dash", "s3": "new", "s4": "dash", "s5": "dash", "s6": "dash",
    "s7": 1, "s8": "dash", "s9": "new", "s11": 6, "s12": "set",
    "s20": "all", "s21": "set", "s22": "set",
}

THREADS = [
    (1, "Daily newsletter", "sched", "Next tomorrow, 7:00 am"),
    (2, "Inbox triage", "attn", "Gmail needs reconnecting"),
    (3, "Computer health", "work", "Checking now"),
    (4, "Board deck — July", "done", "Finished 21:04"),
    (5, "Q3 revenue model", "done", "Finished yesterday"),
    (6, "Partnership agreement", "work", "You edited it 2 days ago"),
]

GLYPH = {
    "work": '<span class="g work" role="img" aria-label="{t}"></span>',
    "sched": '<span class="g sched" role="img" aria-label="{t}"><svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4"><circle cx="12" cy="12" r="9"/><path d="M12 7v5l3 2"/></svg></span>',
    "attn": '<span class="g attn" role="img" aria-label="{t}"><svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4"><path d="M12 3 2 20h20L12 3z"/></svg></span>',
    "done": '<span class="g done" role="img" aria-label="{t}"><svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3"><path d="M20 6 9 17l-5-5"/></svg></span>',
    "pause": '<span class="g pause" role="img" aria-label="{t}"><svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.6"><path d="M9 5v14M15 5v14"/></svg></span>',
}

ICON_NEW = '<svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M12 5v14M5 12h14"/></svg>'
ICON_DASH = '<svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="3" y="3" width="7" height="7" rx="1"/><rect x="14" y="3" width="7" height="7" rx="1"/><rect x="3" y="14" width="7" height="7" rx="1"/><rect x="14" y="14" width="7" height="7" rx="1"/></svg>'
ICON_SET = '<svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.7 1.7 0 0 0 .3 1.9l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.7 1.7 0 0 0-2.9 1.2v.2a2 2 0 1 1-4 0v-.1a1.7 1.7 0 0 0-2.9-1.3l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1A1.7 1.7 0 0 0 2.5 15a2 2 0 1 1 0-4h.2a1.7 1.7 0 0 0 1.2-2.9l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1a1.7 1.7 0 0 0 2.9-1.2V4a2 2 0 1 1 4 0v.2a1.7 1.7 0 0 0 2.9 1.2l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1A1.7 1.7 0 0 0 21.5 11h.2a2 2 0 1 1 0 4z"/></svg>'


def sidebar(active):
    rows = []
    for idx, name, state, tip in THREADS:
        on = " on" if active == idx else ""
        rows.append(
            f'      <div class="th{on}" tabindex="0" aria-label="{name}, {tip}">'
            f'{GLYPH[state].format(t=tip)}<span class="nm">{name}</span></div>'
        )
    threads = "\n".join(rows)

    docs = [
        ("all", "All files", '<svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8"><path d="M3 7h18v12a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/><path d="M3 7l2-3h6l2 3"/></svg>'),
        ("doc", "Documents", '<svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8"><path d="M14 3v5h5"/><path d="M19 8v11a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h7z"/></svg>'),
        ("deck", "Decks", '<svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8"><rect x="2" y="4" width="20" height="13" rx="2"/><path d="M9 21h6M12 17v4"/></svg>'),
        ("sheet", "Spreadsheets", '<svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8"><rect x="3" y="4" width="18" height="16" rx="2"/><path d="M3 10h18M9 10v10"/></svg>'),
        ("f1", "Board packs", '<svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8"><path d="M3 7h6l2 2h10v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/></svg>'),
        ("f2", "Expenses", '<svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8"><path d="M3 7h6l2 2h10v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/></svg>'),
    ]
    drows = "\n".join(
        f'      <div class="th{" on" if active == key else ""}" tabindex="0">'
        f'<span class="fi">{icon}</span><span class="nm">{label}</span></div>'
        for key, label, icon in docs
    )

    return f"""<div class="side">
    <div class="brand">Zavora Work Studio</div>
    <div class="nav">
      <a class="{'on' if active == 'new' else ''}" href="#">{ICON_NEW}New work</a>
      <a class="{'on' if active == 'dash' else ''}" href="#">{ICON_DASH}Dashboard<span class="b">3</span></a>
    </div>
    <div class="sect">Your work</div>
    <div class="thlist">
{threads}
    </div>
    <div class="sect">Documents</div>
    <div class="thlist">
{drows}
    </div>
    <div class="foot nav">
      <a class="{'on' if active == 'set' else ''}" href="#">{ICON_SET}Settings</a>
    </div>
  </div>
  """


def main():
    html = SRC.read_text()
    # Each narrow frame is: <div class="frame" id="sN"> ... <div class="side"> ... <div class="main"
    pattern = re.compile(
        r'(<div class="frame" id="(s\d+)">\s*)<div class="side">.*?(?=<div class="main)',
        re.DOTALL,
    )

    seen = []

    def repl(m):
        head, fid = m.group(1), m.group(2)
        if fid not in ACTIVE:
            return m.group(0)
        seen.append(fid)
        return head + sidebar(ACTIVE[fid])

    out = pattern.sub(repl, html)
    if out == html:
        print("no sidebars rewritten", file=sys.stderr)
        return 1
    SRC.write_text(out)
    print("rewrote:", ", ".join(seen))
    missing = sorted(set(ACTIVE) - set(seen))
    if missing:
        print("NOT FOUND:", ", ".join(missing), file=sys.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
