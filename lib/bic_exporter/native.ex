defmodule BicExporter.Native do
  @moduledoc false
  version = Mix.Project.config()[:version]

  use Rustler,
    otp_app: :ex_bic_exporter,
    crate: :bic_exporter,
    base_url: "https://github.com/Geekfish/ex_bic_exporter/releases/download/v#{version}",
    force_build: System.get_env("RUSTLER_PRECOMPILATION_EXAMPLE_BUILD") in ["1", "true"],
    version: version

  # NIF stubs - these are replaced when the NIF is loaded

  def headers, do: :erlang.nif_error(:nif_not_loaded)
  def extract_table_from_path(_source), do: :erlang.nif_error(:nif_not_loaded)
  def extract_table_from_binary(_data), do: :erlang.nif_error(:nif_not_loaded)
  def convert_to_csv(_source, _destination), do: :erlang.nif_error(:nif_not_loaded)
end
