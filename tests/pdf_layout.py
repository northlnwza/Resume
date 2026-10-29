#!/usr/bin/env python3
"""Compile edit scenarios and check PDF text, order, bounds and heading pagination.

Requires a supported LaTeX engine and pdfplumber. Use --render for PNG previews.
All fixtures and outputs stay in tmp/pdf-regression; resume.txt is read-only.
"""
import argparse
import hashlib
import json
import re
import shutil
import subprocess
import unicodedata
from pathlib import Path

import pdfplumber

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "tmp/pdf-regression"
HEADER = "@name: Layout Test\n@contact: test@example.com | GitHub => https://github.com/example\n\n"
PROSE = "Built and tested a Java application with clear user controls, reliable event handling, and documented results."


def cases():
    source = (ROOT / "resume.txt").read_text()
    parts = re.split(r"(?m)(?=^# )", source)
    result = {"current": source, "reordered": parts[0] + "\n".join(reversed(parts[1:]))}
    result["renamed"] = re.sub(r"(?m)^# (.+)$", lambda m: "# Custom " + m[1], source)
    projects = "> AddedProject | Java, Swing, SQL\n- AddedResult " + PROSE + "\n"
    result["added-project"] = source + "\n# Additional Work\n" + projects
    project_section = next((p for p in parts[1:] if "Projects" in p.splitlines()[0]), None)
    if project_section:
        heading, *entries = re.split(r"(?m)(?=^> )", project_section)
        replacement = heading + projects + "\n".join(reversed(entries[1:]))
        result["edited-projects"] = source.replace(project_section, replacement)
    result["wrapped-text"] = HEADER + """# Overview
IntroductionMarker introduces the resume.

Languages: Rust, Java
  Python and SQL

ParagraphMarker follows the skills.

- StandaloneMarker preserves a standalone highlight.
  ContinuedMarker continues it.

> GameMarker | Java
SubtitleMarker
  continues the subtitle

OpeningMarker starts a paragraph.
– FirstBulletMarker uses a pasted dash.
  ContinuationMarker belongs to the same bullet.
• SecondBulletMarker uses a pasted bullet.

ClosingMarker follows the bullets.
> NextMarker
* LastBulletMarker tests an asterisk.
# Recognition
> AwardMarker :: 2026
AwardDetailMarker explains the award.
- AwardResultMarker preserves the result.
"""
    result["long-headings"] = HEADER + "# Long Heading Examples\n" + "\n".join(
        f"> LongTitle{i} " + ("Extensive Java application architecture and concurrent programming " * (i + 1))
        + ":: " + ("September 2024 through December 2026 with continued collaboration " * (i + 1))
        + f"\nDegree{i} " + "Computer Engineering and Applied Software Development " * 2
        + ":: " + "Grade point average 3.90 out of 4.00 and additional academic recognition"
        + f"\n- LongResult{i} {PROSE}" for i in range(3)
    )
    result["many-projects"] = HEADER + "# Portfolio\n" + "\n".join(
        f"> Project{i:03} | Java, Swing, Concurrent Programming\n- Result{i:03} {PROSE}\n- Detail{i:03} {PROSE}"
        for i in range(35)
    )
    result["many-awards"] = HEADER + "# Recognition\n" + "\n".join(
        f"> Award{i:03} for software engineering and dependable application development :: September 2026"
        for i in range(90)
    )
    result["many-sections"] = HEADER + "\n".join(
        f"# Section{i:03}\n> Heading{i:03}\n- Outcome{i:03} {PROSE}\n- Evidence{i:03} {PROSE}"
        for i in range(22)
    )
    result["long-section-headings"] = HEADER + "\n".join(
        f"# Group{i:03} " + "Software Engineering and Applied Research Experience " * 4
        + f"\n> Entry{i:03} | Java\n- First{i:03} {PROSE}\n- Second{i:03} {PROSE}"
        for i in range(12)
    )
    result["long-bullet"] = HEADER + "# Research\n> ResearchMarker\n- StartMarker " + (PROSE + " ") * 100 + " EndMarker\n"
    result["special-characters"] = HEADER + """# Engineering & Research
> C# / C++ | Java :: 2024 -- Present
- Accuracy: 95% & improvement: 10%; budget $100; file_name {version}; C:\\tools.
- Copied text: diﬀiculty, ﬁles, ﬂow; café, naïve, résumé.
"""
    result["long-contact"] = HEADER.replace(
        "GitHub => https://github.com/example",
        "https://github.com/example/" + "repository" * 15 + "?name=test_value&version=1#section"
    ) + "# Projects\n> ContactProject\n- ContactResult checks long contact wrapping.\n"
    return result


def normalized(text):
    return unicodedata.normalize("NFKC", text)


