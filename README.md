# BIC Exporter

[![Licence](https://img.shields.io/github/license/Geekfish/ex_bic_exporter.svg)](https://github.com/Geekfish/ex_bic_exporter/blob/main/LICENSE)
[![Elixir CI](https://github.com/Geekfish/ex_bic_exporter/actions/workflows/elixir-ci.yaml/badge.svg)](https://github.com/Geekfish/ex_bic_exporter/actions/workflows/elixir-ci.yaml)
[![Rust CI](https://github.com/Geekfish/ex_bic_exporter/actions/workflows/rust-ci.yaml/badge.svg)](https://github.com/Geekfish/ex_bic_exporter/actions/workflows/rust-ci.yaml)

An Elixir library to extract BIC (Bank Identifier Code) directory data from ISO 9362 PDF files.

The PDF is published by [Swift](https://www.swiftref.com/en/bicsearch).

👉 The latest version should be downloadable from [this url](https://www.swiftref.com/api/download/pdf).

## ⚠️ DISCLAIMER: THIS IS NOT AN API

PDF files are not an official API. This parser might stop working if the PDF format changes even in small ways.

It's a tool to help you explore data. It is not a replacement for official APIs provided by Swift.

## Requirements

- Elixir 1.19 or higher
- Rust 1.70 or higher (for compiling the NIF)


## Useful to know

### Why Rust
The first iteration of this was in Python, which has tons of PDF utils.

However python also took more than 10 minutes to process a real file, and that's after optimizing and switching to multithreading. We'd then need to figure out how to deploy and call the python script.

In contrast, this script takes ~7 seconds on the same machine without any optimization and can be used directly from Elixir.

### Safety
We're using [rustler](https://github.com/rusterlium/rustler) which

> catches rust panics before they unwind into C.

So in theory this lib shouldn't bring the entire BEAM down.

### Choice of PDF Library
This projects uses the [pdf](https://github.com/pdf-rs/pdf) library from [pdf-rs](https://github.com/pdf-rs).

There are a few PDF libraries for rust but most of them either seem to just wrap something else and/or do wall-of-text extraction. E.g. [Extractous](https://github.com/yobix-ai/extractous) wraps Apache Tika, [pdf-extract](https://github.com/jrmuizel/pdf-extract/) is pure rust, but extracts just text.


### Use of AI

⚠️ A lot of this has been written with the help of AI.
It helped figure out all the magic of extracting rows from the PDF.
There is a chance there are simpler ways to do this.

## Installation

Add `bic_exporter` to your list of dependencies in `mix.exs`:

```elixir
def deps do
  [
    {:bic_exporter, "~> 0.1.0"}
  ]
end
```

Then run:

```bash
mix deps.get
mix compile
```

## Usage

### Extract from file path

```elixir
{:ok, records} = BicExporter.extract_table_from_path("/path/to/ISOBIC.pdf")
```

### Extract from binary data

Useful when the PDF is already in memory (e.g., downloaded from a URL):

```elixir
pdf_data = File.read!("/path/to/ISOBIC.pdf")
{:ok, records} = BicExporter.extract_table_from_binary(pdf_data)
```

### Convert directly to CSV

```elixir
{:ok, count} = BicExporter.convert_to_csv("/path/to/ISOBIC.pdf", "/path/to/output.csv")
IO.puts("Extracted #{count} records")
```

### Get column headers

```elixir
BicExporter.headers()
# => ["Record creation date", "Last Update date", "BIC", "Brch Code",
#     "Full legal name", "Registered address", "Operational address",
#     "Branch description", "Branch address", "Instit. Type"]
```

### Record format

Each record is a list of 10 strings corresponding to the CSV columns:

```elixir
[
  "1997-03-01",           # Record creation date
  "2024-06-06",           # Last Update date
  "ABORCA82",             # BIC
  "XXX",                  # Branch Code
  "ABOR BANK",            # Full legal name
  "123 Main Street",      # Registered address
  "456 Business Ave",     # Operational address
  "Main office",          # Branch description (optional)
  "",                     # Branch address (optional)
  "BANK"                  # Institution Type
]
```

## Development

```bash
# From root /
# Fetch dependencies
mix deps.get

# Compile (includes Rust NIF)
mix compile

# Run tests
mix test

# Format Elixir code
mix format

# From native/bic_exporter
# Format Rust code
cargo fmt

# Run Rust linter
cargo clippy --all-targets --all-features

# Run Rust tests
cargo test
```


## Release

For maintainers:

- Bump the version in `mix.exs`
- Create a new tag, e.g. `git tag v0.2.0`
- Push the tag to Github `git push origin main --tags`
- ⚠️ _Wait_ for the Github actions workflows to be successfully completed.
- Run `mix rustler_precompiled.download BicExporter.Native --all --print`.
  You may need to set `RUSTLER_PRECOMPILATION_FORCE_BUILD=true` when running this.
- Publish the package on hex: `mix hex.publish`
- 🍰
