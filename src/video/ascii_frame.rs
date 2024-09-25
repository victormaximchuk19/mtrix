use nokhwa::{Buffer, utils::FrameFormat};
use crossbeam::channel::Sender;
use image::{self, ImageFormat};

use std::{process, thread};
use std::io::{self, Write, Cursor};

const ASCII_CHARS: [&str; 11] = ["@", "#", "0", "O", "*", ";", ":", ".", ",", "'", " "];

const ASCII_FRAME_WIDTH: usize = 192;
const ASCII_FRAME_HEIGHT: usize = 54;

pub fn render(ascii_frame: &String) {
  print!("\x1B[2J\x1B[1;1H");
  println!("\r{}", ascii_frame);
  io::stdout().flush().unwrap();
}

pub fn compress_ascii_image(ascii_image: &str) -> Vec<u8> {
  let mut compressed: Vec<u8> = Vec::new();
  let mut chars = ascii_image.chars().peekable();
  
  while let Some(current_char) = chars.next() {
    let mut count = 1;

    while let Some(&next_char) = chars.peek() {
      if next_char == current_char {
        count += 1;
        chars.next();
      } else {
        break;
      }
    }

    compressed.push(current_char as u8);
    compressed.push(count as u8);
  }

  compressed
}

pub fn decompress_ascii_image(payload: Vec<u8>) -> String {
  let mut decompressed = String::new();
  let mut index = 0;

  while index < payload.len() {
    let symbol_code = payload[index];
    let count = payload[index + 1];

    let character = symbol_code as char;
    decompressed.push_str(&character.to_string().repeat(count as usize));

    index += 2;
  }

  decompressed
}

fn build_ascii_from_grayscale (grayscaled: Vec<u8>) -> String {
  let mut ascii_image = String::with_capacity(grayscaled.len());

  for (i, &gray) in grayscaled.iter().enumerate() {
    let ascii_index = (gray as usize * (ASCII_CHARS.len() - 1)) / 255;
    ascii_image.push_str(ASCII_CHARS[ascii_index]);

    if (i + 1) % ASCII_FRAME_WIDTH == 0 {
      ascii_image.push('\n');
    }
  }

  ascii_image
}

pub fn jpeg_to_ascii_image(jpeg: &[u8]) -> String {
  let image_buf = image::load(Cursor::new(jpeg), ImageFormat::Jpeg)
    .unwrap()
    .resize_exact(
      ASCII_FRAME_WIDTH as u32, 
      ASCII_FRAME_HEIGHT as u32, 
      image::imageops::FilterType::Nearest
    )
    .to_luma8()
    .to_vec();

  build_ascii_from_grayscale(image_buf)
}

pub fn yuv_to_ascii_image(yuv: &[u8], original_width: usize, original_height: usize) -> String {
  let yuv_444_size = original_height * original_width * 3;
  let yuv_422_size = original_height * original_width * 2;
  let yuv_420_size = ((original_height * original_width) as f32 * 1.5) as usize;

  let is_yuv_444 = yuv_444_size == yuv.len();
  let is_yuv_422 = yuv_422_size == yuv.len();
  let is_yuv_420 = yuv_420_size == yuv.len();

  if !is_yuv_444 && !is_yuv_422 && !is_yuv_420 {
    panic!("
      ERROR: Usupported YUV format. 
      Expected file size to equal one of YUV sizes: 

      YUV 4:4:4 {yuv_444_size}, 
      YUV 4:2:2 {yuv_422_size}, 
      YUV 4:2:0 {yuv_420_size}.

      Recieved size: {}
    ", yuv.len());
  }

  let mut grayscale_values = Vec::<u8>::new();

  if is_yuv_444 || is_yuv_420 {
    for i in 0..(original_width * original_height) {
      grayscale_values.push(yuv[i]);
    }
  } else {
    for chunk in yuv.chunks_exact(4) {
      let y0 = chunk[0];
      let y1 = chunk[2];
  
      grayscale_values.push(y0);
      grayscale_values.push(y1);
    }
  }

  // let new_width = original_width / DESCALE_FACTOR_X as usize;
  // let new_height = original_height / DESCALE_FACTOR_Y as usize;

  let mut downscaled_grayscale = Vec::with_capacity(ASCII_FRAME_WIDTH * ASCII_FRAME_HEIGHT);
  
  let block_width = original_width / ASCII_FRAME_WIDTH;
  let block_height = original_height / ASCII_FRAME_HEIGHT;

  for y in 0..ASCII_FRAME_HEIGHT {
    for x in 0..ASCII_FRAME_WIDTH {
      let mut sum: usize = 0;
      let mut count = 0;

      for by in 0..block_height {
        for bx in 0..block_width {
          let orig_x = x * block_width + bx;
          let orig_y = y * block_height + by;
          let idx = orig_y * original_width + orig_x;

          sum += grayscale_values[idx] as usize;
          count += 1;
        }
      }

      let avg_gray = (sum / count) as u8;
      downscaled_grayscale.push(avg_gray);
    }
  }

  build_ascii_from_grayscale(downscaled_grayscale)
}

pub fn spawn_buffer_to_ascii_task (buffer: Buffer, ascii_sender: Sender<(String, u128)>, seq_num: u128) {
  thread::spawn(move || {
    let width = buffer.resolution().width();
    let height = buffer.resolution().height();
    
    let buf = buffer.buffer();
    let ascii_frame = match buffer.source_frame_format() {
      FrameFormat::YUYV => yuv_to_ascii_image(buf, width as usize, height as usize),
      FrameFormat::MJPEG => jpeg_to_ascii_image(buf),
      _ => {
        println!("ERROR: unsupported frame format: {}", buffer.source_frame_format());

        process::exit(1);
      }
    };

    ascii_sender.send((ascii_frame, seq_num)).unwrap();
  });
}
