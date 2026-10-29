use std::env;
use std::fs;
use std::path::Path;
use std::process::{Command, ExitCode};

#[derive(Default)]
struct Resume {
    meta: Vec<(String, String)>,
    sections: Vec<Section>,
}

#[derive(Default)]
struct Section {
    title: String,
    blocks: Vec<Block>,
}

enum Block {
    Paragraph(String),
    Field(String, String),
    Bullets(Vec<String>),
    Entry(Entry),
}

#[derive(Default)]
struct Entry {
    title: String,
    right: String,
    detail: String,
    detail_right: String,
    body: Vec<Block>,
}

fn split_right_column(line: &str) -> (String, String) {
    match line.rsplit_once("::") {
        Some((left, right)) => (left.trim().to_string(), right.trim().to_string()),
        None => (line.trim().to_string(), String::new()),
    }
}

fn bullet_text(line: &str) -> Option<&str> {
    let rest = line.strip_prefix(['-', '–', '—', '•', '*'])?;
    if rest.is_empty() || rest.starts_with(char::is_whitespace) {
        Some(rest.trim())
    } else {
        None
    }
}

fn push_bullet(blocks: &mut Vec<Block>, text: &str) {
    if let Some(Block::Bullets(bullets)) = blocks.last_mut() {
        bullets.push(text.to_string());
    } else {
        blocks.push(Block::Bullets(vec![text.to_string()]));
    }
}

fn continue_text(blocks: &mut [Block], text: &str) -> bool {
    let target = match blocks.last_mut() {
        Some(Block::Bullets(bullets)) => bullets.last_mut(),
        Some(Block::Paragraph(paragraph)) => Some(paragraph),
        _ => None,
    };
    if let Some(target) = target {
        target.push(' ');
        target.push_str(text);
        true
    } else {
        false
    }
}

fn parse_resume(source: &str) -> Result<Resume, String> {
    let mut resume = Resume::default();
    let mut separated = true;
    for (index, raw_line) in source.trim_start_matches('\u{feff}').lines().enumerate() {
        let line = raw_line.trim();
        let error = |message: &str| format!("line {}: {}", index + 1, message);
        if line.is_empty() {
            separated = true;
            continue;
        }
        if line.starts_with("//") {
            continue;
        }
        if let Some(meta) = line.strip_prefix('@') {
            let (key, value) = meta
                .split_once(':')
                .ok_or_else(|| error("expected @name: ... or @contact: ..."))?;
            let key = key.trim().to_lowercase();
            if !["name", "contact"].contains(&key.as_str()) {
                return Err(error("unknown metadata; use @name or @contact"));
            }
            if value.trim().is_empty() {
                return Err(error("metadata value cannot be empty"));
            }
            if resume.meta.iter().any(|(existing, _)| *existing == key) {
                return Err(error(&format!("duplicate @{key}; keep one value")));
            }
            resume.meta.push((key, value.trim().to_string()));
            continue;
        }
        if let Some(title) = line.strip_prefix('#') {
            let title = title.trim();
            if title.is_empty() || title.starts_with('#') {
                return Err(error("expected a section title after a single #"));
            }
            resume.sections.push(Section {
                title: title.to_string(),
                ..Section::default()
            });
            separated = true;
            continue;
        }
        let section = resume
            .sections
            .last_mut()
            .ok_or_else(|| error("content must follow a # section heading"))?;
        if let Some(heading) = line.strip_prefix('>') {
            let (title, right) = split_right_column(heading);
            if title.is_empty() {
                return Err(error("entry title after > cannot be empty"));
            }
            section.blocks.push(Block::Entry(Entry {
                title,
                right,
                ..Entry::default()
            }));
            separated = false;
            continue;
        }
        if let Some(bullet) = bullet_text(line) {
            if bullet.is_empty() {
                return Err(error("bullet text cannot be empty"));
            }
            if let Some(Block::Entry(entry)) = section.blocks.last_mut() {
                push_bullet(&mut entry.body, bullet);
            } else {
                push_bullet(&mut section.blocks, bullet);
            }
            separated = false;
            continue;
        }
        if let Some(Block::Entry(entry)) = section.blocks.last_mut() {
            if !separated && continue_text(&mut entry.body, line) {
                // Wrapped lines remain attached to the preceding bullet or paragraph.
            } else if entry.detail.is_empty() && entry.body.is_empty() {
                (entry.detail, entry.detail_right) = split_right_column(line);
            } else if !separated && entry.body.is_empty() {
                entry.detail.push(' ');
                entry.detail.push_str(line);
            } else {
                entry.body.push(Block::Paragraph(line.to_string()));
            }
        } else if !separated && raw_line.starts_with(char::is_whitespace) {
            if let Some(Block::Field(_, value)) = section.blocks.last_mut() {
                value.push(' ');
                value.push_str(line);
            } else if !continue_text(&mut section.blocks, line) {
                section.blocks.push(Block::Paragraph(line.to_string()));
            }
        } else if let Some((label, value)) = line
            .split_once(':')
            .filter(|(label, value)| !label.trim().is_empty() && !value.starts_with('/'))
        {
            section.blocks.push(Block::Field(
                label.trim().to_string(),
                value.trim().to_string(),
            ));
        } else if separated || !continue_text(&mut section.blocks, line) {
            section.blocks.push(Block::Paragraph(line.to_string()));
        }
        separated = false;
    }
    Ok(resume)
}

