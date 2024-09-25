#[cfg(test)]
use std::{fs::File, io::BufReader};
#[cfg(test)]
use crate::video::ascii_frame::{jpeg_to_ascii_image, yuv_to_ascii_image}; // Import your function from the main module.
#[cfg(test)]
use std::{io::Read, env::current_dir};

#[test]
fn test_jpeg_to_ascii() {
    let current_dir = current_dir().unwrap();
    let assets_path = current_dir.join("src").join("tests").join("assets");
    let image_path = assets_path.join("mock.jpeg");    
    
    let mocked_jpeg = File::open(image_path).unwrap();
    let mut reader = BufReader::new(mocked_jpeg);
    
    let mut buffer = Vec::new();
    reader.read_to_end(&mut buffer).unwrap();

    let output = jpeg_to_ascii_image(&buffer);

    let mut expected_jpeg_output_file = File::open(assets_path.join("jpeg_output.txt")).unwrap();
    let mut expected_jpeg_output = Vec::<u8>::new();
    
    expected_jpeg_output_file.read_to_end(&mut expected_jpeg_output).unwrap();

    assert_eq!(String::from_utf8(expected_jpeg_output).unwrap(), output);
}

#[test]
fn test_yuv_to_ascii() {
    let current_dir = current_dir().unwrap();
    let assets_path = current_dir.join("src").join("tests").join("assets");
    let image_path = assets_path.join("mock.yuv");
    let mocked_yuv = File::open(image_path).unwrap();
    let mut reader = BufReader::new(mocked_yuv);
    
    let mut buffer = Vec::new();
    reader.read_to_end(&mut buffer).unwrap();

    let output = yuv_to_ascii_image(&buffer, 192, 54);

    let mut expected_yuv_output_file = File::open(assets_path.join("yuv_output.txt")).unwrap();
    let mut expected_yuv_output = Vec::<u8>::new();
    
    expected_yuv_output_file.read_to_end(&mut expected_yuv_output).unwrap();

    assert_eq!(String::from_utf8(expected_yuv_output).unwrap(), output);
}