def verify_pdf(name, pdf_path, source):
    records = []
    page_texts = []
    with pdfplumber.open(pdf_path) as pdf:
        assert pdf.pages, "empty PDF"
        for number, page in enumerate(pdf.pages, 1):
            assert abs(page.width - 595.28) < 1 and abs(page.height - 841.89) < 1, "not A4"
            chars = [c for c in page.chars if c["text"].strip()]
            assert chars, f"blank page {number}"
            # TeX fits baselines at the lower margin; font bounding boxes include
            # descenders below it. Check baselines there, and actual page bounds too.
            margin = 0.38 * 72 - 1.5
            escaped = [c for c in chars if c["x0"] < margin or c["x1"] > page.width - margin
                       or c["top"] < 0.38 * 72 - max(1.5, c["size"] * 0.15)
                       or c["matrix"][5] < margin or c["bottom"] > page.height]
            assert not escaped, f"text outside margins on page {number}: " + str([(c["text"], round(c["x0"], 1), round(c["top"], 1)) for c in escaped[:5]])
            text = normalized(page.extract_text() or "")
            page_texts.append(text)
            records.append({"page": number, "characters": len(chars), "bottom": round(max(c["bottom"] for c in chars), 2)})
        text = "\n".join(page_texts)
        if name == "wrapped-text":
            markers = ["IntroductionMarker", "Languages", "Python and SQL", "ParagraphMarker", "StandaloneMarker", "ContinuedMarker", "GameMarker", "SubtitleMarker", "OpeningMarker", "FirstBulletMarker", "ContinuationMarker", "SecondBulletMarker", "ClosingMarker", "NextMarker", "LastBulletMarker", "AwardMarker", "AwardDetailMarker", "AwardResultMarker"]
        else:
            markers = re.findall(r"\b(?:Added(?:Project|Result)|(?:LongTitle|LongResult|Degree)\d|(?:Project|Result|Detail|Award|Section|Heading|Outcome|Evidence)\d{3}|(?:Start|End|Research|ContactProject|ContactResult)Marker)\b", source)
        offset = 0
        for marker in markers:
            position = text.find(marker, offset)
            assert position >= 0, f"missing or out-of-order content: {marker}"
            assert text.count(marker) == 1, f"duplicated content: {marker}"
            offset = position + len(marker)
        # Check every source section, including reordered copies of the real resume.
        compact_pages = [" ".join(t.split()) for t in page_texts]
        for section in re.split(r"(?m)(?=^# )", source)[1:]:
            lines = [line.strip() for line in section.splitlines() if line.strip() and not line.startswith("//")]
            if len(lines) < 2:
                continue
            heading = normalized(lines[0][2:].strip())
            first = lines[1].lstrip(">-–—•* ").split("::")[0].split("|")[0].strip()
            # The first three words avoid line-wrap hyphenation in long prose.
            first = normalized(" ".join(first.split()[:3]))
            heading_page = next(i for i, t in enumerate(compact_pages) if heading in t)
            body_page = next(i for i, t in enumerate(compact_pages) if first in t)
            assert heading_page == body_page, f"orphaned section: {heading}"
        # Every bullet's original words must survive compilation in their order.
        alnum = lambda value: "".join(c for c in normalized(value) if c.isalnum())
        compact_text = alnum(text)
        offset = 0
        for bullet in re.findall(r"(?m)^\s*[-–—•*]\s+(.+)$", source):
            fragment = alnum(bullet)
            position = compact_text.find(fragment, offset)
            assert position >= 0, f"missing or reordered bullet: {bullet[:60]}"
            offset = position + len(fragment)
        pairs = []
        if name == "many-projects": pairs = [(f"Project{i:03}", f"Result{i:03}") for i in range(35)]
        if name == "many-sections":
            pairs = [(f"Section{i:03}", f"Heading{i:03}") for i in range(22)] + [(f"Heading{i:03}", f"Outcome{i:03}") for i in range(22)]
        if name == "long-headings": pairs = [(f"LongTitle{i}", f"LongResult{i}") for i in range(3)]
        for heading, body in pairs:
            heading_page = next(i for i, t in enumerate(page_texts) if heading in t)
            body_page = next(i for i, t in enumerate(page_texts) if body in t)
            assert heading_page == body_page, f"orphaned heading: {heading}"
        if name in ("many-projects", "many-awards", "many-sections", "long-bullet", "long-section-headings"):
            assert len(pdf.pages) > 1, "stress fixture did not exercise pagination"
    return records


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--render", action="store_true")
    parser.add_argument("--case", action="append", help="Only run these case names")
    args = parser.parse_args()
    if args.render and not shutil.which("pdftoppm"):
        parser.error("--render requires pdftoppm")
    subprocess.run(["cargo", "build"], cwd=ROOT, check=True)
    OUT.mkdir(parents=True, exist_ok=True)
    report = {}
    failures = []
    for name, source in cases().items():
        if args.case and name not in args.case:
            continue
        directory = OUT / name
        directory.mkdir(exist_ok=True)
        input_path = directory / "resume.txt"
        input_path.write_text(source)
        result = subprocess.run([str(ROOT / "target/debug/resume-compiler"), str(input_path), str(directory / "resume.tex"), "--pdf", "--out-dir", str(directory)], cwd=ROOT, capture_output=True, text=True)
        log = result.stdout + result.stderr
        (directory / "compile.log").write_text(log)
        try:
            assert result.returncode == 0, log
            assert not re.search(r"Overfull \\[hv]box|Missing character:|Undefined control sequence", log, re.I), log
            pdf = directory / "resume.pdf"
            records = verify_pdf(name, pdf, source)
            report[name] = {"status": "passed", "pages": records, "source_sha256": hashlib.sha256(source.encode()).hexdigest(), "compiler_sha256": hashlib.sha256((ROOT / "target/debug/resume-compiler").read_bytes()).hexdigest()}
            print(f"PASS {name}: {len(records)} page(s)", flush=True)
        except (AssertionError, StopIteration) as error:
            report[name] = {"status": "failed", "reason": str(error)}
            failures.append(name)
            print(f"FAIL {name}: {error}", flush=True)
        if args.render and (directory / "resume.pdf").exists():
            for old in directory.glob("page-*.png"):
                old.unlink()
            subprocess.run(["pdftoppm", "-scale-to", "1400", "-png", str(directory / "resume.pdf"), str(directory / "page")], check=True)
    assert report, "no cases selected"
    (OUT / "report.json").write_text(json.dumps(report, indent=2))
    if failures:
        raise SystemExit("PDF regressions: " + ", ".join(failures))
    print(f"Passed {len(report)} PDF scenarios. Report: {OUT / 'report.json'}")


if __name__ == "__main__":
    main()
