use clap::{Parser, Subcommand, ValueEnum};
use core_invoice::Profile;
use core_invoice_formats::{convert, diff, validate_xml, Syntax};
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser)]
#[command(name = "core-invoice", version, about = "EN 16931 + Peppol PINT, offline")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Validate a UBL or CII invoice against a profile
    Validate {
        path: PathBuf,
        #[arg(short, long, value_enum, default_value = "pint")]
        profile: ProfileArg,
    },
    /// Convert through the semantic model
    Convert {
        path: PathBuf,
        #[arg(long, value_enum)]
        to: SyntaxArg,
    },
    /// Semantic diff of two documents
    Diff { left: PathBuf, right: PathBuf },
    /// Explain a rule id
    Explain { id: String },
}

#[derive(Clone, Copy, ValueEnum)]
enum ProfileArg {
    En16931,
    Peppol,
    Pint,
    PintMy,
}

impl From<ProfileArg> for Profile {
    fn from(value: ProfileArg) -> Self {
        match value {
            ProfileArg::En16931 => Profile::En16931,
            ProfileArg::Peppol => Profile::PeppolBis3,
            ProfileArg::Pint => Profile::Pint,
            ProfileArg::PintMy => Profile::PintMy,
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
            let report = validate_xml(&xml, Some(profile.into())).map_err(|e| e.to_string())?;
            if report.ok() {
                println!("valid ({})", Profile::from(profile).slug());
                Ok(ExitCode::SUCCESS)
            } else {
                eprint!("{report}");
                Ok(ExitCode::from(1))
            }
        }
        Command::Convert { path, to } => {
            let xml = fs::read_to_string(&path).map_err(|e| format!("{path:?}: {e}"))?;
            let out = convert(&xml, to.into()).map_err(|e| e.to_string())?;
            print!("{out}");
            Ok(ExitCode::SUCCESS)
        }
        Command::Diff { left, right } => {
            let a = fs::read_to_string(&left).map_err(|e| format!("{left:?}: {e}"))?;
            let b = fs::read_to_string(&right).map_err(|e| format!("{right:?}: {e}"))?;
            println!("{}", diff(&a, &b).map_err(|e| e.to_string())?);
            Ok(ExitCode::SUCCESS)
        }
        Command::Explain { id } => {
            println!("{}", explain(&id));
            Ok(ExitCode::SUCCESS)
        }
    }
}

fn explain(id: &str) -> String {
    match id.to_ascii_uppercase().as_str() {
        "BR-02" => "Invoice number (BT-1) shall be present.".into(),
        "BR-05" => "Invoice currency code (BT-5) shall be present.".into(),
        "BR-16" => "An invoice shall have at least one invoice line (BG-25).".into(),
        "BR-CO-16" => "Amount due for payment (BT-115) = invoice total with tax (BT-112) − paid (BT-113) + rounding (BT-114). Here: payable = line net + tax total.".into(),
        "PINT-TAX" => "Tax system on a line must be allowed by the profile. EN 16931 / Peppol BIS 3.0: VAT only. PINT / PINT-MY: VAT, GST, SST, consumption.".into(),
        "PINT-MY-ID" => "PINT-MY seller identification scheme must be TIN, BRN, NRIC or PASSPORT.".into(),
        other => format!("no explanation registered for {other}"),
    }
}
