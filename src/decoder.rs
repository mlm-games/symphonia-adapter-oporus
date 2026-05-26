use log::warn;
use symphonia_core::errors::Result;

#[derive(Debug)]
pub(crate) struct Decoder {
    inner: mousiki::Decoder,
}

impl Decoder {
    pub(crate) fn new(sample_rate: u32, channels: u32) -> Result<Self> {
        let ch = match channels {
            1 => mousiki::Channels::Mono,
            2 => mousiki::Channels::Stereo,
            _ => {
                return Err(symphonia_core::errors::Error::DecodeError(
                    "opus: unsupported number of channels",
                ));
            }
        };

        let inner = mousiki::Decoder::new(sample_rate, ch).map_err(|e| {
            log::error!("mousiki decoder creation failed: {e:?}");
            symphonia_core::errors::Error::DecodeError("opus: error creating decoder")
        })?;

        Ok(Self { inner })
    }

    pub(crate) fn decode(&mut self, input: &[u8], output: &mut [i16]) -> Result<usize> {
        if input.is_empty() {
            self.inner.conceal(output).map_err(|e| {
                warn!("mousiki conceal failed: {e:?}");
                symphonia_core::errors::Error::DecodeError("opus: decode failed")
            })
        } else {
            self.inner.decode(input, output, false).map_err(|e| {
                warn!("mousiki decode failed: {e:?}");
                symphonia_core::errors::Error::DecodeError("opus: decode failed")
            })
        }
    }

    pub(crate) fn reset(&mut self) {
        if let Err(e) = self.inner.reset_state() {
            warn!("mousiki decoder reset failed: {e:?}");
        }
    }
}