fn escape_latex(value: &str) -> String {
    let mut escaped = String::new();
    for ch in value.chars() {
        match ch {
            '\\' => escaped.push_str(r"\textbackslash{}"),
            '&' => escaped.push_str(r"\&"),
            '%' => escaped.push_str(r"\%"),
            '$' => escaped.push_str(r"\$"),
            '#' => escaped.push_str(r"\#"),
            '_' => escaped.push_str(r"\_"),
            '{' => escaped.push_str(r"\{"),
            '}' => escaped.push_str(r"\}"),
            '~' => escaped.push_str(r"\textasciitilde{}"),
            '^' => escaped.push_str(r"\textasciicircum{}"),
            '|' => escaped.push_str(r"$|$"),
            'ﬀ' => escaped.push_str("ff"),
            'ﬁ' => escaped.push_str("fi"),
            'ﬂ' => escaped.push_str("fl"),
            'ﬃ' => escaped.push_str("ffi"),
            'ﬄ' => escaped.push_str("ffl"),
            '\u{00a0}' | '\u{202f}' => escaped.push(' '),
            _ => escaped.push(ch),
        }
    }
    escaped
}

fn meta_value<'a>(resume: &'a Resume, key: &str) -> &'a str {
    resume
        .meta
        .iter()
        .find(|(meta_key, _)| meta_key == key)
        .map(|(_, value)| value.as_str())
        .unwrap_or("")
}

fn render_contact_item(token: &str) -> String {
    let (label_text, text) = token
        .trim()
        .split_once("=>")
        .unwrap_or((token.trim(), token.trim()));
    let text = text.trim();
    let label = escape_latex(
        label_text
            .trim()
            .trim_start_matches("https://")
            .trim_start_matches("http://"),
    );

    if text.contains('@') && !text.starts_with("http") {
        return format!(r"\href{{mailto:{}}}{{{}}}", escape_latex(text), label);
    }

    if text.starts_with("http://")
        || text.starts_with("https://")
        || text.starts_with("linkedin.com")
        || text.starts_with("github.com")
    {
        let href = if text.starts_with("http") {
            text.to_string()
        } else {
            format!("https://{}", text)
        };
        return format!(r"\href{{{}}}{{{}}}", escape_latex(&href), label);
    }

    escape_latex(text)
}

