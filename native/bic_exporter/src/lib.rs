//! # BIC Exporter
//!
//! Extracts BIC (Bank Identifier Code) directory data from ISO 9362 PDF files
//! and converts it to CSV format.
//!
//! ## Why Position-Based Extraction?
//!
//! The ISO BIC directory PDF doesn't use standard PDF table structures that
//! libraries can easily parse. Instead, it renders text at specific X/Y
//! coordinates. We must:
//!
//! 1. Extract each text fragment with its position
//! 2. Group fragments into rows by Y coordinate (with tolerance for slight variations)
//! 3. Assign fragments to columns based on X position and table line boundaries
//!
//! ## Multi-Line Record Handling
//!
//! Address fields (registered address, operational address, branch address) often
//! span multiple lines in the PDF. A single BIC record may occupy 2-4 visual rows.
//! We detect record boundaries by looking for the creation date (YYYY-MM-DD format)
//! in the first column - only new records start with a date. Rows without a date
//! are continuation rows that get merged into the current record.

use anyhow::{Context, Result};
use csv::Writer;
use pdf::content::{Op, TextDrawAdjusted};
use pdf::file::FileOptions;
use rustler::Binary;
use std::collections::BTreeMap;
use std::fs::File;
use std::path::Path;

// PDF text extraction constants
//
// These values are tuned for the ISO BIC directory PDF format.
// The PDF uses specific spacing and layout that these constants accommodate.

/// Default line height when TextNewline operator doesn't specify leading.
const DEFAULT_LINE_HEIGHT: f32 = 12.0;

/// Threshold for detecting word spaces in TJ operator arrays.
/// PDF TJ arrays use negative values for kerning; values below this threshold
/// indicate intentional word spacing rather than letter kerning.
const SPACE_THRESHOLD: f32 = -100.0;

/// PDF text spacing is specified in thousandths of the text space unit.
const SPACING_DIVISOR: f32 = 1000.0;

/// Tolerance for grouping text elements into the same row.
/// Text fragments within this Y-distance are considered part of the same line.
const Y_TOLERANCE: f32 = 3.0;

/// Tolerance for detecting vertical lines (table column separators).
const VERTICAL_LINE_TOLERANCE: f32 = 1.0;

/// Tolerance for deduplicating detected vertical lines.
const LINE_DEDUP_TOLERANCE: f32 = 2.0;

/// Required number of column boundaries (10 columns = 11 boundaries including end marker)
const REQUIRED_BOUNDARIES: usize = 11;

pub const HEADERS: [&str; 10] = [
    "Record creation date",
    "Last Update date",
    "BIC",
    "Brch Code",
    "Full legal name",
    "Registered address",
    "Operational address",
    "Branch description",
    "Branch address",
    "Instit. Type",
];

/// A text element extracted from PDF with its page position.
///
/// PDFs don't have a concept of "cells" or "rows" - they just place text
/// at specific coordinates. We extract each text fragment with its position
/// so we can later reconstruct the table structure.
#[derive(Debug, Clone)]
struct TextElement {
    text: String,
    x: f32,
    y: f32,
}

/// A reconstructed row of text elements, grouped by Y coordinate.
///
/// After extracting individual text elements, we group them by their Y position
/// (with tolerance) to form logical rows. Within each row, cells are sorted
/// by X position for left-to-right reading order.
#[derive(Debug, Clone)]
struct TableRow {
    y: f32,
    cells: Vec<(f32, String)>, // (x_position, text)
}

/// Decode a PDF string to UTF-8.
///
/// PDF strings can be encoded as UTF-16BE (with BOM) or PDFDocEncoding/Latin-1.
/// The BIC directory uses UTF-16BE for text with special characters
/// (accented names, non-ASCII addresses).
fn decode_pdf_string(text: &pdf::primitive::PdfString) -> String {
    let bytes = text.as_bytes();

    if bytes.len() >= 2 && bytes[0] == 0xFE && bytes[1] == 0xFF {
        // UTF-16BE with BOM
        let chars: Vec<u16> = bytes[2..]
            .chunks(2)
            .filter_map(|chunk| {
                if chunk.len() == 2 {
                    Some(u16::from_be_bytes([chunk[0], chunk[1]]))
                } else {
                    None
                }
            })
            .collect();
        String::from_utf16_lossy(&chars)
    } else {
        // Try as Latin-1 / PDFDocEncoding
        bytes.iter().map(|&b| b as char).collect()
    }
}

