use std::fmt;

use symphonia_core::audio::{
    AsGenericAudioBufferRef, AudioBuffer, AudioMut, AudioSpec, GenericAudioBufferRef, layouts,
};
use symphonia_core::codecs::CodecInfo;
use symphonia_core::codecs::audio::well_known::CODEC_ID_OPUS;
use symphonia_core::codecs::audio::{
    AudioCodecParameters, AudioDecoder, AudioDecoderOptions, FinalizeResult,
};
use symphonia_core::codecs::registry::{RegisterableAudioDecoder, SupportedAudioCodec};
use symphonia_core::errors::{Result, unsupported_error};
use symphonia_core::io::{BufReader, ReadBytes};
use symphonia_core::support_audio_codec;

use crate::decoder::Decoder;

mod decoder;

const MAX_SAMPLE_RATE: usize = 48000;
const DEFAULT_SAMPLE_RATE: usize = 48000;
const DEFAULT_SAMPLES_PER_CHANNEL: usize = DEFAULT_SAMPLE_RATE * 20 / 1000;
const MAX_SAMPLES_PER_CHANNEL: usize = MAX_SAMPLE_RATE * 120 / 1000;

pub struct OpusDecoder {
    params: AudioCodecParameters,
    decoder: Decoder,
    buf: AudioBuffer<f32>,
    pcm_i16: [i16; MAX_SAMPLES_PER_CHANNEL * 2],
    pcm_f32: [f32; MAX_SAMPLES_PER_CHANNEL * 2],
    samples_per_channel: usize,
    sample_rate: u32,
    num_channels: usize,
    pre_skip: usize,
}

impl fmt::Debug for OpusDecoder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OpusDecoder")
            .field("params", &self.params)
            .field("decoder", &self.decoder)
            .field("buf", &"<buf>")
            .field("pcm_i16", &"<pcm>")
            .field("samples_per_channel", &self.samples_per_channel)
            .field("sample_rate", &self.sample_rate)
            .field("num_channels", &self.num_channels)
            .finish()
    }
}

struct OpusHead {
    channels: usize,
    pre_skip: usize,
    sample_rate: u32,
}

fn parse_opus_head(buf: &[u8]) -> Option<OpusHead> {
    let mut reader = BufReader::new(buf);

    let mut header = [0; 8];
    reader.read_buf_exact(&mut header).ok()?;
    if &header != b"OpusHead" {
        return None;
    }

    let _version = reader.read_byte().ok()?;
    let channels = reader.read_byte().ok()? as usize;
    let pre_skip = reader.read_u16().ok()? as usize;

    Some(OpusHead {
        channels,
        pre_skip,
        sample_rate: 48000,
    })
}

impl OpusDecoder {
    fn try_new(params: &AudioCodecParameters, _opts: &AudioDecoderOptions) -> Result<Self> {
        let opus_head = params
            .extra_data
            .as_ref()
            .and_then(|d| parse_opus_head(d));

        let num_channels = params
            .channels
            .as_ref()
            .map(|c| c.count())
            .or_else(|| opus_head.as_ref().map(|h| h.channels))
            .unwrap_or(2);

        if !(1..=2).contains(&num_channels) {
            return unsupported_error("opus: unsupported number of channels");
        }

        let sample_rate = params
            .sample_rate
            .or_else(|| opus_head.as_ref().map(|h| h.sample_rate))
            .unwrap_or(DEFAULT_SAMPLE_RATE as u32);

        let pre_skip = opus_head.map(|h| h.pre_skip).unwrap_or(0);

        let mut owned_params = params.to_owned();
        owned_params.sample_rate = Some(sample_rate);
        owned_params.channels = Some(map_to_channels(num_channels).unwrap_or_else(|| {
            symphonia_core::audio::Channels::Discrete(num_channels as u16)
        }));

        Ok(Self {
            params: owned_params,
            decoder: Decoder::new(sample_rate, num_channels as u32)?,
            buf: audio_buffer(sample_rate, DEFAULT_SAMPLES_PER_CHANNEL, num_channels),
            pcm_i16: [0; MAX_SAMPLES_PER_CHANNEL * 2],
            pcm_f32: [0.0; MAX_SAMPLES_PER_CHANNEL * 2],
            samples_per_channel: DEFAULT_SAMPLES_PER_CHANNEL,
            sample_rate,
            num_channels,
            pre_skip,
        })
    }
}

impl AudioDecoder for OpusDecoder {
    fn codec_info(&self) -> &CodecInfo {
        &Self::supported_codecs()
            .first()
            .expect("missing codecs")
            .info
    }

    fn reset(&mut self) {
        self.decoder.reset()
    }

    fn codec_params(&self) -> &AudioCodecParameters {
        &self.params
    }

    fn decode_ref(
        &mut self,
        packet: &symphonia_core::packet::PacketRef<'_>,
    ) -> Result<GenericAudioBufferRef<'_>> {
        let samples_per_channel = self.decoder.decode(packet.data, &mut self.pcm_i16)?;

        if samples_per_channel != self.samples_per_channel {
            self.buf = audio_buffer(self.sample_rate, samples_per_channel, self.num_channels);
            self.samples_per_channel = samples_per_channel;
        }

        let samples = samples_per_channel * self.num_channels;
        let pcm_i16 = &self.pcm_i16[..samples];
        convert_i16_to_f32(pcm_i16, &mut self.pcm_f32[..samples]);

        self.buf.clear();
        self.buf.render_uninit(None);
        let slice: &[f32] = &self.pcm_f32[..samples];
        self.buf.copy_from_slice_interleaved(&slice);

        self.buf.trim(
            packet.trim_start.get() as usize
                + (self.pre_skip * self.sample_rate as usize) / DEFAULT_SAMPLE_RATE,
            packet.trim_end.get() as usize,
        );
        self.pre_skip = 0;
        Ok(self.buf.as_generic_audio_buffer_ref())
    }

    fn finalize(&mut self) -> FinalizeResult {
        FinalizeResult::default()
    }

    fn last_decoded(&self) -> GenericAudioBufferRef<'_> {
        self.buf.as_generic_audio_buffer_ref()
    }
}

impl RegisterableAudioDecoder for OpusDecoder {
    fn try_registry_new(
        params: &AudioCodecParameters,
        opts: &AudioDecoderOptions,
    ) -> Result<Box<dyn AudioDecoder>>
    where
        Self: Sized,
    {
        Ok(Box::new(OpusDecoder::try_new(params, opts)?))
    }

    fn supported_codecs() -> &'static [SupportedAudioCodec] {
        &[support_audio_codec!(CODEC_ID_OPUS, "opus", "Opus")]
    }
}

pub(crate) fn map_to_channels(num_channels: usize) -> Option<symphonia_core::audio::Channels> {
    let channels = match num_channels {
        1 => layouts::CHANNEL_LAYOUT_MONO,
        2 => layouts::CHANNEL_LAYOUT_STEREO,
        _ => return None,
    };

    Some(channels)
}

fn convert_i16_to_f32(src: &[i16], dst: &mut [f32]) {
    for (s, d) in src.iter().zip(dst.iter_mut()) {
        *d = *s as f32 * (1.0 / 32768.0);
    }
}

fn audio_buffer(
    sample_rate: u32,
    samples_per_channel: usize,
    num_channels: usize,
) -> AudioBuffer<f32> {
    let channels = map_to_channels(num_channels).expect("invalid channels");
    let spec = AudioSpec::new(sample_rate, channels);
    AudioBuffer::new(spec, samples_per_channel)
}
