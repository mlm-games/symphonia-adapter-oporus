# symphonia-adapter-oporus

Adapter to use [oporus](https://crates.io/crates/oporus) (pure-Rust Opus codec) with [Symphonia](https://github.com/pdeljanov/Symphonia).

See [symphonia-adapters](https://github.com/aschey/symphonia-adapters) for the original adapter pattern and usage examples (was the reference for this repo).

```rust
use symphonia::core::codecs::registry::CodecRegistry;
use symphonia_adapter_oporus::OpusDecoder;

let mut registry = CodecRegistry::new();
registry.register_audio_decoder::<OpusDecoder>();
```