/// Extract text elements with positions from PDF content stream operations.
///
/// PDF content streams contain operators that draw text at specific positions.
/// We track the current text position through operators (BT, Tm, Td, Tj, TJ)
/// to capture each text fragment with its X/Y coordinates.
fn extract_text_from_ops(ops: &[Op]) -> Vec<TextElement> {
    let mut elements = Vec::new();

    // Current transformation matrix state
    let mut current_x: f32 = 0.0;
    let mut current_y: f32 = 0.0;
    let mut text_matrix_x: f32 = 0.0;
    let mut _text_matrix_y: f32 = 0.0;

    for op in ops {
        match op {
            // Text positioning operators
            Op::TextNewline => {
                // Move to next line (Td with default leading)
                current_y -= DEFAULT_LINE_HEIGHT;
                current_x = text_matrix_x;
            }
            Op::MoveTextPosition { translation } => {
                // Td operator - move text position
                current_x += translation.x;
                current_y += translation.y;
            }
            Op::SetTextMatrix { matrix } => {
                // Tm operator - set text matrix
                current_x = matrix.e;
                current_y = matrix.f;
                text_matrix_x = matrix.e;
                _text_matrix_y = matrix.f;
            }
            Op::BeginText => {
                // BT operator - reset text position
                current_x = 0.0;
                current_y = 0.0;
            }
            // Text showing operators
            Op::TextDraw { text } => {
                let decoded = decode_pdf_string(text);
                if !decoded.trim().is_empty() {
                    elements.push(TextElement {
                        text: decoded,
                        x: current_x,
                        y: current_y,
                    });
                }
            }
            Op::TextDrawAdjusted { array } => {
                let mut combined_text = String::new();
                let start_x = current_x;

                for item in array.iter() {
                    match item {
                        TextDrawAdjusted::Text(text) => {
                            combined_text.push_str(&decode_pdf_string(text));
                        }
                        TextDrawAdjusted::Spacing(spacing) => {
                            // Large negative spacing often indicates a space
                            if *spacing < SPACE_THRESHOLD {
                                combined_text.push(' ');
                            }
                            // Adjust x position (spacing is in thousandths of text space unit)
                            current_x -= spacing / SPACING_DIVISOR;
                        }
                    }
                }

                if !combined_text.trim().is_empty() {
                    elements.push(TextElement {
                        text: combined_text,
                        x: start_x,
                        y: current_y,
                    });
                }
            }
            _ => {}
        }
    }

    elements
}

/// Group text elements into rows based on Y coordinate.
///
/// PDF Y coordinates increase upward (origin at bottom-left), so we sort
/// rows in descending Y order to get top-to-bottom reading order.
/// Elements within `y_tolerance` are grouped into the same row.
fn group_into_rows(elements: Vec<TextElement>, y_tolerance: f32) -> Vec<TableRow> {
    if elements.is_empty() {
        return Vec::new();
    }

    // Group by Y coordinate with tolerance
    let mut rows_map: BTreeMap<i32, Vec<(f32, String)>> = BTreeMap::new();

    for elem in elements {
        // Round Y to nearest tolerance unit for grouping
        let y_key = (elem.y / y_tolerance).round() as i32;
        rows_map.entry(y_key).or_default().push((elem.x, elem.text));
    }

    // Convert to TableRow and sort cells by X position
    let mut rows: Vec<TableRow> = rows_map
        .into_iter()
        .map(|(y_key, mut cells)| {
            cells.sort_by(|a, b| a.0.total_cmp(&b.0));
            TableRow {
                y: y_key as f32 * y_tolerance,
                cells,
            }
        })
        .collect();

    // Sort rows by Y (descending, since PDF Y increases upward)
    rows.sort_by(|a, b| b.y.total_cmp(&a.y));

    rows
}

