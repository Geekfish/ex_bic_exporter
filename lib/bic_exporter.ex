defmodule BicExporter do
  @moduledoc """
  Extracts BIC (Bank Identifier Code) directory data from ISO 9362 PDF files.

  This module provides functions to parse the official ISO BIC directory PDF
  and extract structured data from it.

  ## Examples

      # Extract from a file path
      {:ok, records} = BicExporter.extract_table_from_path("/path/to/ISOBIC.pdf")

      # Extract from binary data
      pdf_data = File.read!("/path/to/ISOBIC.pdf")
      {:ok, records} = BicExporter.extract_table_from_binary(pdf_data)

      # Convert directly to CSV
      {:ok, count} = BicExporter.convert_to_csv("/path/to/ISOBIC.pdf", "/path/to/output.csv")

  Each record is a list of 10 strings corresponding to the CSV columns.
  Use `headers/0` to get the column names.
  """

  @doc """
  Returns the CSV column headers.

  ## Example

      iex> BicExporter.headers()
      ["Record creation date", "Last Update date", "BIC", "Brch Code",
       "Full legal name", "Registered address", "Operational address",
       "Branch description", "Branch address", "Instit. Type"]
  """
  defdelegate headers(), to: BicExporter.Native

  @doc """
  Extracts BIC records from a PDF file path.

  Returns `{:ok, records}` on success or `{:error, reason}` on failure.
  Each record is a list of 10 strings corresponding to the CSV columns.

  ## Example

      {:ok, records} = BicExporter.extract_table_from_path("/path/to/ISOBIC.pdf")
      Enum.each(records, fn [_created, _updated, bic | _rest] ->
        IO.puts("BIC: \#{bic}")
      end)
  """
  defdelegate extract_table_from_path(source), to: BicExporter.Native

  @doc """
  Extracts BIC records from PDF binary data.

  This is useful when the PDF is already loaded in memory (e.g., downloaded
  from a URL or read from a database).

  Returns `{:ok, records}` on success or `{:error, reason}` on failure.

  ## Example

      pdf_data = File.read!("/path/to/ISOBIC.pdf")
      {:ok, records} = BicExporter.extract_table_from_binary(pdf_data)
  """
  defdelegate extract_table_from_binary(data), to: BicExporter.Native

  @doc """
  Converts a BIC directory PDF to CSV format.

  Returns `{:ok, record_count}` on success or `{:error, reason}` on failure.

  ## Example

      {:ok, count} = BicExporter.convert_to_csv("/path/to/ISOBIC.pdf", "/path/to/output.csv")
      IO.puts("Extracted \#{count} records")
  """
  defdelegate convert_to_csv(source, destination), to: BicExporter.Native
end
