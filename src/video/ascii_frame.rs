use nokhwa::Buffer;
use crossbeam::channel::Sender;
use std::thread;

use std::io::{self, Write};

// const ASCII_CHARS: [&str; 11] = ["@", "◯", "□", "▷" ,"✦", "✧", "◇", "✩", "▵", ".", " "];
const ASCII_CHARS: [&str; 11] = ["@", "#", "0", "O", "*", ";", ":", ".", ",", "'", " "];

const DESCALE_FACTOR_X: f32 = 10.0;
const DESCALE_FACTOR_Y: f32 = DESCALE_FACTOR_X * 2.25;

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
    compressed.push(count);
    
    if current_char == '\n' {
      compressed.push(u8::MAX);
      compressed.push(1); // Add a new line indicator
    }
  }

  compressed
}

pub fn decompress_ascii_image(payload: Vec<u8>) -> String {
  let mut decompressed = String::new();
  let mut index = 0;

  while index < payload.len() {
    let symbol_code = payload[index];
    let count = payload[index + 1];

    if symbol_code == u8::MAX {
      decompressed.push('\n');
    } else {
      let character = symbol_code as char;
      decompressed.push_str(&character.to_string().repeat(count as usize));
    }

    index += 2;
  }

  decompressed
}


fn yuyv_to_ascii_image(yuyv: &[u8], original_width: usize, original_height: usize) -> String {
  if yuyv.len() % 4 != 0 {
    panic!("Invalid YUYV buffer size.");
  }

  let mut grayscale_values = Vec::with_capacity(yuyv.len() / 2);
  
  for chunk in yuyv.chunks_exact(4) {
    let y0 = chunk[0];
    let y1 = chunk[2];
    
    grayscale_values.push(y0);
    grayscale_values.push(y1);
  }

  let new_width = original_width / DESCALE_FACTOR_X as usize;
  let new_height = original_height / DESCALE_FACTOR_Y as usize;

  let mut downscaled_grayscale = Vec::with_capacity(new_width * new_height);
  
  let block_width = original_width / new_width;
  let block_height = original_height / new_height;

  for y in 0..new_height {
    for x in 0..new_width {
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

  let mut ascii_image = String::with_capacity(downscaled_grayscale.len());

  for (i, &gray) in downscaled_grayscale.iter().enumerate() {
    let ascii_index = (gray as usize * (ASCII_CHARS.len() - 1)) / 255;
    ascii_image.push_str(ASCII_CHARS[ascii_index]);

    if (i + 1) % new_width == 0 {
      ascii_image.push('\n');
    }
  }

  ascii_image
}

pub fn spawn_buffer_to_ascii_task (buffer: Buffer, ascii_sender: Sender<(String, u128)>, seq_num: u128) {
  thread::spawn(move || {
    let width = buffer.resolution().width();
    let height = buffer.resolution().height();
    
    let buf = buffer.buffer();
    
    let ascii_frame = yuyv_to_ascii_image(buf, width as usize, height as usize);

    ascii_sender.send((ascii_frame, seq_num)).unwrap();
  });
}
