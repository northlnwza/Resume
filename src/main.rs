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
    content: Vec<String>,
    entries: Vec<Entry>,
    skills: Vec<Skill>,
}

#[derive(Default)]
struct Entry {
    title: String,
    right: String,
    detail: String,
    detail_right: String,
    bullets: Vec<String>,
}

struct Skill {
    label: String,
    value: String,
}

fn split_right_column(line: &str) -> (String, String) {
    match line.rfind(" :: ") {
        Some(index) => (
            line[..index].trim().to_string(),
            line[index + 4..].trim().to_string(),
        ),
        None => (line.trim().to_string(), String::new()),
    }
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
            _ => escaped.push(ch),
        }
    }
    escaped
}

fn parse_resume(source: &str) -> Resume {
    let mut resume = Resume::default();
    let mut current_section: Option<usize> = None;
    let mut current_entry: Option<usize> = None;

    for raw_line in source.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with("//") {
            continue;
        }

        if let Some(meta_line) = line.strip_prefix('@') {
            if let Some(index) = meta_line.find(':') {
                resume.meta.push((
                    meta_line[..index].trim().to_lowercase(),
                    meta_line[index + 1..].trim().to_string(),
                ));
            }
            continue;
        }

        if let Some(title) = line.strip_prefix("# ") {
            resume.sections.push(Section {
                title: title.trim().to_string(),
                ..Section::default()
            });
            current_section = Some(resume.sections.len() - 1);
            current_entry = None;
            continue;
        }

        let Some(section_index) = current_section else {
            continue;
        };

        if let Some(entry_line) = line.strip_prefix("> ") {
            let (title, right) = split_right_column(entry_line);
            resume.sections[section_index].entries.push(Entry {
                title,
                right,
                ..Entry::default()
            });
            current_entry = Some(resume.sections[section_index].entries.len() - 1);
            continue;
        }

        if let Some(bullet) = line.strip_prefix("- ") {
            if let Some(entry_index) = current_entry {
                resume.sections[section_index].entries[entry_index]
                    .bullets
                    .push(bullet.trim().to_string());
            }
            continue;
        }

        let section = &mut resume.sections[section_index];
        if current_entry.is_none()
            && section.title.to_lowercase().contains("skill")
            && line.contains(':')
        {
            let index = line.find(':').unwrap();
            section.skills.push(Skill {
                label: line[..index].trim().to_string(),
                value: line[index + 1..].trim().to_string(),
            });
            continue;
        }

        if let Some(entry_index) = current_entry {
            let entry = &mut section.entries[entry_index];
            if entry.detail.is_empty() {
                let (detail, detail_right) = split_right_column(line);
                entry.detail = detail;
                entry.detail_right = detail_right;
                continue;
            }
        }

        section.content.push(line.to_string());
    }

    resume
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
    let text = token.trim();
    let label = escape_latex(text.trim_start_matches("https://").trim_start_matches("http://"));

    if text.contains('@') && !text.starts_with("http") {
        return format!(r"\href{{mailto:{}}}{{{}}}", text, label);
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
        return format!(r"\href{{{}}}{{{}}}", href, label);
    }

    escape_latex(text)
}

