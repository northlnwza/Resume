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

The generated PDF will be written to `build/resume.pdf`.

## Install a LaTeX compiler

The build script supports `latexmk`, `pdflatex`, or `tectonic`.

Option 1: install `tectonic` with Cargo. This works on macOS, Linux, and WSL:

```bash
cargo install tectonic
```

Option 2: install a minimal TeX Live toolchain on WSL or Ubuntu:

```bash
sudo apt update
sudo apt install -y latexmk texlive-latex-base texlive-latex-recommended texlive-latex-extra
```

Option 3: install a LaTeX toolchain on macOS with Homebrew:

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
