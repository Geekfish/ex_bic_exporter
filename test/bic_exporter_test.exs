defmodule BicExporterTest do
  use ExUnit.Case

  @fixture_path Path.join([__DIR__, "..", "native", "bic_exporter", "tests", "fixtures"])
  @pdf_path Path.join(@fixture_path, "ISOBIC-mini.pdf")
  @expected_csv_path Path.join(@fixture_path, "ISOBIC-mini-expected.csv")
  @expected_record_count 86

  describe "headers/0" do
    test "returns 10 column headers" do
      assert BicExporter.headers() == [
               "Record creation date",
               "Last Update date",
               "BIC",
               "Brch Code",
               "Full legal name",
               "Registered address",
               "Operational address",
               "Branch description",
               "Branch address",
               "Instit. Type"
             ]
    end
  end

  describe "extract_table_from_path/1" do
    test "extracts records from PDF file" do
      assert {:ok, records} = BicExporter.extract_table_from_path(@pdf_path)

      assert length(records) == @expected_record_count

      # Each record should have 10 fields
      Enum.each(records, fn record ->
        assert length(record) == 10
      end)
    end

    test "returns error for non-existent file" do
      assert {:error, "Failed to open PDF file"} =
               BicExporter.extract_table_from_path("/non/existent/file.pdf")
    end

    test "extracts expected BIC codes" do
      {:ok, [[_, _, first_bic | _] | _]} = BicExporter.extract_table_from_path(@pdf_path)

      assert first_bic == "AAAARSBG"
    end

    test "records have valid date format in first column" do
      {:ok, records} = BicExporter.extract_table_from_path(@pdf_path)

      Enum.each(records, fn [creation_date, last_update_date | _rest] ->
        assert %Date{} = Date.from_iso8601!(creation_date)
        assert %Date{} = Date.from_iso8601!(last_update_date)
      end)
    end
  end

  describe "extract_table_from_binary/1" do
    test "extracts records from PDF binary" do
      pdf_data = File.read!(@pdf_path)

      assert {:ok, records} = BicExporter.extract_table_from_binary(pdf_data)
      assert length(records) == @expected_record_count
    end

    test "returns same results as extract_table_from_path" do
      {:ok, from_path} = BicExporter.extract_table_from_path(@pdf_path)
      {:ok, from_binary} = BicExporter.extract_table_from_binary(File.read!(@pdf_path))

      assert from_path == from_binary
    end

    test "returns error for invalid PDF data" do
      assert {:error, "Failed to load PDF from bytes"} =
               BicExporter.extract_table_from_binary("not a pdf")
    end
  end

  describe "convert_to_csv/2" do
    @tag :tmp_dir
    test "creates CSV file matching expected output", %{tmp_dir: tmp_dir} do
      output_path = Path.join(tmp_dir, "output.csv")

      assert {:ok, @expected_record_count} = BicExporter.convert_to_csv(@pdf_path, output_path)

      expected_content = File.read!(@expected_csv_path)
      actual_content = File.read!(output_path)

      assert actual_content == expected_content
    end

    test "returns error for invalid source path" do
      assert {:error, "Failed to open PDF file"} =
               BicExporter.convert_to_csv("/non/existent.pdf", "/tmp/out.csv")
    end
  end
end
