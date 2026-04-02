use bytes::{Buf, BufMut, BytesMut};
use serde::{Deserialize, Serialize};
use tokio_util::codec::{Decoder, Encoder};

use crate::error::Error;

const DEFAULT_MAX_FRAME_SIZE: usize = 16 * 1024 * 1024; // 16 MiB
const LENGTH_PREFIX_SIZE: usize = 4;

pub struct BincodeCodec<T> {
    max_frame_size: usize,
    _marker: std::marker::PhantomData<T>,
}

impl<T> BincodeCodec<T> {
    pub fn new() -> Self {
        Self {
            max_frame_size: DEFAULT_MAX_FRAME_SIZE,
            _marker: std::marker::PhantomData,
        }
    }

    pub fn with_max_frame_size(max_frame_size: usize) -> Self {
        Self {
            max_frame_size,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<T> Default for BincodeCodec<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: for<'de> Deserialize<'de>> Decoder for BincodeCodec<T> {
    type Item = T;
    type Error = Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.len() < LENGTH_PREFIX_SIZE {
            return Ok(None);
        }

        let length = u32::from_be_bytes([src[0], src[1], src[2], src[3]]) as usize;

        if length > self.max_frame_size {
            return Err(Error::FrameTooLarge {
                size: length,
                max: self.max_frame_size,
            });
        }

        let total = LENGTH_PREFIX_SIZE + length;
        if src.len() < total {
            src.reserve(total - src.len());
            return Ok(None);
        }

        src.advance(LENGTH_PREFIX_SIZE);
        let payload = src.split_to(length);
        let item = bincode::deserialize(&payload)?;
        Ok(Some(item))
    }
}

impl<T: Serialize> Encoder<T> for BincodeCodec<T> {
    type Error = Error;

    fn encode(&mut self, item: T, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let payload = bincode::serialize(&item)?;

        if payload.len() > self.max_frame_size {
            return Err(Error::FrameTooLarge {
                size: payload.len(),
                max: self.max_frame_size,
            });
        }

        dst.reserve(LENGTH_PREFIX_SIZE + payload.len());
        dst.put_u32(payload.len() as u32);
        dst.put_slice(&payload);
        Ok(())
    }
}