fn render_heading(resume: &Resume) -> String {
    let name = escape_latex(meta_value(resume, "name"));
    let contact = meta_value(resume, "contact")
        .split('|')
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

fn render_entry(section: &Section, entry: &Entry) -> String {
    let title = escape_latex(&entry.title);
    let right = escape_latex(&entry.right);
    let bullets = render_bullets(&entry.bullets);
    let section_key = section.title.to_lowercase();
    let should_use_project_heading = entry.detail.is_empty()
        && (section_key == "projects" || section_key == "awards & activities");

    if should_use_project_heading {
        let mut output = format!("  \\resumeProjectHeading\n    {{{}}}{{{}}}", title, right);
        if !bullets.is_empty() {
            output.push('\n');
            output.push_str(&bullets);
        }
        return output;
    }

    let mut output = format!(
        "  \\resumeSubheading\n    {{{}}}{{{}}}\n    {{{}}}{{{}}}",
        title,
        right,
        escape_latex(&entry.detail),
        escape_latex(&entry.detail_right)
    );
    if !bullets.is_empty() {
        output.push('\n');
        output.push_str(&bullets);
    }
    output
}

fn render_section(section: &Section) -> String {
    let mut output = format!("\\section{{{}}}", escape_latex(&section.title));

    if !section.content.is_empty() {
        output.push_str("\n\\small\n");
        output.push_str(
            &section
                .content
                .iter()
                .map(|line| escape_latex(line))
                .collect::<Vec<_>>()
                .join("\n\n"),
        );
        output.push_str("\n\\normalsize");
    }

    if !section.skills.is_empty() {
        output.push_str("\n\\begin{itemize}[leftmargin=0.15in, label={}]\n");
        output.push_str("  \\small{\\item{\n");
        output.push_str(
            &section
                .skills
                .iter()
                .map(|skill| {
                    format!(
                        "    \\textbf{{{}}}: {}",
                        escape_latex(&skill.label),
                        escape_latex(&skill.value)
                    )
                })
                .collect::<Vec<_>>()
                .join(" \\\\\n"),
        );
        output.push_str("\n  }}\n\\end{itemize}");
    }

    if !section.entries.is_empty() {
        output.push_str("\n\\resumeSubHeadingListStart\n");
        output.push_str(
            &section
                .entries
                .iter()
                .map(|entry| render_entry(section, entry))
                .collect::<Vec<_>>()
                .join("\n\n"),
        );
        output.push_str("\n\\resumeSubHeadingListEnd");
    }

    output
}

fn render_latex(resume: &Resume) -> String {
    let mut output = String::from(
        r"\documentclass[letterpaper,11pt]{article}

\usepackage[empty]{fullpage}
\usepackage{titlesec}
\usepackage{enumitem}
\usepackage[hidelinks]{hyperref}
\usepackage{fancyhdr}
\usepackage[english]{babel}
\usepackage{tabularx}
\input{glyphtounicode}

\pagestyle{fancy}
\fancyhf{}
\fancyfoot{}
\renewcommand{\headrulewidth}{0pt}
\renewcommand{\footrulewidth}{0pt}

\addtolength{\oddsidemargin}{-0.5in}
\addtolength{\evensidemargin}{-0.5in}
\addtolength{\textwidth}{1in}
\addtolength{\topmargin}{-.6in}
\addtolength{\textheight}{1.2in}
\setlength{\footskip}{5pt}

\urlstyle{same}
\raggedbottom
\raggedright
\setlength{\tabcolsep}{0in}
\pdfgentounicode=1

\titleformat{\section}{
  \vspace{-6pt}\scshape\raggedright\large
}{}{0em}{}[\titlerule \vspace{-6pt}]

\newcommand{\resumeItem}[1]{
  \item\small{#1}
}

\newcommand{\resumeSubheading}[4]{
  \vspace{-2pt}\item
  \begin{tabular*}{0.97\textwidth}[t]{l@{\extracolsep{\fill}}r}
    \textbf{#1} & #2 \\
    \textit{\small #3} & \textit{\small #4} \\
  \end{tabular*}\vspace{-6pt}
}

\newcommand{\resumeProjectHeading}[2]{
  \vspace{-2pt}\item
  \begin{tabularx}{0.97\textwidth}[t]{@{}Xr@{}}
    \small\textbf{#1} & #2 \\
  \end{tabularx}\vspace{-6pt}
}

\newcommand{\resumeSubHeadingListStart}{\begin{itemize}[leftmargin=0.15in, label={}]}
\newcommand{\resumeSubHeadingListEnd}{\end{itemize}}
\newcommand{\resumeItemListStart}{\begin{itemize}[leftmargin=0.25in]}
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
        Command::new("where").arg(command).status()
    } else {
        Command::new("sh")
            .arg("-c")
            .arg(format!("command -v {}", command))
            .status()
    };

    status.map(|status| status.success()).unwrap_or(false)
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
    let should_build_pdf = args.iter().any(|arg| arg == "--pdf");
    let positional = args
        .iter()
        .filter(|arg| !arg.starts_with("--"))
        .cloned()
        .collect::<Vec<_>>();

    let input_file = positional.get(0).map(String::as_str).unwrap_or("resume.txt");
    let output_file = positional.get(1).map(String::as_str).unwrap_or("resume.tex");

    let source = match fs::read_to_string(input_file) {
        Ok(source) => source,
        Err(error) => {
            eprintln!("Could not read {}: {}", input_file, error);
            return ExitCode::FAILURE;
        }
    };

    let resume = parse_resume(&source);
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
