defmodule BicExporterTest do
  use ExUnit.Case

  @fixture_path Path.join([__DIR__, "..", "native", "bic_exporter", "tests", "fixtures"])
  @pdf_path Path.join(@fixture_path, "ISOBIC-mini.pdf")
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

  describe "extract_table_from_binary/1" do
    test "extracts records from PDF binary " do
      pdf_data = File.read!(@pdf_path)

      {:ok, result} = BicExporter.extract_table_from_binary(pdf_data)
      assert length(result) == @expected_record_count

      assert [
               [
                 "1997-03-01",
                 "2024-06-06",
                 "AAAARSBG",
                 "XXX",
                 "YETTEL BANK AD",
                 "88 OMLADINSKIH BRIGADA BEOGRAD 11070 SERBIA",
                 "88 OMLADINSKIH BRIGADA BEOGRAD 11070 BEOGRAD SERBIA",
                 "",
                 "",
                 "FIIN"
               ],
               [
                 "1994-03-07",
                 "2024-07-05",
                 "AAACKWKW",
                 "XXX",
                 "AL MUZAINI EXCHANGE CO. KSCC",
                 "BLOCK 4, SAUD BIN ABDULAZIZ ST. BUILDING 9 KUWAIT, AL MUBARAKIYA 13022 KUWAIT",
                 "BUILDING 9 BLOCK 4 SAUD BIN ABDULAZIZ ST. KUWAIT 13022 KUWAIT POB 2156 KUWAIT",
                 "",
                 "",
                 "FIIN"
               ],
               [
                 "2006-06-03",
                 "2024-11-05",
                 "AAADFRP1",
                 "XXX",
                 "ABN AMRO INVESTMENT SOLUTIONS S.A.",
                 "119-121 BOULEVARD HAUSSMANN PARIS 75008 FRANCE",
                 "3 AVENUE HOCHE CHEZ NSM CHEZ NSM PARIS 75008 PARIS FRANCE",
                 "",
                 "",
                 "FIIN"
               ],
               [
                 "2014-07-05",
                 "2018-04-14",
                 "AAAJBG21",
                 "XXX",
                 "ARCUS ASSET MANAGEMENT JSC",
                 "BUSINESS CENTER LEGIS 6TH OF SEPTEMBER BLVD. 152 PLOVDIV 4000 BULGARIA",
                 "BUSINESS CENTER LEGIS 6TH OF SEPTEMBER BLVD. 152 PLOVDIV 4000 PLOVDIV BULGARIA",
                 "",
                 "",
                 "FIIN"
               ],
               [
                 "2006-06-03",
                 "2021-05-20",
                 "AAAMFRP1",
                 "XXX",
                 "NEXAM",
                 "14 RUE HALEVY PARIS 75009 FRANCE",
                 "20 RUE LE PELETIER PARIS 75009 PARIS FRANCE",
                 "",
                 "",
                 "FIIN"
               ]
             ] == Enum.take(result, 5)
    end

    test "records have valid date format in first two columns" do
      pdf_data = File.read!(@pdf_path)
      {:ok, records} = BicExporter.extract_table_from_binary(pdf_data)

      Enum.each(records, fn [creation_date, last_update_date | _rest] ->
        assert %Date{} = Date.from_iso8601!(creation_date)
        assert %Date{} = Date.from_iso8601!(last_update_date)
      end)
    end

    test "returns error for invalid PDF data" do
      assert {:error, "Failed to load PDF from bytes"} =
               BicExporter.extract_table_from_binary("not a pdf")
    end
  end
end