fn render_heading(resume: &Resume) -> String {
    let name = escape_latex(meta_value(resume, "name"));
    let contact = meta_value(resume, "contact")
        .split('|')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(render_contact_item)
        .filter(|item| !item.is_empty())
        .collect::<Vec<_>>()
        .join(" $|$ ");

    format!(
        r"\begin{{center}}
    {{\Huge \textbf{{{}}}}} \\ \vspace{{4pt}}
    \small {}
\end{{center}}",
        name, contact
    )
}

fn render_bullets(bullets: &[String]) -> String {
    if bullets.is_empty() {
        return String::new();
    }

    let mut output = String::from("    \\resumeItemListStart\n");
    for bullet in bullets {
        output.push_str(&format!("      \\resumeItem{{{}}}\n", escape_latex(bullet)));
    }
    output.push_str("    \\resumeItemListEnd");
    output
}

fn is_compact(entry: &Entry) -> bool {
    entry.detail.is_empty()
        && entry.detail_right.is_empty()
        && entry.body.is_empty()
        && !entry.right.is_empty()
}

fn render_entry(entry: &Entry) -> String {
    let title = escape_latex(&entry.title);
    let right = escape_latex(&entry.right);
    // A date-only entry is a compact row (awards, certificates, etc.).
    // Formatting depends on content, never on the section's spelling or position.
    let compact = is_compact(entry);
    let mut output = if compact {
        format!("  \\resumeCompactHeading{{{title}}}{{{right}}}")
    } else if entry.detail.is_empty() && entry.detail_right.is_empty() {
        format!("  \\resumeProjectHeading\n    {{{title}}}{{{right}}}")
    } else {
        format!(
            "  \\resumeSubheading\n    {{{title}}}{{{right}}}\n    {{{}}}{{{}}}",
            escape_latex(&entry.detail),
            escape_latex(&entry.detail_right)
        )
    };
    for block in &entry.body {
        match block {
            Block::Bullets(bullets) => {
                output.push('\n');
                output.push_str(&render_bullets(bullets));
            }
            Block::Paragraph(text) => {
                output.push_str(&format!(
                    "\n\\par\\small {}\\par\\normalsize\n",
                    escape_latex(text)
                ));
            }
            _ => unreachable!("entry bodies only contain paragraphs and bullets"),
        }
    }
    output
}

fn render_section(section: &Section) -> String {
    if section.blocks.is_empty() {
        return String::new();
    }
    let mut output = format!("\\section{{{}}}", escape_latex(&section.title));
    let mut in_entries = false;
    let mut previous_entry_has_body = false;
    let mut in_fields = false;
    for block in &section.blocks {
        if !matches!(block, Block::Entry(entry) if !is_compact(entry)) && in_entries {
            output.push_str("\n\\resumeSubHeadingListEnd\n");
            in_entries = false;
        }
        if !matches!(block, Block::Field(_, _)) && in_fields {
            output.push_str("\n\\end{itemize}\n\\endgroup\n");
            in_fields = false;
        }
        match block {
            Block::Entry(entry) if is_compact(entry) => {
                output.push_str("\n");
                output.push_str(&render_entry(entry));
                output.push_str("\n");
            }
            Block::Entry(entry) => {
                if !in_entries {
                    output.push_str("\n\\resumeSubHeadingListStart\n");
                    in_entries = true;
                } else if previous_entry_has_body || !entry.body.is_empty() {
                    output.push_str("\n\n\\vspace{5pt}\n");
                } else {
                    output.push('\n');
                }
                output.push_str(&render_entry(entry));
                previous_entry_has_body = !entry.body.is_empty();
            }
            Block::Field(label, value) => {
                if !in_fields {
                    output.push_str("\n\\begingroup\\small\n\\begin{itemize}[leftmargin=0pt,label={},itemsep=0pt,topsep=2pt,parsep=0pt]\n");
                    in_fields = true;
                }
                output.push_str(&format!(
                    "  \\item {{\\small\\textbf{{{}}}: {}}}\n",
                    escape_latex(label),
                    escape_latex(value)
                ));
            }
            Block::Paragraph(text) => {
                output.push_str(&format!("\n\\par{{\\small {}}}\\par\n", escape_latex(text)))
            }
            Block::Bullets(bullets) => {
                output.push('\n');
                output.push_str(&render_bullets(bullets));
                output.push('\n');
            }
        }
    }
    if in_entries {
        output.push_str("\n\\resumeSubHeadingListEnd");
    }
    if in_fields {
        output.push_str("\n\\end{itemize}\n\\endgroup");
    }
    output
}

