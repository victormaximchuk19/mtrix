use bytes::buf;
use nokhwa::{
  pixel_format::RgbFormat, utils::{
    CameraIndex, 
    RequestedFormat, 
    RequestedFormatType, Resolution
  }, Buffer, Camera, FormatDecoder
};
use image::{imageops::FilterType, DynamicImage, GenericImageView};

use crate::masp::{sender::MaspSender, message::PacketType};

use std::path::Path;

const ASCII_CHARS: [&str; 11] = ["@", "#", "0", "O", "L", ";", ":", ".", ",", "'", " "]; // ASCII character gradient from dark to light
const SCALED_WIDTH: u32 = 192;
const SCALED_HEIGHT: u32 = 108;

use crossterm::{execute, cursor};
use std::io::{stdout, Write};

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


fn buffer_to_ascii(buffer: &mut Buffer) -> String {
  let dynamic_image = DynamicImage::ImageRgb8(buffer_to_image(buffer));
  let resized_image = dynamic_image.resize(SCALED_WIDTH, SCALED_HEIGHT, FilterType::Nearest);

  resized_image.save(Path::new("resized_output_from_buffer.png")).unwrap();
  // let grayscale_image = resized_image.to_luma8();

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

fn buffer_to_image(buffer: &Buffer) -> image::RgbImage {
  let resolution = buffer.resolution();

  image::RgbImage::from_raw(
    resolution.width(), 
    resolution.height(), 
    buffer.buffer_bytes().to_vec()
  ).unwrap()
}

pub async fn run(mut sender: MaspSender) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
  let mut camera = Camera::new(
    CameraIndex::Index(0),
    RequestedFormat::new::<RgbFormat>(RequestedFormatType::AbsoluteHighestFrameRate)
  )?;

  // Stascii_image capturing
  camera.open_stream()?;

  loop {
    let mut frame = camera.frame()?;
    let ascii_image = buffer_to_ascii(&mut frame);

    render_frame(&ascii_image);

    let payload = compress_ascii_image(&ascii_image);

    // println!("{:?}", payload);
    // return Ok(());

    sender.send_data(PacketType::TextData, payload).await.unwrap();

    // Simulate the frame rate
    std::thread::sleep(std::time::Duration::from_millis(42));
  }
}
