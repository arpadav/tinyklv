EXAMPLES=(
  01_hello_world
  02_custom_key_types
  03_strings_and_types
  04_optional_fields
  05_basic_roundtrip
  06_custom_encoder_decoder
  07_sentinel_seeking
  08_nested_packets
  09_ber_keyed
  10_defaults_and_init
  11_repeated_extraction
  12_enum_dispatch_stream
  13_variable_length_fields
  14_break_condition_custom
  15_tokio_stream_e2e
)

for example in "${EXAMPLES[@]}"; do
  cargo run --example $example
done
