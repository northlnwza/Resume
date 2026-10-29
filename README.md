# Resume

This folder contains:

- `resume.txt`: the easy text source you edit
- `src/main.rs`: converts `resume.txt` into `resume.tex`
- `resume.tex`: the editable LaTeX resume source
- `build_resume.sh`: a small build script that compiles `resume.tex` into PDF

## Easy edit format

Edit `resume.txt`.

```text
# Projects
> Project Topic | Python, LLM, RAG
- What you built, improved, measured, or delivered.
- Another result or responsibility.
```

Bullet lines accept `-`, `–`, `—`, `•`, or `*` followed by whitespace (spaces,
tabs, and pasted nonbreaking spaces work). They stay under the preceding `>`
entry in source order. Bullets directly under a section heading also work.

Move whole sections or entries, add projects, or rename headings freely. The
layout follows the content, not special section names:

- `# Heading` starts a section. Empty sections are omitted.
- `> Title` starts an entry. Add `:: right-side text` for a date or location.
- A plain line immediately below an entry is its italic subtitle.
- A title and date with no subtitle or body use a compact row, suitable for awards.
- `Label: value` outside an entry creates a bold label, suitable for skills.
- Consecutive plain lines continue the preceding bullet, subtitle, or paragraph.
  Use a blank line to start a separate paragraph. Indent wrapped skill values.
- Blank lines do not end an entry; the next `>` or `#` does.
- `//` starts a comment line. `@name:` and optional `@contact:` may appear anywhere,
  once each; the name/contact block always appears at the top of the PDF.

For example, wrapped text and extra paragraphs stay with their project:

```text
# Portfolio
> Space Fighter | Java, Swing
- Built a desktop game with collision detection
  and concurrent asteroid spawning.

Additional context for this project.

> Another Project :: 2026
- Another result.
```

Syntax errors (such as empty entry titles, empty bullets, duplicate metadata,
or text before a section) stop generation with the input filename and line
number. Existing output is kept when validation fails.

Use `::` only when you want text on the right side of the resume:

```text
> Mahidol University :: Aug 2024 -- May 2028
Bachelor of Engineering Program in Computer Engineering :: GPA: 3.96
```

Generate the LaTeX resume:

If you do not have Rust installed yet, install it with rustup first.
This works on macOS, Linux, and WSL:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

After installing Rust, restart your terminal or load Cargo into your
current shell:

```bash
. "$HOME/.cargo/env"
```

```bash
cargo run
```

## Build

```bash
./build_resume.sh
```

This regenerates `resume.tex` from `resume.txt`, then writes `build/resume.pdf`.
You can run the script from another working directory using its absolute path.

To build a different text source or choose an output folder:

```bash
./build_resume.sh "another resume.txt" "build/custom"
```

This writes both the generated `.tex` and `.pdf` into the chosen folder. A failed
PDF compilation leaves the last successful PDF in place and exits with an error;
it does not mean that the old PDF contains your latest edits.

Validate syntax without creating files:

```bash
cargo run -- resume.txt --check
```

## Install a LaTeX compiler

The build script supports `latexmk`, `pdflatex`, or `tectonic`.

Option 1: install `tectonic` on macOS with Homebrew:

```bash
brew install tectonic
```

Option 2: install a minimal TeX Live toolchain on WSL or Ubuntu:

```bash
sudo apt update
sudo apt install -y latexmk texlive-latex-base texlive-latex-recommended texlive-latex-extra
```

Option 3: install a larger LaTeX toolchain on macOS with Homebrew:

```bash
brew install --cask basictex
```

After installing BasicTeX, restart your terminal and install `latexmk`:

```bash
sudo tlmgr update --self
sudo tlmgr install latexmk
```

## Rebuild

```bash
./build_resume.sh resume.tex build
```

## Generate and build

```bash
cargo run -- resume.txt resume.tex --pdf
```

Use `--out-dir DIRECTORY` to change the PDF destination. `--help` lists options.

## Windows / WSL

This project can use the Rust and LaTeX installations in WSL. From this
folder in PowerShell, run:

```powershell
wsl --exec bash -lc 'cargo run -- resume.txt resume.tex --pdf'
```

Keep `resume.txt` as the source of truth; regenerating overwrites `resume.tex`.
The layout retains A4 paper, 0.38-inch margins, and the original Computer Modern
font. Content flows naturally to additional pages when needed, without shrinking
the type to force a one-page fit. Long titles and right-side text wrap in separate
columns. Entries keep their headings with the beginning of their content, and
long lists of awards can break across pages.
Use a labeled link in `@contact` to keep the header compact:

```text
@contact: email@example.com | LinkedIn => https://linkedin.com/in/your-profile
```

## Regression checks

Run `cargo test` for parser, rendering, and command-line tests. These include all
120 orders of five sections, project reordering/addition/removal, renamed headings,
wrapped text, pasted bullets, diagnostics, and preservation of the previous PDF
when compilation fails. Compiler-failure tests use a fake engine; no LaTeX install
is required for this suite.

For real PDF layout checks, install Python's `pdfplumber` and one of the supported
LaTeX engines, then run:

```bash
python3 tests/pdf_layout.py
```

Add `--render` to produce page PNGs with Poppler's `pdftoppm`, or use
`--case many-projects` to check one scenario. The suite builds reordered and edited
copies of your resume plus synthetic stress cases. It checks content order,
page boundaries, missing glyphs, text overflow, and orphaned headings. Fixtures,
PDFs, logs, and `report.json` are written to `tmp/pdf-regression/`; your editable
source and normal build outputs are not changed.
