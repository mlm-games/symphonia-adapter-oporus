use symphonia_core::codecs::audio::well_known::CODEC_ID_OPUS;
use symphonia_core::codecs::audio::{AudioCodecParameters, AudioDecoderOptions};
use symphonia_core::codecs::registry::RegisterableAudioDecoder;
use symphonia_core::units::{Duration, Timestamp};

use symphonia_adapter_mousiki::OpusDecoder;

fn test_params() -> AudioCodecParameters {
    let mut params = AudioCodecParameters::new();
    params.codec = CODEC_ID_OPUS;
    params.sample_rate = Some(48000);
    params.channels = Some(symphonia_core::audio::layouts::CHANNEL_LAYOUT_MONO);
    params
}

#[test]
fn test_decoder_creation() {
    let decoder = OpusDecoder::try_registry_new(&test_params(), &AudioDecoderOptions::default());
    assert!(decoder.is_ok(), "decoder creation should succeed");
}

#[test]
fn test_supported_codecs() {
    let codecs = OpusDecoder::supported_codecs();
    assert_eq!(codecs.len(), 1);
    assert_eq!(codecs[0].id, CODEC_ID_OPUS);
    assert_eq!(codecs[0].info.short_name, "opus");
}

#[test]
fn test_codec_info() {
    let decoder = OpusDecoder::try_registry_new(&test_params(), &AudioDecoderOptions::default())
        .expect("decoder creation");
    let info = decoder.codec_info();
    assert_eq!(info.short_name, "opus");
    assert_eq!(info.long_name, "Opus");
}

#[test]
fn test_decode_fails_with_garbage_packet() {
    let mut decoder =
        OpusDecoder::try_registry_new(&test_params(), &AudioDecoderOptions::default())
            .expect("decoder creation");

    let packet = symphonia_core::packet::PacketRef::new(
        0,
        Timestamp::default(),
        Duration::default(),
        &[0xFF, 0xFF, 0xFF],
    );

    let result = decoder.decode_ref(&packet);
    assert!(result.is_err(), "garbage packet should fail decode");
}

#[test]
fn test_stereo_params() {
    let mut params = AudioCodecParameters::new();
    params.codec = CODEC_ID_OPUS;
    params.sample_rate = Some(48000);
    params.channels = Some(symphonia_core::audio::layouts::CHANNEL_LAYOUT_STEREO);

    let decoder = OpusDecoder::try_registry_new(&params, &AudioDecoderOptions::default());
    assert!(decoder.is_ok(), "stereo decoder creation should succeed");
}

#[test]
fn test_creation_without_channels_or_sample_rate_falls_back() {
    let mut params = AudioCodecParameters::new();
    params.codec = CODEC_ID_OPUS;
    // omittin channels and sample_rate should fall back to defaults

    let decoder = OpusDecoder::try_registry_new(&params, &AudioDecoderOptions::default());
    assert!(decoder.is_ok(), "decoder should fall back to stereo/48kHz");

    let decoder = decoder.unwrap();
    let codec_params = decoder.codec_params();
    assert_eq!(codec_params.sample_rate, Some(48000));
    assert_eq!(codec_params.channels.as_ref().map(|c| c.count()), Some(2));
}

#[test]
fn test_reset() {
    let mut decoder =
        OpusDecoder::try_registry_new(&test_params(), &AudioDecoderOptions::default())
            .expect("decoder creation");
    decoder.reset();
    // After reset, the decoder should still be usable
    let params = decoder.codec_params();
    assert!(params.sample_rate == Some(48000));
}