fn render_latex(resume: &Resume) -> String {
    let mut output = String::from(
        r"\documentclass[a4paper,11pt]{article}

\usepackage[a4paper,margin=0.38in]{geometry}
\usepackage{titlesec}
\usepackage{enumitem}
\usepackage[hidelinks]{hyperref}
\usepackage{fancyhdr}
\usepackage[english]{babel}
\usepackage{tabularx}
\usepackage{needspace}
\usepackage{xurl}
\ifdefined\pdfgentounicode
\input{glyphtounicode}
\pdfgentounicode=1
\fi

\pagestyle{fancy}
\fancyhf{}
\fancyfoot{}
\renewcommand{\headrulewidth}{0pt}
\renewcommand{\footrulewidth}{0pt}

\setlength{\footskip}{5pt}

\urlstyle{same}
\raggedbottom
\raggedright
\setlength{\tabcolsep}{0in}

\titleformat{\section}{
  \vspace{-6pt}\scshape\raggedright\large
}{}{0em}{}[\titlerule \vspace{-6pt}]

\newcommand{\resumeItem}[1]{
  \item\small{#1}
}

% Measure each heading so long dates wrap and headings stay with following text.
\newsavebox{\resumeHeadingBox}
\newsavebox{\resumeRightBox}
\newlength{\resumeRightWidth}
\newcommand{\resumeHeadingRows}[4]{%
  #1 & #2%
  \if\relax\detokenize{#3#4}\relax
    \\
  \else
    \\ \textit{\small #3} & \textit{\small #4} \\
  \fi
}
\newcommand{\resumeHeadingTable}[4]{%
  \sbox{\resumeRightBox}{#2}%
  \setlength{\resumeRightWidth}{\wd\resumeRightBox}%
  \sbox{\resumeRightBox}{\textit{\small #4}}%
  \ifdim\wd\resumeRightBox>\resumeRightWidth
    \setlength{\resumeRightWidth}{\wd\resumeRightBox}%
  \fi
  \ifdim\resumeRightWidth>0pt
    \ifdim\resumeRightWidth>0.34\linewidth
      \begin{tabularx}{\linewidth}[t]{@{}>{\raggedright\arraybackslash}X@{\hspace{12pt}}>{\raggedleft\arraybackslash}p{0.34\linewidth}@{}}
        \resumeHeadingRows{#1}{#2}{#3}{#4}
      \end{tabularx}%
    \else
      \begin{tabularx}{\linewidth}[t]{@{}>{\raggedright\arraybackslash}X@{\hspace{12pt}}r@{}}
        \resumeHeadingRows{#1}{#2}{#3}{#4}
      \end{tabularx}%
    \fi
  \else
    \begin{tabularx}{\linewidth}[t]{@{}>{\raggedright\arraybackslash}Xr@{}}
      \resumeHeadingRows{#1}{#2}{#3}{#4}
    \end{tabularx}%
  \fi
}
\newcommand{\resumeSubheading}[4]{%
  \sbox{\resumeHeadingBox}{\resumeHeadingTable{\small\textbf{#1}}{#2}{#3}{#4}}%
  \needspace{\dimexpr\ht\resumeHeadingBox+\dp\resumeHeadingBox+3\baselineskip\relax}%
  \vspace{-2pt}\item\usebox{\resumeHeadingBox}\vspace{-2pt}
}
\newcommand{\resumeProjectHeading}[2]{\resumeSubheading{#1}{#2}{}{}}
\newcommand{\resumeCompactHeading}[2]{%
  \par\begingroup\small
  \renewcommand{\arraystretch}{1.35}%
  \sbox{\resumeHeadingBox}{\resumeHeadingTable{#1}{#2}{}{}}%
  \needspace{\dimexpr\ht\resumeHeadingBox+\dp\resumeHeadingBox\relax}%
  \nointerlineskip\noindent\usebox{\resumeHeadingBox}\par
  \endgroup
}

\newcommand{\resumeSubHeadingListStart}{\begin{itemize}[leftmargin=0pt,label={},itemsep=3pt,topsep=2pt,parsep=0pt]}
\newcommand{\resumeSubHeadingListEnd}{\end{itemize}}
\newcommand{\resumeItemListStart}{\begin{itemize}[leftmargin=0.26in,itemsep=1pt,topsep=3pt,parsep=0pt,beginpenalty=10000,label=\textbullet]}
\newcommand{\resumeItemListEnd}{\end{itemize}\vspace{-5pt}}

\begin{document}

",
    );

    output.push_str(&render_heading(resume));
    output.push_str("\n\n");
    output.push_str(
        &resume
            .sections
            .iter()
            .map(render_section)
            .collect::<Vec<_>>()
            .join("\n\n"),
    );
    output.push_str("\n\n\\end{document}\n");
    output
}

fn has_command(command: &str) -> bool {
    let status = if cfg!(windows) {
        Command::new("where").arg(command).output()
    } else {
        Command::new("sh")
            .arg("-c")
            .arg(format!("command -v {}", command))
            .output()
    };

    status
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn run(command: &str, args: &[&str]) -> bool {
    Command::new(command)
        .args(args)
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn build_pdf(tex_file: &str) -> bool {
    if fs::create_dir_all("build").is_err() {
        eprintln!("Could not create build directory.");
        return false;
    }

    if has_command("latexmk") {
        return run(
            "latexmk",
            &[
                "-pdf",
                "-interaction=nonstopmode",
                "-halt-on-error",
                "-output-directory=build",
                tex_file,
            ],
        );
    }

    if has_command("pdflatex") {
        let args = [
            "-interaction=nonstopmode",
            "-halt-on-error",
            "-output-directory=build",
            tex_file,
        ];
        return run("pdflatex", &args) && run("pdflatex", &args);
    }

    if has_command("tectonic") {
        return run("tectonic", &["--outdir", "build", tex_file]);
    }

    eprintln!("No LaTeX compiler found. Install one of: latexmk, pdflatex, or tectonic.");
    false
}

fn main() -> ExitCode {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args
        .iter()
        .any(|arg| arg.starts_with("--") && arg != "--pdf")
        || args.iter().filter(|arg| !arg.starts_with("--")).count() > 2
    {
        eprintln!("Usage: resume-compiler [input.txt] [output.tex] [--pdf]");
        return ExitCode::FAILURE;
    }
    let should_build_pdf = args.iter().any(|arg| arg == "--pdf");
    let positional = args
        .iter()
        .filter(|arg| !arg.starts_with("--"))
        .cloned()
        .collect::<Vec<_>>();

    let input_file = positional
        .get(0)
        .map(String::as_str)
        .unwrap_or("resume.txt");
    let output_file = positional
        .get(1)
        .map(String::as_str)
        .unwrap_or("resume.tex");

    let source = match fs::read_to_string(input_file) {
        Ok(source) => source,
        Err(error) => {
            eprintln!("Could not read {}: {}", input_file, error);
            return ExitCode::FAILURE;
        }
    };

    let resume = match parse_resume(&source) {
        Ok(resume) => resume,
        Err(error) => {
            eprintln!("{}: {}", input_file, error);
            return ExitCode::FAILURE;
        }
    };
    if meta_value(&resume, "name").is_empty()
        || resume
            .sections
            .iter()
            .all(|section| section.blocks.is_empty())
    {
        eprintln!("Resume requires @name and at least one # section.");
        return ExitCode::FAILURE;
    }
    let latex = render_latex(&resume);

    if let Some(parent) = Path::new(output_file).parent() {
        if !parent.as_os_str().is_empty() && fs::create_dir_all(parent).is_err() {
            eprintln!("Could not create output directory for {}.", output_file);
            return ExitCode::FAILURE;
        }
    }

    if let Err(error) = fs::write(output_file, latex) {
        eprintln!("Could not write {}: {}", output_file, error);
        return ExitCode::FAILURE;
    }

    println!("Wrote {} from {}", output_file, input_file);

    if should_build_pdf && !build_pdf(output_file) {
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parsed(source: &str) -> Resume {
        parse_resume(source).expect("valid test input")
    }

    fn entries(section: &Section) -> Vec<&Entry> {
        section
            .blocks
            .iter()
            .filter_map(|block| match block {
                Block::Entry(entry) => Some(entry),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn bom_and_right_columns_are_parsed() {
        let resume =
            parsed("\u{feff}@name: Test\n# Education\n> University :: 2024\nDegree :: GPA: 3.90");
        assert_eq!(meta_value(&resume, "name"), "Test");
        assert_eq!(entries(&resume.sections[0])[0].right, "2024");
        assert_eq!(entries(&resume.sections[0])[0].detail_right, "GPA: 3.90");
    }

    #[test]
    fn selected_projects_and_awards_have_no_empty_detail_row() {
        let resume = parsed("# Selected Projects\n> Project\n- Result\n# Awards\n> Award :: 2025");
        for section in &resume.sections[..1] {
            let rendered = render_section(section);
            assert!(rendered.contains("\\resumeProjectHeading"));
            assert!(!rendered.contains("\\resumeSubheading"));
        }
    }

    #[test]
    fn labeled_contacts_escape_link_targets() {
        assert_eq!(
            render_contact_item("GitHub => https://github.com/test_user"),
            r"\href{https://github.com/test\_user}{GitHub}"
        );
        assert_eq!(escape_latex("50% & C_1"), r"50\% \& C\_1");
    }

    #[test]
    fn pasted_bullet_markers_stay_under_their_project_in_source_order() {
        for marker in ["-", "–", "—", "•"] {
            let resume = parsed(&format!(
                "# Selected Projects\n> Earlier project\n- Earlier result\n\
                 > Space Fighter | Java\n{marker} Built a game\n{marker} Applied OOP\n\
                 > Later project\n- Later result"
            ));
            let section = &resume.sections[0];
            let java = entries(section)[1];
            assert!(
                section
                    .blocks
                    .iter()
                    .all(|block| matches!(block, Block::Entry(_))),
                "marker: {marker}"
            );
            assert!(java.detail.is_empty(), "marker: {marker}");
            assert!(
                matches!(&java.body[0], Block::Bullets(bullets) if bullets == &["Built a game", "Applied OOP"])
            );

            let rendered = render_section(section);
            assert!(!rendered.contains("\\resumeSubheading"));
            let expected_order = [
                "Earlier project",
                "\\resumeItem{Earlier result}",
                "Space Fighter",
                "\\resumeItem{Built a game}",
                "\\resumeItem{Applied OOP}",
                "Later project",
                "\\resumeItem{Later result}",
            ];
            let positions: Vec<_> = expected_order
                .iter()
                .map(|text| rendered.find(text).expect("missing project content"))
                .collect();
            assert!(positions.windows(2).all(|pair| pair[0] < pair[1]));
        }
    }

    #[test]
    fn renaming_a_section_does_not_change_entry_layout() {
        let project = render_latex(&parsed("# Projects\n> Game\n- Built it"));
        let renamed = render_latex(&parsed("# Portfolio\n> Game\n- Built it"));
        assert_eq!(project.replace("{Projects}", "{Portfolio}"), renamed);
    }

    #[test]
    fn awards_keep_descriptions_and_bullets() {
        let rendered = render_latex(&parsed(
            "# Awards\n> Gold :: 2025\nNational contest\n- Ranked first",
        ));
        assert!(rendered.contains("National contest"));
        assert!(rendered.contains("Ranked first"));
    }

    #[test]
    fn wrapped_bullet_text_does_not_become_a_subtitle() {
        let rendered = render_latex(&parsed(
            "# Projects\n> Game\n- Built a game\n  using Java and Swing",
        ));
        assert!(rendered.contains(r"\resumeItem{Built a game using Java and Swing}"));
        assert!(!rendered.contains("\\resumeSubheading\n"));
    }

    #[test]
    fn standalone_bullets_are_not_discarded() {
        let rendered = render_latex(&parsed("# Highlights\n- Improved performance"));
        assert!(rendered.contains("Improved performance"));
    }

    #[test]
    fn fields_keep_their_format_when_section_is_renamed() {
        let skills = render_latex(&parsed("# Skills\nLanguages: Rust, Java"));
        let renamed = render_latex(&parsed("# Toolbox\nLanguages: Rust, Java"));
        assert_eq!(skills.replace("{Skills}", "{Toolbox}"), renamed);
    }
    fn assert_order(rendered: &str, texts: &[&str]) {
        let mut offset = 0;
        for text in texts {
            let position = rendered[offset..]
                .find(text)
                .unwrap_or_else(|| panic!("missing or out of order: {text}"));
            offset += position + text.len();
        }
    }

    fn permutations(values: &mut [usize], start: usize, visit: &mut impl FnMut(&[usize])) {
        if start == values.len() {
            visit(values);
        } else {
            for i in start..values.len() {
                values.swap(start, i);
                permutations(values, start + 1, visit);
                values.swap(start, i);
            }
        }
    }

    #[test]
    fn all_section_orders_preserve_content_and_format() {
        let source = [
            "# Education\n> University :: 2024\nDegree :: GPA: 3.90",
            "# Experience\n> Employer :: 2025\nEngineer\n- Delivered software",
            "# Projects\n> Java Game\n- Built a game\n- Added sound",
            "# Skills\nLanguages: Rust, Java\nTools: Git",
            "# Awards\n> Gold medal :: 2023\n> Silver medal :: 2022",
        ];
        let expected: Vec<_> = source
            .iter()
            .map(|text| render_section(&parsed(text).sections[0]))
            .collect();
        let mut count = 0;
        permutations(&mut [0, 1, 2, 3, 4], 0, &mut |order| {
            let input = order
                .iter()
                .map(|&i| source[i])
                .collect::<Vec<_>>()
                .join("\n\n");
            let resume = parsed(&input);
            let rendered = render_latex(&resume);
            let blocks: Vec<_> = order.iter().map(|&i| expected[i].as_str()).collect();
            assert_order(&rendered, &blocks);
            count += 1;
        });
        assert_eq!(count, 120);
    }

    #[test]
    fn adding_removing_and_reordering_projects_preserves_neighbors() {
        let projects = [
            "> First\n- First result",
            "> Second :: 2026\nDescription\n- Second result",
            "> Third\n– Third result\n  with continuation",
        ];
        permutations(&mut [0, 1, 2], 0, &mut |order| {
            let source = format!(
                "# Portfolio\n{}",
                order
                    .iter()
                    .map(|&i| projects[i])
                    .collect::<Vec<_>>()
                    .join("\n\n")
            );
            let resume = parsed(&source);
            assert_eq!(entries(&resume.sections[0]).len(), 3);
            for (position, &i) in order.iter().enumerate() {
                let alone = parsed(&format!("# Any name\n{}", projects[i]));
                assert_eq!(
                    render_entry(entries(&resume.sections[0])[position]),
                    render_entry(entries(&alone.sections[0])[0])
                );
            }
        });
        for omitted in 0..projects.len() {
            let source = format!(
                "# Portfolio\n{}",
                projects
                    .iter()
                    .enumerate()
                    .filter(|(i, _)| *i != omitted)
                    .map(|(_, p)| *p)
                    .collect::<Vec<_>>()
                    .join("\n")
            );
            assert_eq!(entries(&parsed(&source).sections[0]).len(), 2);
        }
    }

    #[test]
    fn mixed_blocks_and_entry_paragraphs_remain_in_source_order() {
        let resume = parsed("# Overview\nIntroduction\n\nLanguages: Rust\n\nMore context\n\n- Highlight\n\n> Project\nSubtitle\n\nOpening paragraph\n- Result\n  continued\n\nClosing paragraph\n> Next\n- Final result");
        assert_order(
            &render_latex(&resume),
            &[
                "Introduction",
                "Languages",
                "More context",
                "Highlight",
                "{Project}",
                "Subtitle",
                "Opening paragraph",
                "Result continued",
                "Closing paragraph",
                "{Next}",
                "Final result",
            ],
        );
    }

    #[test]
    fn common_spacing_and_line_endings_are_equivalent() {
        let canonical =
            parsed("@name: Test\n# Education\n> University :: 2024\nDegree :: GPA: 3.90\n- Result");
        let pasted = parsed("\u{feff}@name: Test\r\n\r\n#\tEducation\r\n>University::2024\r\nDegree::GPA: 3.90\r\n•\u{00a0}Result\r\n");
        assert_eq!(render_latex(&canonical), render_latex(&pasted));
        for marker in ["-", "–", "—", "•", "*"] {
            for gap in [" ", "  ", "\t", "\u{00a0}"] {
                let input = format!("@name: Test\n# Education\n> University :: 2024\nDegree :: GPA: 3.90\n{marker}{gap}Result");
                assert_eq!(render_latex(&canonical), render_latex(&parsed(&input)));
            }
        }
    }

    #[test]
    fn invalid_input_reports_the_source_line() {
        for (source, line) in [
            ("@name: Test\nOrphan text", 2),
            ("@name: Test\n#", 2),
            ("@name: Test\n## Nested", 2),
            ("# Projects\n> :: 2026", 2),
            ("# Projects\n-", 2),
            ("@name Test", 1),
            ("@naem: Test", 1),
            ("@name: Test\n@name: Other", 2),
        ] {
            let error = parse_resume(source)
                .err()
                .expect("must reject malformed input");
            assert!(error.starts_with(&format!("line {line}:")), "{error}");
        }
    }

    #[test]
    fn empty_sections_are_omitted_and_metadata_can_move() {
        let source =
            "# Empty\n# Projects\n> Game\n- Built it\n@name: Test\n@contact: test@example.com";
        let rendered = render_latex(&parsed(source));
        assert!(!rendered.contains("\\section{Empty}"));
        assert!(rendered.contains("\\textbf{Test}"));
        assert!(rendered.contains("mailto:test@example.com"));
    }

    #[test]
    fn copied_ligatures_and_special_characters_are_safe() {
        assert_eq!(
            escape_latex("diﬀiculty ﬁle ﬂow ﬃ ﬄ"),
            "difficulty file flow ffi ffl"
        );
        assert_eq!(
            escape_latex("C# & 50% _ $ {x} \\ ~ ^"),
            r"C\# \& 50\% \_ \$ \{x\} \textbackslash{} \textasciitilde{} \textasciicircum{}"
        );
    }
}
