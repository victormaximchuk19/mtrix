use nokhwa::{pixel_format::RgbFormat, utils::{CameraIndex, RequestedFormat, RequestedFormatType}, Buffer, CallbackCamera};
use image::{imageops::FilterType, DynamicImage, GenericImageView};
use crossbeam::channel::{self, Sender, Receiver};
use std::thread;
use crossterm::{execute, cursor};
use std::io::{stdout, Write};

use crate::masp::{sender::MaspSender, message::PacketType};

const ASCII_CHARS: [&str; 13] = ["X", "@", "%", "#", "0", "O", "L", ";", ":", ".", ",", "'", " "]; // ASCII character gradient from dark to light

const SCALED_WIDTH: u32 = 192;
const SCALED_HEIGHT: u32 = 54;

const FRAME_RATE: u64= 1000 / 15;

fn move_cursor_to_top() {
  let mut stdout = stdout();
  execute!(stdout, cursor::MoveTo(0, 0)).unwrap();
}

fn render_frame(image: &str) {
  print!("\x1B[2J\x1B[1;1H");
  move_cursor_to_top();

  print!("{}", image);

  stdout().flush().unwrap();
}

fn compress_ascii_image(ascii_image: &str) -> Vec<u8> {
  let mut compressed: Vec<u8> = Vec::new();
  let mut chars = ascii_image.chars().peekable();
  
  while let Some(current_char) = chars.next() {
    let mut count = 1;

    // Count consecutive characters
    while let Some(&next_char) = chars.peek() {
      if next_char == current_char {
        count += 1;
        chars.next(); // Consume the character
      } else {
        break;
      }
    }

    // Push the character code and count
    compressed.push(current_char as u8);
    compressed.push(count);
    
    // If we encounter a newline, add a new line marker (usize::MAX)
    if current_char == '\n' {
      compressed.push(u8::MAX);
      compressed.push(1); // Add a new line indicator
    }
  }

  compressed
}


fn image_to_ascii(image: image::RgbImage) -> String {
  let dynamic_image = DynamicImage::ImageRgb8(image);
  let resized_image = dynamic_image.resize_exact(SCALED_WIDTH, SCALED_HEIGHT, FilterType::Nearest);

  let mut ascii_image = String::new();
  let mut last_y = 0;

  for pixel in resized_image.pixels() {
    if last_y != pixel.1 {
      ascii_image.push_str("\n");
      last_y = pixel.1;
    }

    let pixel_data = pixel.2;
    let brightness: f64 =
      ((pixel_data[0] as u64 + pixel_data[1] as u64 + pixel_data[2] as u64) / 3) as f64;
    let character_position =
      ((brightness / 255.0) * (ASCII_CHARS.len() - 1) as f64).round() as usize;
    ascii_image.push_str(ASCII_CHARS[character_position])
  }

  ascii_image
}

fn buffer_to_ascii (buffer: Buffer, ascii_sender: Sender<String>) {
  thread::spawn(move || {
    let width = buffer.resolution().width();
    let height = buffer.resolution().height();
  
    let buf = buffer.decode_image::<RgbFormat>().unwrap();
    
    let rgb_image = image::RgbImage::from_raw(
      width,
      height,
      buf.to_vec()
    ).unwrap();
  
    ascii_sender.send(image_to_ascii(rgb_image)).unwrap();
  });
}

pub async fn run(mut sender: MaspSender) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
  let (frame_sender, frame_receiver) = channel::unbounded();
  let (ascii_frame_sender, ascii_frame_receiver): (Sender<String>, Receiver<String>) = channel::unbounded();

  let mut camera = CallbackCamera::new(
    CameraIndex::Index(0),
    RequestedFormat::new::<RgbFormat>(RequestedFormatType::AbsoluteHighestFrameRate),
    move |frame| {
      frame_sender.send(frame).unwrap();
    }
  ).unwrap();

  camera.open_stream().unwrap();

  thread::spawn(move || {
    while let Ok(ascii_frame) = ascii_frame_receiver.recv() {
      render_frame(&ascii_frame);
    }
  });

  while let Ok(raw_frame) = frame_receiver.recv() {
    buffer_to_ascii(raw_frame, ascii_frame_sender.clone());
  };

  Ok(())
}
