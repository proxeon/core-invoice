use clap::{Parser, Subcommand, ValueEnum};
use core_invoice::Profile;
use core_invoice_formats::{Syntax, convert, diff, validate_xml};
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser)]
#[command(
    name = "core-invoice",
    version,
    about = "EN 16931 + Peppol PINT, offline. 0.1.x is a skeleton."
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
    },
    /// Convert through the semantic model (UBL ↔ CII D16B)
    Convert {
        path: PathBuf,
        #[arg(long, value_enum)]
        to: SyntaxArg,
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

fn run() -> Result<ExitCode, String> {
    let cli = Cli::parse();
    match cli.command {
        Command::Validate { path, profile } => {
            let xml = fs::read_to_string(&path).map_err(|e| format!("{path:?}: {e}"))?;
            let report = match validate_xml(&xml, profile.forced()) {
                Ok(report) => report,
                Err(e) => return Err(e.to_string()),
            };
            if report.ok() {
                println!("valid ({})", report.profile_slug);
                Ok(ExitCode::SUCCESS)
            } else {
                print!("{report}");
                Ok(ExitCode::from(1))
            }
        }
        Command::Convert { path, to } => {
            let xml = fs::read_to_string(&path).map_err(|e| format!("{path:?}: {e}"))?;
            match convert(&xml, to.into()) {
                Ok(out) => {
                    print!("{out}");
                    Ok(ExitCode::SUCCESS)
                }
                Err(e) => Err(e.to_string()),
            }
        }
        Command::Diff { left, right } => {
            let a = fs::read_to_string(&left).map_err(|e| format!("{left:?}: {e}"))?;
            let b = fs::read_to_string(&right).map_err(|e| format!("{right:?}: {e}"))?;
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
            let xml = fs::read_to_string(&path).map_err(|e| format!("{path:?}: {e}"))?;
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