/// Extract column boundaries from PDF table lines.
///
/// The BIC directory PDF draws vertical lines to separate columns.
/// We detect these by finding MoveTo/LineTo pairs with the same X coordinate,
/// which lets us accurately assign text to columns.
fn extract_column_boundaries_from_ops(ops: &[Op]) -> Vec<f32> {
    let mut vertical_lines: Vec<f32> = Vec::new();

    // Look for vertical lines (same X for MoveTo and LineTo)
    let mut last_move_x: Option<f32> = None;

    for op in ops {
        match op {
            Op::MoveTo { p } => {
                last_move_x = Some(p.x);
            }
            Op::LineTo { p } => {
                if let Some(move_x) = last_move_x {
                    // Check if this is a vertical line (same X position)
                    if (move_x - p.x).abs() < VERTICAL_LINE_TOLERANCE {
                        vertical_lines.push(move_x);
                    }
                }
                last_move_x = None;
            }
            _ => {}
        }
    }

    // Remove duplicates and sort
    vertical_lines.sort_by(|a, b| a.total_cmp(b));
    vertical_lines.dedup_by(|a, b| (*a - *b).abs() < LINE_DEDUP_TOLERANCE);

    // Add end boundary
    if !vertical_lines.is_empty() {
        vertical_lines.push(f32::MAX);
    }

    vertical_lines
}

/// Assign cells to columns based on X position
fn assign_cells_to_columns(row: &TableRow, boundaries: &[f32]) -> Vec<String> {
    let num_columns = boundaries.len() - 1;
    let mut columns: Vec<String> = vec![String::new(); num_columns];

    for (x, text) in &row.cells {
        // Find which column this cell belongs to
        for i in 0..num_columns {
            if *x >= boundaries[i] && *x < boundaries[i + 1] {
                if !columns[i].is_empty() {
                    columns[i].push(' ');
                }
                columns[i].push_str(text);
                break;
            }
        }
    }

    // Clean up: normalize whitespace in each column
    columns
        .into_iter()
        .map(|s| s.split_whitespace().collect::<Vec<_>>().join(" "))
        .collect()
}

/// Check if a row is a header row that should be skipped.
///
/// The PDF repeats column headers on each page. We detect these by looking
/// for characteristic header text and exclude them from the output.
fn is_header_row(cells: &[String]) -> bool {
    let combined = cells.join(" ").to_lowercase();
    combined.contains("record") && combined.contains("creation")
        || combined.contains("last update")
        || combined.contains("brch code")
        || combined.contains("bic brch")
        || combined.contains("full legal name")
        || combined.contains("instit. type")
        || combined.contains("inst. type")
        || combined.contains("iso bic directory")
        || combined.contains("registration authority")
        || combined.contains("iso 9362")
}

/// Check if a row starts a new data record (has a date in the first column).
///
/// BIC records always start with a creation date in YYYY-MM-DD format.
/// Rows without a date are continuation rows containing wrapped address content.
fn is_data_row(cells: &[String]) -> bool {
    if cells.is_empty() || cells[0].is_empty() {
        return false;
    }

    // Check if first cell looks like a date (YYYY-MM-DD)
    let first = cells[0].trim();
    if first.len() >= 10 {
        let parts: Vec<&str> = first.split('-').collect();
        if parts.len() >= 3 {
            return parts[0].len() == 4
                && parts[0].chars().all(|c| c.is_ascii_digit())
                && parts[1].len() == 2
                && parts[2].len() >= 2;
        }
    }

    false
}

/// Merge a continuation row into the current record.
///
/// Address fields often span multiple lines in the PDF. Continuation rows
/// (those without a date) contain wrapped content that belongs to the
/// previous record. We append each column's content to preserve multi-line
/// addresses as single fields.
fn merge_continuation_row(record: &mut [String], continuation: &[String]) {
    for (i, cell) in continuation.iter().enumerate() {
        if i < record.len() && !cell.is_empty() {
            if !record[i].is_empty() {
                record[i].push(' ');
            }
            record[i].push_str(cell);
        }
    }
}

