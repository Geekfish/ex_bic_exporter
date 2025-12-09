defmodule BicExporter.Native do
  @moduledoc false

  use Rustler,
    otp_app: :ex_bic_exporter,
    crate: :bic_exporter

  # NIF stubs - these are replaced when the NIF is loaded

  def headers, do: :erlang.nif_error(:nif_not_loaded)
  def extract_table_from_path(_source), do: :erlang.nif_error(:nif_not_loaded)
  def extract_table_from_binary(_data), do: :erlang.nif_error(:nif_not_loaded)
  def convert_to_csv(_source, _destination), do: :erlang.nif_error(:nif_not_loaded)
end
