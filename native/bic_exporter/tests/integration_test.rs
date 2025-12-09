use bic_exporter::{convert_bic_pdf_to_csv, extract_table_from_pdf, HEADERS};
use std::fs;
use std::path::PathBuf;
use tempfile::NamedTempFile;

fn fixtures_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
}

#[test]
fn test_headers_has_10_columns() {
    assert_eq!(HEADERS.len(), 10);
    assert_eq!(HEADERS[0], "Record creation date");
    assert_eq!(HEADERS[2], "BIC");
    assert_eq!(HEADERS[9], "Instit. Type");
}

#[test]
fn test_extract_table_from_pdf() {
    let pdf_path = fixtures_path().join("ISOBIC-mini.pdf");

    let rows = extract_table_from_pdf(&pdf_path).expect("Failed to extract table from PDF");

    // The mini PDF should have multiple records
    assert!(!rows.is_empty(), "Expected to find data rows");

    // Each row should have 10 columns
    for row in &rows {
        assert_eq!(
            row.len(),
            10,
            "Expected 10 columns, got {}: {:?}",
            row.len(),
            row
        );
    }
}

#[test]
fn test_extract_table_dates_format() {
    let pdf_path = fixtures_path().join("ISOBIC-mini.pdf");

    let rows = extract_table_from_pdf(&pdf_path).expect("Failed to extract table from PDF");

    let date_pattern = regex::Regex::new(r"^\d{4}-\d{2}-\d{2}$").unwrap();

    for row in &rows {
        // First column should be creation date
        assert!(
            date_pattern.is_match(&row[0]),
            "Creation date '{}' should match YYYY-MM-DD format",
            row[0]
        );
        // Second column should be update date
        assert!(
            date_pattern.is_match(&row[1]),
            "Update date '{}' should match YYYY-MM-DD format",
            row[1]
        );
    }
}

#[test]
fn test_extract_table_bic_format() {
    let pdf_path = fixtures_path().join("ISOBIC-mini.pdf");

    let rows = extract_table_from_pdf(&pdf_path).expect("Failed to extract table from PDF");

    let bic_pattern = regex::Regex::new(r"^[A-Z0-9]{8}$").unwrap();

    for row in &rows {
        // Third column should be BIC (8 alphanumeric characters)
        assert!(
            bic_pattern.is_match(&row[2]),
            "BIC '{}' should be 8 alphanumeric characters",
            row[2]
        );
    }
}

#[test]
fn test_convert_bic_pdf_to_csv_creates_output() {
    let pdf_path = fixtures_path().join("ISOBIC-mini.pdf");
    let output_file = NamedTempFile::new().expect("Failed to create temp file");
    let output_path = output_file.path().to_path_buf();

    let row_count =
        convert_bic_pdf_to_csv(&pdf_path, &output_path).expect("Failed to convert PDF to CSV");

    // The mini PDF should have some records
    assert!(
        row_count > 0,
        "Expected at least one record to be extracted"
    );

    // Verify the output file exists and has content
    let csv_content = fs::read_to_string(&output_path).expect("Failed to read output CSV");

    // Check that headers are present
    assert!(csv_content.contains("Record creation date"));
    assert!(csv_content.contains("BIC"));
    assert!(csv_content.contains("Instit. Type"));
}

#[test]
fn test_convert_bic_pdf_to_csv_output_is_valid_csv() {
    let pdf_path = fixtures_path().join("ISOBIC-mini.pdf");
    let output_file = NamedTempFile::new().expect("Failed to create temp file");
    let output_path = output_file.path().to_path_buf();

    convert_bic_pdf_to_csv(&pdf_path, &output_path).expect("Failed to convert PDF to CSV");

    // Parse the output as CSV to verify it's valid
    let mut reader = csv::Reader::from_path(&output_path).expect("Failed to open CSV for reading");

    // Verify headers
    let headers = reader.headers().expect("Failed to read CSV headers");
    assert_eq!(headers.len(), 10);
    assert_eq!(&headers[0], "Record creation date");
    assert_eq!(&headers[2], "BIC");

    // Count records
    let record_count = reader.records().count();
    assert!(record_count > 0, "Expected CSV to have data records");
}

#[test]
fn test_known_bic_codes_are_extracted() {
    let pdf_path = fixtures_path().join("ISOBIC-mini.pdf");

    let rows = extract_table_from_pdf(&pdf_path).expect("Failed to extract table from PDF");

    // Collect all BIC codes (column 2)
    let bic_codes: Vec<&str> = rows.iter().map(|r| r[2].as_str()).collect();

    // Check for some known BIC codes from the mini PDF
    let expected_bics = ["AAAARSBG", "AAACKWKW", "AAADFRP1"];

    for expected in &expected_bics {
        assert!(
            bic_codes.contains(expected),
            "Expected to find BIC code {} in extracted records. Found: {:?}",
            expected,
            bic_codes
        );
    }
}

#[test]
fn test_output_matches_expected_csv() {
    let pdf_path = fixtures_path().join("ISOBIC-mini.pdf");
    let expected_path = fixtures_path().join("ISOBIC-mini-expected.csv");
    let output_file = NamedTempFile::new().expect("Failed to create temp file");
    let output_path = output_file.path().to_path_buf();

    convert_bic_pdf_to_csv(&pdf_path, &output_path).expect("Failed to convert PDF to CSV");

    // Read both CSVs and compare
    let mut expected_reader =
        csv::Reader::from_path(&expected_path).expect("Failed to open expected CSV");
    let mut actual_reader =
        csv::Reader::from_path(&output_path).expect("Failed to open actual CSV");

    // Compare headers
    let expected_headers = expected_reader
        .headers()
        .expect("Failed to read expected headers");
    let actual_headers = actual_reader
        .headers()
        .expect("Failed to read actual headers");
    assert_eq!(
        expected_headers, actual_headers,
        "Headers don't match.\nExpected: {:?}\nActual: {:?}",
        expected_headers, actual_headers
    );

    // Compare records row by row
    let expected_records: Vec<csv::StringRecord> = expected_reader
        .records()
        .collect::<Result<Vec<_>, _>>()
        .expect("Failed to read expected records");
    let actual_records: Vec<csv::StringRecord> = actual_reader
        .records()
        .collect::<Result<Vec<_>, _>>()
        .expect("Failed to read actual records");

    assert_eq!(
        expected_records.len(),
        actual_records.len(),
        "Row count mismatch. Expected {} rows, got {} rows",
        expected_records.len(),
        actual_records.len()
    );

    for (i, (expected, actual)) in expected_records
        .iter()
        .zip(actual_records.iter())
        .enumerate()
    {
        assert_eq!(
            expected,
            actual,
            "Row {} mismatch.\nExpected: {:?}\nActual: {:?}",
            i + 1,
            expected,
            actual
        );
    }
}