/// Process a page's content and extract complete records.
///
/// This is the core extraction logic: extract positioned text, group into rows,
/// assign to columns, identify record boundaries (rows starting with dates),
/// and merge continuation rows into their parent records.
fn process_page_rows(ops: &[Op], boundaries: &[f32]) -> Vec<Vec<String>> {
    let elements = extract_text_from_ops(ops);
    if elements.is_empty() {
        return Vec::new();
    }

    let rows = group_into_rows(elements, Y_TOLERANCE);
    if rows.is_empty() {
        return Vec::new();
    }

    let mut records: Vec<Vec<String>> = Vec::new();
    let mut current_record: Option<Vec<String>> = None;

    for row in &rows {
        let cells = assign_cells_to_columns(row, boundaries);

        // Skip empty rows
        if cells.iter().all(|c| c.is_empty()) {
            continue;
        }

        // Skip header rows
        if is_header_row(&cells) {
            continue;
        }

        // Check if this is a new data row (starts with a date) or a continuation
        if is_data_row(&cells) {
            // Save the previous record if any
            if let Some(record) = current_record.take() {
                records.push(record);
            }
            // Start a new record
            current_record = Some(cells.iter().map(|c| c.trim().to_string()).collect());
        } else if let Some(record) = current_record.as_mut() {
            // This is a continuation row - merge it with the current record
            merge_continuation_row(record, &cells);
        }
    }

    // Don't forget the last record
    if let Some(record) = current_record.take() {
        records.push(record);
    }

    records
}

/// Extract table data from PDF bytes in memory.
///
/// Processes all pages (except the cover page) and extracts BIC records.
/// Column boundaries are detected from the first data page and reused
/// for consistency across all pages.
pub fn extract_table_from_bytes(data: Vec<u8>) -> Result<Vec<Vec<String>>> {
    let file = FileOptions::cached()
        .load(data)
        .context("Failed to load PDF from bytes")?;

    extract_table_from_file(file)
}

/// Extract table data from a PDF file path.
///
/// Processes all pages (except the cover page) and extracts BIC records.
/// Column boundaries are detected from the first data page and reused
/// for consistency across all pages.
pub fn extract_table_from_pdf(source: &Path) -> Result<Vec<Vec<String>>> {
    let file = FileOptions::cached()
        .open(source)
        .context("Failed to open PDF file")?;

    extract_table_from_file(file)
}

/// Internal function to extract table data from a loaded PDF file.
fn extract_table_from_file<T: std::ops::Deref<Target = [u8]>>(
    file: pdf::file::CachedFile<T>,
) -> Result<Vec<Vec<String>>> {
    let resolver = file.resolver();
    let mut all_rows: Vec<Vec<String>> = Vec::new();
    let mut boundaries: Option<Vec<f32>> = None;

    // Process each page
    for (page_num, page_result) in file.pages().enumerate() {
        let page = page_result.context(format!("Failed to get page {}", page_num))?;

        // Skip cover page (page 0)
        if page_num == 0 {
            continue;
        }

        // Get content operations
        let contents = match &page.contents {
            Some(c) => c,
            None => continue,
        };

        let ops = contents
            .operations(&resolver)
            .context(format!("Failed to parse operations on page {}", page_num))?;

        // Extract column boundaries from table lines on first data page
        if boundaries.is_none() {
            let mut detected = extract_column_boundaries_from_ops(&ops);
            if detected.len() >= REQUIRED_BOUNDARIES {
                detected.truncate(REQUIRED_BOUNDARIES);
                boundaries = Some(detected);
            } else {
                anyhow::bail!(
                    "Failed to detect column boundaries from PDF. Expected at least {} vertical lines, found {}. \
                     This PDF may have a different format than the standard ISO BIC directory.",
                    REQUIRED_BOUNDARIES,
                    detected.len()
                );
            }
        }

        let page_records = process_page_rows(&ops, boundaries.as_ref().unwrap());
        all_rows.extend(page_records);
    }

    Ok(all_rows)
}

