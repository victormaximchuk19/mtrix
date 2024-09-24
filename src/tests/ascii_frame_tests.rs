#[cfg(test)]
use std::{fs::File, io::BufReader};
#[cfg(test)]
use crate::video::ascii_frame::{jpeg_to_ascii_image, yuv_to_ascii_image}; // Import your function from the main module.
#[cfg(test)]
use std::{io::Read, env::current_dir};

#[cfg(test)]
const JPEG_OUTPUT: &str = r#"
@@@@                        @@@@
   @@@@@                @@@@@'  
      @@@@@         .@@@@@      
         '@@@@@  @@@@@          
             @@@@@0             
          @@@@@  @@@@@          
      @@@@@          @@@@@      
   @@@@@              ' @@@@@   
@@@@                        @@@@
"#;

#[cfg(test)]
const YUV_OUTPUT: &str = r#"
#@@0;,'                 '',;0@@#
',;0@@#*:'            ':*#@@0;,'
    '.*#@@0;,''  '',;0@@#*.'    
       ',;O@@@O::O#@@O;,        
          '';@@@@@@;''          
        ':O@@@O::O@@@O:,        
    '.*0@@0*.'    '.;0@@#*.''   
 ,:O@@@O:''           ':O#@@0;,'
#@@0;.'                  ',;0@@#
"#; 

#[test]
fn test_jpeg_to_ascii() {
    let current_dir = current_dir().unwrap();
    let image_path = current_dir.join("src").join("tests").join("assets").join("mock.jpeg");
    let mocked_jpeg = File::open(image_path).unwrap();
    let mut reader = BufReader::new(mocked_jpeg);
    
    let mut buffer = Vec::new();
    reader.read_to_end(&mut buffer).unwrap();

    let output = jpeg_to_ascii_image(&buffer, 320, 180);

    assert_eq!(output.trim(), JPEG_OUTPUT.trim());
}

#[test]
fn test_yuv_to_ascii() {
    let current_dir = current_dir().unwrap();
    let image_path = current_dir.join("src").join("tests").join("assets").join("mock.yuv");
    let mocked_jpeg = File::open(image_path).unwrap();
    let mut reader = BufReader::new(mocked_jpeg);
    
    let mut buffer = Vec::new();
    reader.read_to_end(&mut buffer).unwrap();

    let output = yuv_to_ascii_image(&buffer, 320, 180);

    assert_eq!(output.trim(), YUV_OUTPUT.trim());
}
