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
    about = "EN 16931 + Peppol PINT, offline. 0.2.x is a development engine, not a legal validator. CII is a D16B subset for EN/Peppol; PINT-MY is UBL-only."
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Validate a UBL invoice against a profile
    Validate {
        /// Paths, or `-` for stdin. Several paths: worst exit (any 1 → 1; else any 2 → 2).
        paths: Vec<PathBuf>,
        /// `auto` reads BT-24 (CustomizationID). A named profile forces that rule set.
        #[arg(short, long, value_enum, default_value = "auto")]
        profile: ProfileArg,
        #[arg(long, value_enum, default_value = "text")]
        format: RulesFormat,
        /// No stdout on success. Invalid findings still print unless combined with json (json still on stdout).
        #[arg(short, long)]
        quiet: bool,
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
        /// Restrict to a profile's extras (peppol) plus CORE. Default: full catalogue.
        #[arg(short, long, value_enum)]
        profile: Option<ProfileArg>,
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
    if path.as_os_str() == "-" {
        use std::io::Read;
        let mut buf = String::new();
        std::io::stdin()
            .read_to_string(&mut buf)
            .map_err(|e| format!("stdin: {e}"))?;
        if buf.len() > core_invoice_formats::xml::MAX_INPUT_BYTES {
            return Err(format!(
                "stdin: input exceeds {} bytes",
                core_invoice_formats::xml::MAX_INPUT_BYTES
            ));
        }
        return Ok(buf);
    }
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
            paths,
            profile,
            format,
            quiet,
        } => {
            if paths.is_empty() {
                return Err("validate requires a path or - for stdin".into());
            }
            let mut any_invalid = false;
            let mut any_unreadable = false;
            for path in &paths {
                let xml = match read_xml(path) {
                    Ok(x) => x,
                    Err(e) => {
                        eprintln!("{e}");
                        any_unreadable = true;
                        continue;
                    }
                };
                let report = match validate_xml(&xml, profile.forced()) {
                    Ok(report) => report,
                    Err(e) => {
                        eprintln!("{e}");
                        any_unreadable = true;
                        continue;
                    }
                };
                let prefix = if paths.len() > 1 {
                    format!("{}: ", path.display())
                } else {
                    String::new()
                };
                match format {
                    RulesFormat::Json => {
                        if !quiet || !report.ok() {
                            println!("{prefix}{}", report_json(&report));
                        }
                    }
                    RulesFormat::Text => {
                        if report.ok() {
                            if !quiet {
                                println!("{prefix}valid ({})", report.profile_slug);
                            }
                        } else {
                            print!("{prefix}{report}");
                        }
                    }
                }
                if !report.ok() {
                    any_invalid = true;
                }
            }
            if any_invalid {
                Ok(ExitCode::from(1))
            } else if any_unreadable {
                Ok(ExitCode::from(2))
            } else {
                Ok(ExitCode::SUCCESS)
            }
        }
        Command::Convert {
            path,
            to,
            profile,
            output,
        } => {
            let xml = read_xml(&path)?;
            if let Ok(traced) = core_invoice_formats::read_with_trace(&xml) {
                for u in &traced.unmapped {
                    eprintln!("unmapped: {u}");
                }
                for m in &traced.malformed {
                    eprintln!("malformed: {m}");
                }
            }
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
        Command::Rules { format, profile } => {
            let rules: Vec<_> = match profile.and_then(|p| p.forced()) {
                Some(Profile::PeppolBis3) => core_invoice::core_rules()
                    .iter()
                    .chain(Profile::PeppolBis3.extra_rules())
                    .copied()
                    .collect(),
                Some(Profile::PintMy) => core_invoice::core_rules()
                    .iter()
                    .chain(Profile::PintMy.extra_rules())
                    .copied()
                    .collect(),
                _ => core_invoice::catalogue().to_vec(),
            };
            match format {
                RulesFormat::Text => {
                    for rule in &rules {
                        println!("{}\t{}\t{:?}", rule.id, rule.text, rule.source);
                    }
                }
                RulesFormat::Json => {
                    println!("[");
                    for (i, rule) in rules.iter().enumerate() {
                        let comma = if i + 1 == rules.len() { "" } else { "," };
                        println!(
                            "  {{\"id\":\"{}\",\"text\":\"{}\",\"source\":\"{:?}\"}}{comma}",
                            rule.id,
                            rule.text.replace('\\', "\\\\").replace('"', "\\\""),
                            rule.source
                        );
                    }
                    println!("]");
                }
            }
            Ok(ExitCode::SUCCESS)
        }
        Command::Inspect { path } => {
            let xml = read_xml(&path)?;
            let traced = core_invoice_formats::read_with_trace(&xml).map_err(|e| e.to_string())?;
            let inv = &traced.invoice;
            println!(
                "syntax={}",
                if xml.contains("CrossIndustryInvoice") {
                    "cii"
                } else {
                    "ubl"
                }
            );
            println!("number={}", inv.number);
            println!("profile={}", inv.profile.slug());
            println!("bt-24={}", inv.specification_id.as_deref().unwrap_or(""));
            println!("currency={}", inv.currency);
            println!("kind={:?}", inv.kind);
            println!("seller={}", inv.seller.name);
            println!("buyer={}", inv.buyer.name);
            println!("lines={}", inv.lines.len());
            if let Some(t) = inv.totals.as_ref() {
                println!("payable={}", t.payable);
            }
            for u in &traced.unmapped {
                println!("unmapped={u}");
            }
            Ok(ExitCode::SUCCESS)
        }
        Command::Profiles => {
            println!(
                "en16931\t{}\tVAT\t{}",
                Profile::En16931.specification_id(),
                core_invoice::ARTEFACT_VERSION
            );
            println!(
                "peppol\t{}\tVAT\t{}",
                Profile::PeppolBis3.specification_id(),
                core_invoice::PEPPOL_BIS_VERSION
            );
            println!(
                "pint\t{}\tVAT,GST,SST,consumption\t{}",
                Profile::Pint.specification_id(),
                core_invoice::PINT_VERSION
            );
            println!(
                "pint-my\t{}\tSST\t{}",
                Profile::PintMy.specification_id(),
                core_invoice::PINT_MY_VERSION
            );
            Ok(ExitCode::SUCCESS)
        }
    }
}