/// Convert a BIC directory PDF to CSV format
///
/// Returns the number of records extracted.
pub fn convert_bic_pdf_to_csv(source: &Path, destination: &Path) -> Result<usize> {
    let rows = extract_table_from_pdf(source)?;

    // Write to CSV
    let file = File::create(destination).context("Failed to create output CSV file")?;
    let mut writer = Writer::from_writer(file);

    // Write headers
    writer
        .write_record(HEADERS)
        .context("Failed to write CSV headers")?;

    // Write data rows
    let row_count = rows.len();
    for row in rows {
        writer
            .write_record(&row)
            .context("Failed to write CSV row")?;
    }

    writer.flush().context("Failed to flush CSV writer")?;

    Ok(row_count)
}

// =============================================================================
// NIF Functions for Elixir/Erlang integration via Rustler
// =============================================================================

/// NIF: Extract BIC records from a PDF file path.
///
/// Returns `{:ok, records}` on success or `{:error, reason}` on failure.
/// Each record is a list of 10 strings corresponding to the CSV columns.
#[rustler::nif(schedule = "DirtyIo")]
fn extract_table_from_path(source: String) -> Result<Vec<Vec<String>>, String> {
    extract_table_from_pdf(Path::new(&source)).map_err(|e| e.to_string())
}

/// NIF: Extract BIC records from PDF binary data.
///
/// Returns `{:ok, records}` on success or `{:error, reason}` on failure.
/// Each record is a list of 10 strings corresponding to the CSV columns.
///
/// This is useful when the PDF is already loaded in memory (e.g., downloaded
/// from a URL or read from a database).
#[rustler::nif(schedule = "DirtyIo")]
fn extract_table_from_binary(data: Binary) -> Result<Vec<Vec<String>>, String> {
    extract_table_from_bytes(data.as_slice().to_vec()).map_err(|e| e.to_string())
}

/// NIF: Convert a BIC directory PDF to CSV format.
///
/// Returns `{:ok, record_count}` on success or `{:error, reason}` on failure.
#[rustler::nif(schedule = "DirtyIo")]
fn convert_to_csv(source: String, destination: String) -> Result<usize, String> {
    convert_bic_pdf_to_csv(Path::new(&source), Path::new(&destination)).map_err(|e| e.to_string())
}

/// NIF: Get the CSV column headers.
///
/// Returns the list of column headers used in the CSV output.
#[rustler::nif]
fn headers() -> Vec<&'static str> {
    HEADERS.to_vec()
}

// =============================================================================
// Test-only NIFs to verify Rustler prevents BEAM crashes
// =============================================================================
/// NIFs that deliberately panic/unwrap to verify Rustler's runtime safety.
/// This is only compiled when the `panic_test` feature is enabled.
/// Rustler should catch this panic and convert it to an Erlang error
/// rather than crashing the BEAM.
#[cfg(feature = "panic_test")]
#[rustler::nif]
fn deliberate_panic() -> bool {
    panic!("deliberate_panic: testing Rustler panic safety");
}

#[cfg(feature = "panic_test")]
#[rustler::nif]
#[allow(clippy::unnecessary_literal_unwrap)]
fn deliberate_unwrap_panic() -> bool {
    let option: Option<bool> = None;
    option.expect("deliberate_unwrap_panic: testing Rustler panic safety")
}

