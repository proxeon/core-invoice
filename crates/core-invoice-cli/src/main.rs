use clap::{Parser, Subcommand, ValueEnum};
use core_invoice::Profile;
use core_invoice_formats::{
    FormatError, SemanticReject, Syntax, convert_with_profile, diff, validate_xml,
};
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser)]
#[command(
    name = "core-invoice",
    version,
    about = "EN 16931 + Peppol PINT, offline. 0.1.x is a skeleton. CII is a D16B subset for EN/Peppol; PINT-MY is UBL-only."
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Validate a UBL invoice against a profile
    Validate {
        path: PathBuf,
        /// `auto` reads BT-24 (CustomizationID). A named profile forces that rule set.
        #[arg(short, long, value_enum, default_value = "auto")]
        profile: ProfileArg,
        #[arg(long, value_enum, default_value = "text")]
        format: RulesFormat,
    },
    /// Convert through the semantic model (UBL; CII subset for EN/Peppol, not PINT-MY)
    Convert {
        path: PathBuf,
        #[arg(long, value_enum)]
        to: SyntaxArg,
        /// `auto` reads BT-24 (CustomizationID). A named profile forces that rule set.
        #[arg(short, long, value_enum, default_value = "auto")]
        profile: ProfileArg,
        /// Write XML to this path; stdout stays empty on success.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Semantic diff of two documents (exit 1 if they differ)
    Diff { left: PathBuf, right: PathBuf },
    /// Explain a rule id
    Explain { id: String },
    /// Dump the rule catalogue
    Rules {
        #[arg(long, default_value = "text")]
        format: RulesFormat,
    },
    /// Print model fields without a valid/invalid verdict
    Inspect { path: PathBuf },
    /// List profile slugs
    Profiles,
}

#[derive(Clone, Copy, ValueEnum)]
enum RulesFormat {
    Text,
    Json,
}

#[derive(Clone, Copy, ValueEnum)]
enum ProfileArg {
    Auto,
    En16931,
    Peppol,
    Pint,
    PintMy,
}

impl ProfileArg {
    fn forced(self) -> Option<Profile> {
        match self {
            Self::Auto => None,
            Self::En16931 => Some(Profile::En16931),
            Self::Peppol => Some(Profile::PeppolBis3),
            Self::Pint => Some(Profile::Pint),
            Self::PintMy => Some(Profile::PintMy),
        }
    }
}

#[derive(Clone, Copy, ValueEnum)]
enum SyntaxArg {
    Ubl,
    Cii,
}

impl From<SyntaxArg> for Syntax {
    fn from(value: SyntaxArg) -> Self {
        match value {
            SyntaxArg::Ubl => Syntax::Ubl,
            SyntaxArg::Cii => Syntax::Cii,
        }
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::from(2)
        }
    }
}

fn report_json(report: &core_invoice::Report) -> String {
    let mut findings = String::new();
    for (i, f) in report.findings.iter().enumerate() {
        if i > 0 {
            findings.push(',');
        }
        let msg = f
            .message
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n");
        findings.push_str(&format!(
            r#"{{"id":"{}","severity":"{:?}","path":"{}","message":"{msg}"}}"#,
            f.id, f.severity, f.path
        ));
    }
    format!(
        r#"{{"ok":{},"profile":"{}","findings":[{findings}]}}"#,
        if report.ok() { "true" } else { "false" },
        report.profile_slug
    )
}

fn write_stdout(s: &str) -> Result<(), String> {
    use std::io::{self, Write};
    match io::stdout().write_all(s.as_bytes()) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == io::ErrorKind::BrokenPipe => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}

fn read_xml(path: &PathBuf) -> Result<String, String> {
    // Hostile or mistaken multi-GB “invoice” is size, not a valid document.
    if let Ok(meta) = fs::metadata(path)
        && meta.len() > core_invoice_formats::xml::MAX_INPUT_BYTES as u64
    {
        return Err(format!(
            "{path:?}: input exceeds {} bytes",
            core_invoice_formats::xml::MAX_INPUT_BYTES
        ));
    }
    fs::read_to_string(path).map_err(|e| format!("{path:?}: {e}"))
}

