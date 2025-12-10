defmodule RustlerPanicSafetyTest do
  @moduledoc """
  Tests to verify that Rustler catches Rust panics and converts them to
  Erlang errors instead of crashing the BEAM.

  This proves that even if Rust code panics (via unwrap(), expect(), panic!(), etc.),
  the BEAM VM will not crash - Rustler catches the panic at the FFI boundary.

  The `panic_test` Cargo feature is automatically enabled in test builds.
  """
  use ExUnit.Case

  describe "Rustler panic safety" do
    test "panic!() is caught and converted to an Erlang error (BEAM does not crash)" do
      assert_raise ErlangError, fn ->
        BicExporter.Native.deliberate_panic()
      end

      # If we get here, the BEAM is still running! The panic was caught.
      # Verify the VM is still functional by calling another NIF
      assert is_list(BicExporter.headers())
    end

    test "unwrap() panic is caught and converted to an Erlang error (BEAM does not crash)" do
      assert_raise ErlangError, fn ->
        BicExporter.Native.deliberate_unwrap_panic()
      end

      assert is_list(BicExporter.headers())
    end
  end
end
