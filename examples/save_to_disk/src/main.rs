use bytes::{BufMut, BytesMut};
use futures::StreamExt;
use livekit::prelude::*;
use livekit::webrtc::audio_stream::native::NativeAudioStream;
use std::env;
use tokio::fs::File;
use tokio::io::{AsyncWriteExt, BufWriter};

const WAV_HEADER_SIZE: usize = 44;
const FILE_PATH: &str = "record.wav";

#[derive(Debug, Clone, Copy)]
pub struct WavHeader {
    pub sample_rate: u32,
    pub bit_depth: u16,
    pub num_channels: u16,
}

pub struct WavWriter {
    header: WavHeader,
    data: BytesMut,
    writer: BufWriter<File>,
}

impl WavWriter {
    pub async fn create<P: AsRef<std::path::Path>>(
        path: P,
        header: WavHeader,
    ) -> Result<WavWriter, std::io::Error> {
        let file = File::create(path).await?;
        let writer = BufWriter::new(file);

        let mut wav_writer = WavWriter {
            header,
            data: BytesMut::new(),
            writer,
        };

        wav_writer.write_header()?;
        Ok(wav_writer)
    }

    fn write_header(&mut self) -> Result<(), std::io::Error> {
        let byte_rate = (self.header.sample_rate
            * self.header.bit_depth as u32
            * self.header.num_channels as u32);

        let block_align = byte_rate / self.header.sample_rate as u32;

        self.data.put_slice(b"RIFF");
        self.data.put_u32_le(0); // Placeholder for file size
        self.data.put_slice(b"WAVE");
        self.data.put_slice(b"fmt ");
        self.data.put_u32_le(16); // Subchunk1Size (16 for PCM)
        self.data.put_u16_le(1); // AudioFormat (1 for PCM)
        self.data.put_u16_le(self.header.num_channels);
        self.data.put_u32_le(self.header.sample_rate);
        self.data.put_u32_le(byte_rate);
        self.data.put_u16_le(32);
        self.data.put_u16_le(self.header.bit_depth);
        self.data.put_slice(b"data");
        self.data.put_u32_le(0); // Placeholder for data size

        assert_eq!(self.data.len(), WAV_HEADER_SIZE);

        Ok(())
    }

    pub async fn write_sample(&mut self, sample: i16) -> Result<(), std::io::Error> {
        self.data.put_i16_le(sample);
        Ok(())
    }

    pub async fn finalize(mut self) -> Result<(), std::io::Error> {
        let data_size = self.data.len() as u32 - WAV_HEADER_SIZE as u32;
        let file_size = data_size + WAV_HEADER_SIZE as u32 - 8;
        self.data.as_mut()[4..8].copy_from_slice(&file_size.to_le_bytes());
        self.data.as_mut()[40..44].copy_from_slice(&data_size.to_le_bytes());

        self.writer.write_all(&self.data).await?;
        self.writer.flush().await?;
        Ok(())
    }
}

#[tokio::main]
async fn main() {
    let url = env::var("LIVEKIT_URL").expect("LIVEKIT_URL is not set");
    let token = env::var("LIVEKIT_TOKEN").expect("LIVEKIT_TOKEN is not set");

    let (room, mut rx) = Room::connect(&url, &token).await.unwrap();
    let session = room.session();
    println!("Connected to room: {} - {}", session.name(), session.sid());

    while let Some(msg) = rx.recv().await {
        match msg {
            RoomEvent::TrackSubscribed {
                track,
                publication: _,
                participant: _,
            } => {
                if let RemoteTrack::Audio(audio_track) = track {
                    record_track(audio_track).await.unwrap();
                    break;
                }
            }
            _ => {}
        }
    }

    println!("Done");
}

async fn record_track(audio_track: RemoteAudioTrack) -> Result<(), std::io::Error> {
    println!("Recording track {:?}", audio_track.sid());
    let rtc_track = audio_track.rtc_track();

    // TODO(theomonnom): Remove hardcoded values
    let header = WavHeader {
        sample_rate: 48000,
        bit_depth: 16,
        num_channels: 1,
    };

    let mut wav_writer = WavWriter::create(FILE_PATH, header).await?;
    let mut audio_stream = NativeAudioStream::new(rtc_track);

    let max_record = 5 * header.sample_rate * header.num_channels as u32;
    let mut sample_count = 0;
    'recv_loop: while let Some(frame) = audio_stream.next().await {
        for sample in frame.data {
            wav_writer.write_sample(sample).await.unwrap();
            sample_count += 1;

            if sample_count >= max_record {
                break 'recv_loop;
            }
        }
    }

    wav_writer.finalize().await?;
    Ok(())
}
