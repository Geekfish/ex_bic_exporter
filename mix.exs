defmodule BicExporter.MixProject do
  use Mix.Project

  @version "0.1.2"
  @source_url "https://github.com/Geekfish/ex_bic_exporter"

  def project do
    [
      app: :ex_bic_exporter,
      description: "A library to extract data from ISO BIC directory PDF files",
      version: @version,
      elixir: "~> 1.19",
      start_permanent: Mix.env() == :prod,
      deps: deps(),
      docs: docs(),
      url: @source_url,
      package: package()
    ]
  end

  def application do
    [
      extra_applications: [:logger]
    ]
  end

  defp docs do
    [
      main: "BicExporter",
      extras: ["README.md"]
    ]
  end

  defp deps do
    [
      {:rustler, ">= 0.0.0", optional: true},
      {:rustler_precompiled, "~> 0.8"},
      {:credo, "~> 1.7", only: [:dev, :test], runtime: false},
      {:ex_doc, "~> 0.30", only: [:dev, :test], runtime: false}
    ]
  end

  defp package do
    [
      name: "ex_bic_exporter",
      licenses: ["MIT"],
      links: %{
        GitHub: @source_url
      },
      files: [
        "lib",
        "native/bic_exporter/.cargo",
        "native/bic_exporter/src",
        "native/bic_exporter/Cargo.toml",
        "native/bic_exporter/Cargo.lock",
        "native/bic_exporter/Cross.toml",
        "checksum-*.exs",
        "mix.exs",
        "README.md",
        "LICENSE*"
      ]
    ]
  end
end
