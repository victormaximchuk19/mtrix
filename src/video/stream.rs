use std::{sync::{Arc, Mutex}, thread, time::Duration};
use nokhwa::{pixel_format::RgbFormat, utils::{CameraIndex, RequestedFormat, RequestedFormatType}, CallbackCamera};
use crossbeam::channel::{self, Sender, Receiver};

use super::ascii_frame;
use crate::masp::{sender::MaspSender, message::PacketType};

const FPS: u64 = 25;
const FRAME_RATE: u64 = 1000 / FPS;

pub async fn run(mut sender: MaspSender) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
  let (frame_sender, frame_receiver) = channel::unbounded();
  let (ascii_frame_sender, ascii_frame_receiver): (Sender<(String, u128)>, Receiver<(String, u128)>) = channel::unbounded();

  let mut camera = CallbackCamera::new(
    CameraIndex::Index(0),
    RequestedFormat::new::<RgbFormat>(RequestedFormatType::AbsoluteHighestFrameRate),
    move |frame| {
      frame_sender.send(frame).unwrap();

      thread::sleep(Duration::from_millis(FRAME_RATE));
    }
  ).unwrap();

  camera.open_stream().unwrap();

  let mut frames_buff = Arc::new(Mutex::new(Vec::<(String, u128)>::new()));
  let buffer_clone = Arc::clone(&mut frames_buff);

  thread::spawn(move || {
    while let Ok((ascii_frame, seq_num)) = ascii_frame_receiver.recv() {
      frames_buff.lock().unwrap().push((ascii_frame, seq_num));
    }
  });

  thread::spawn(move || {
    while camera.is_stream_open().unwrap() {
      let mut locked_buffer_clone = buffer_clone.lock().unwrap();

      if locked_buffer_clone.len() == 0 {
        continue;
      }

      locked_buffer_clone.sort_by(|prv, nxt| {
        prv.1.cmp(&nxt.1)
      });

      let (frame, _) = locked_buffer_clone.first().unwrap();

      ascii_frame::render(&frame.clone());
      // let compressed_frame = ascii_frame::compress_ascii_image(&frame.clone());
      // sender.send_data(PacketType::TextData, compressed_frame);

      locked_buffer_clone.remove(0);
    }
  });

  let mut seq_num: u128 = 0;

  while let Ok(raw_frame) = frame_receiver.recv() {
    seq_num = seq_num.wrapping_add(1);

    ascii_frame::spawn_buffer_to_ascii_task(
      raw_frame, 
      ascii_frame_sender.clone(), 
      seq_num
    );
  };

  Ok(())
}
