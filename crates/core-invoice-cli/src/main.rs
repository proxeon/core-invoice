use clap::{Parser, Subcommand, ValueEnum};
use core_invoice::Profile;
use core_invoice_formats::{FormatError, Syntax, convert, diff, validate_xml};
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
    /// Convert through the semantic model (UBL only until CII D16B exists)
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
                Err(FormatError::CiiNotImplemented) => {
                    eprintln!("{}", FormatError::CiiNotImplemented);
                    Ok(ExitCode::from(2))
                }
                Err(e) => Err(e.to_string()),
            }
        }
        Command::Diff { left, right } => {
            let a = fs::read_to_string(&left).map_err(|e| format!("{left:?}: {e}"))?;
            let b = fs::read_to_string(&right).map_err(|e| format!("{right:?}: {e}"))?;
            println!("{}", diff(&a, &b).map_err(|e| e.to_string())?);
            Ok(ExitCode::SUCCESS)
        }
        Command::Explain { id } => match explain(&id) {
            Ok(text) => {
                println!("{text}");
                Ok(ExitCode::SUCCESS)
            }
            Err(text) => {
                eprintln!("{text}");
                Ok(ExitCode::from(2))
            }
        },
    }
}

fn explain(id: &str) -> Result<String, String> {
    match id.to_ascii_uppercase().as_str() {
        "BR-02" => Ok("Invoice number (BT-1) shall be present.".into()),
        "BR-05" => Ok("Invoice currency code (BT-5) shall be present.".into()),
        "BR-06" => Ok("Seller name (BT-27) shall be present.".into()),
        "BR-07" => Ok("Buyer name (BT-44) shall be present.".into()),
        "BR-16" => Ok("An invoice shall have at least one invoice line (BG-25).".into()),
        "BR-CO-16" => Ok(
            "Amount due for payment (BT-115) = invoice total with tax (BT-112) − paid (BT-113) + rounding (BT-114). Not evaluated until document totals exist."
                .into(),
        ),
        "PINT-TAX" => Ok(
            "Tax system on a line must be allowed by the profile. EN 16931 / Peppol BIS 3.0: VAT only. PINT / PINT-MY: VAT, GST, SST, consumption."
                .into(),
        ),
        "PINT-MY-ID" => Ok(
            "PINT-MY seller identification scheme must be TIN, BRN, NRIC or PASSPORT. Replaced by IBR-02/03/04-MY once party identifiers are split."
                .into(),
        ),
        other => Err(format!("no explanation registered for {other}")),
    }
}