fn run() -> Result<ExitCode, String> {
    let cli = Cli::parse();
    match cli.command {
        Command::Validate {
            path,
            profile,
            format,
        } => {
            let xml = read_xml(&path)?;
            let report = match validate_xml(&xml, profile.forced()) {
                Ok(report) => report,
                Err(e) => return Err(e.to_string()),
            };
            match format {
                RulesFormat::Json => {
                    println!("{}", report_json(&report));
                }
                RulesFormat::Text => {
                    if report.ok() {
                        println!("valid ({})", report.profile_slug);
                    } else {
                        print!("{report}");
                    }
                }
            }
            if report.ok() {
                Ok(ExitCode::SUCCESS)
            } else {
                Ok(ExitCode::from(1))
            }
        }
        Command::Convert {
            path,
            to,
            profile,
            output,
        } => {
            let xml = read_xml(&path)?;
            match convert_with_profile(&xml, to.into(), profile.forced()) {
                Ok(out) => {
                    if let Some(dest) = output {
                        fs::write(&dest, &out).map_err(|e| format!("{dest:?}: {e}"))?;
                    } else if let Err(e) = write_stdout(&out) {
                        return Err(e);
                    }
                    Ok(ExitCode::SUCCESS)
                }
                Err(FormatError::Semantic(SemanticReject(report))) => {
                    // Fatal: findings on stdout like validate; no XML.
                    print!("{report}");
                    Ok(ExitCode::from(1))
                }
                // FormatError (parse, CiiNotForProfile, …) is not a semantic finding.
                // CLI contract: unreadable / refused syntax → exit 2 on stderr.
                Err(e) => Err(e.to_string()),
            }
        }
        Command::Diff { left, right } => {
            let a = read_xml(&left)?;
            let b = read_xml(&right)?;
            let out = diff(&a, &b).map_err(|e| e.to_string())?;
            println!("{out}");
            if out == "no semantic difference" {
                Ok(ExitCode::SUCCESS)
            } else {
                Ok(ExitCode::from(1))
            }
        }
        Command::Explain { id } => match core_invoice::explain(&id) {
            Some(text) => {
                println!("{text}");
                Ok(ExitCode::SUCCESS)
            }
            None => {
                eprintln!("no explanation registered for {id}");
                Ok(ExitCode::from(2))
            }
        },
        Command::Rules { format } => {
            match format {
                RulesFormat::Text => {
                    for rule in core_invoice::catalogue() {
                        println!("{}	{}", rule.id, rule.text);
                    }
                }
                RulesFormat::Json => {
                    println!("[");
                    let rules = core_invoice::catalogue();
                    for (i, rule) in rules.iter().enumerate() {
                        let comma = if i + 1 == rules.len() { "" } else { "," };
                        println!(
                            "  {{\"id\":\"{}\",\"text\":\"{}\"}}{comma}",
                            rule.id,
                            rule.text.replace('\\', "\\\\").replace('"', "\\\"")
                        );
                    }
                    println!("]");
                }
            }
            Ok(ExitCode::SUCCESS)
        }
        Command::Inspect { path } => {
            let xml = read_xml(&path)?;
            let inv = core_invoice_formats::read(&xml).map_err(|e| e.to_string())?;
            println!("number={}", inv.number);
            println!("profile={}", inv.profile.slug());
            println!("currency={}", inv.currency);
            println!("kind={:?}", inv.kind);
            println!("lines={}", inv.lines.len());
            Ok(ExitCode::SUCCESS)
        }
        Command::Profiles => {
            println!("{}", Profile::known_slugs());
            Ok(ExitCode::SUCCESS)
        }
    }
}
