defmodule BicExporter.Native do
  @moduledoc false
  version = Mix.Project.config()[:version]

  # Always force build in test/dev to test current Rust code, not past releases.
  # Precompiled NIFs are only for production use by library consumers.
  force_build? =
    Mix.env() in [:test, :dev] or
      System.get_env("RUSTLER_PRECOMPILATION_FORCE_BUILD") in ["1", "true"]

  # Enable panic_test feature in test builds to verify Rustler's panic safety
  features = if Mix.env() == :test, do: ["panic_test"], else: []

  use RustlerPrecompiled,
    otp_app: :ex_bic_exporter,
    crate: :bic_exporter,
    base_url: "https://github.com/Geekfish/ex_bic_exporter/releases/download/v#{version}",
    force_build: force_build?,
    features: features,
    version: version,
    targets: [
      "aarch64-apple-darwin",
      "aarch64-unknown-linux-gnu",
      "aarch64-unknown-linux-musl",
      "x86_64-apple-darwin",
      "x86_64-unknown-linux-gnu",
      "x86_64-unknown-linux-musl"
    ],
    nif_versions: ["2.17", "2.16"]

  # NIF stubs - these are replaced when the NIF is loaded

  def headers, do: :erlang.nif_error(:nif_not_loaded)
  def extract_table_from_path(_source), do: :erlang.nif_error(:nif_not_loaded)
  def extract_table_from_binary(_data), do: :erlang.nif_error(:nif_not_loaded)
  def convert_to_csv(_source, _destination), do: :erlang.nif_error(:nif_not_loaded)

  # Test-only NIFs to verify Rustler catches panics (only available with panic_test feature)
  if Mix.env() == :test do
    def deliberate_panic, do: :erlang.nif_error(:nif_not_loaded)
    def deliberate_unwrap_panic, do: :erlang.nif_error(:nif_not_loaded)
  end
end
