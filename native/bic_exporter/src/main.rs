use anyhow::Result;
use bic_exporter::convert_bic_pdf_to_csv;
use clap::Parser;
use std::path::PathBuf;

/// Convert BIC directory PDF to CSV format
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the source PDF file
    #[arg(short, long, default_value = "ISOBIC.pdf")]
    source: PathBuf,

    /// Path to the destination CSV file
    #[arg(short, long, default_value = "ISOBIC.csv")]
    destination: PathBuf,
}

fn main() -> Result<()> {
    let args = Args::parse();

    println!(
        "Converting {} to {}...",
        args.source.display(),
        args.destination.display()
    );

    let row_count = convert_bic_pdf_to_csv(&args.source, &args.destination)?;

    println!(
        "Extracted {} records to {}",
        row_count,
        args.destination.display()
    );

    Ok(())
}