rustler::init!("Elixir.BicExporter.Native");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_header_row() {
        assert!(is_header_row(&["Record creation date".to_string()]));
        assert!(is_header_row(&["BIC Brch Code".to_string()]));
        assert!(!is_header_row(&["1997-03-01".to_string()]));
    }

    #[test]
    fn test_is_data_row() {
        assert!(is_data_row(&[
            "1997-03-01".to_string(),
            "2024-06-06".to_string()
        ]));
        assert!(is_data_row(&["2021-05-22".to_string()]));
        assert!(!is_data_row(&["Record".to_string()]));
        assert!(!is_data_row(&["".to_string()]));
    }

    #[test]
    fn test_is_data_row_edge_cases() {
        // Too short
        assert!(!is_data_row(&["2021-05".to_string()]));
        // Invalid format
        assert!(!is_data_row(&["21-05-2021".to_string()]));
        // Empty cells
        assert!(!is_data_row(&[]));
        // Non-digit year
        assert!(!is_data_row(&["ABCD-05-22".to_string()]));
    }

    #[test]
    fn test_group_into_rows_empty() {
        let elements: Vec<TextElement> = vec![];
        let rows = group_into_rows(elements, Y_TOLERANCE);
        assert!(rows.is_empty());
    }

    #[test]
    fn test_group_into_rows_single_element() {
        let elements = vec![TextElement {
            text: "Hello".to_string(),
            x: 10.0,
            y: 100.0,
        }];
        let rows = group_into_rows(elements, Y_TOLERANCE);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].cells.len(), 1);
        assert_eq!(rows[0].cells[0].1, "Hello");
    }

    #[test]
    fn test_group_into_rows_same_line() {
        let elements = vec![
            TextElement {
                text: "First".to_string(),
                x: 10.0,
                y: 99.0,
            },
            TextElement {
                text: "Second".to_string(),
                x: 50.0,
                y: 100.0, // Both 99/3=33 and 100/3=33.3 round to 33
            },
        ];
        let rows = group_into_rows(elements, Y_TOLERANCE);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].cells.len(), 2);
    }

    #[test]
    fn test_group_into_rows_different_lines() {
        let elements = vec![
            TextElement {
                text: "Line1".to_string(),
                x: 10.0,
                y: 100.0,
            },
            TextElement {
                text: "Line2".to_string(),
                x: 10.0,
                y: 80.0, // Different line
            },
        ];
        let rows = group_into_rows(elements, Y_TOLERANCE);
        assert_eq!(rows.len(), 2);
        // Rows should be sorted by Y descending
        assert_eq!(rows[0].cells[0].1, "Line1");
        assert_eq!(rows[1].cells[0].1, "Line2");
    }

    #[test]
    fn test_assign_cells_to_columns() {
        let boundaries = vec![0.0, 50.0, 100.0, f32::MAX];
        let row = TableRow {
            y: 100.0,
            cells: vec![
                (10.0, "Col1".to_string()),
                (60.0, "Col2".to_string()),
                (110.0, "Col3".to_string()),
            ],
        };
        let columns = assign_cells_to_columns(&row, &boundaries);
        assert_eq!(columns.len(), 3);
        assert_eq!(columns[0], "Col1");
        assert_eq!(columns[1], "Col2");
        assert_eq!(columns[2], "Col3");
    }

    #[test]
    fn test_assign_cells_to_columns_multiple_in_same_column() {
        let boundaries = vec![0.0, 100.0, f32::MAX];
        let row = TableRow {
            y: 100.0,
            cells: vec![(10.0, "First".to_string()), (30.0, "Second".to_string())],
        };
        let columns = assign_cells_to_columns(&row, &boundaries);
        assert_eq!(columns.len(), 2);
        assert_eq!(columns[0], "First Second");
    }

    #[test]
    fn test_assign_cells_to_columns_empty_columns() {
        let boundaries = vec![0.0, 50.0, 100.0, f32::MAX];
        let row = TableRow {
            y: 100.0,
            cells: vec![(60.0, "OnlyCol2".to_string())],
        };
        let columns = assign_cells_to_columns(&row, &boundaries);
        assert_eq!(columns.len(), 3);
        assert_eq!(columns[0], "");
        assert_eq!(columns[1], "OnlyCol2");
        assert_eq!(columns[2], "");
    }

    #[test]
    fn test_merge_continuation_row() {
        let mut record = vec![
            "2021-01-01".to_string(),
            "".to_string(),
            "ABCD1234".to_string(),
        ];
        let continuation = vec!["".to_string(), "continued".to_string(), "more".to_string()];
        merge_continuation_row(&mut record, &continuation);
        assert_eq!(record[0], "2021-01-01");
        assert_eq!(record[1], "continued");
        assert_eq!(record[2], "ABCD1234 more");
    }

    #[test]
    fn test_merge_continuation_row_empty_continuation() {
        let mut record = vec!["Original".to_string()];
        let continuation = vec!["".to_string()];
        merge_continuation_row(&mut record, &continuation);
        assert_eq!(record[0], "Original");
    }
}
