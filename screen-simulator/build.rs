use std::{
    env,
    fs::{self, File},
    io::BufReader,
    path::Path,
};

fn byte_to_string(pixels: &[u8]) -> String {
    let mut res = "0b".to_string();
    for bit in 0..8 {
        res.push(if pixels[bit] > 0 { '1' } else { '0' });
        //res.push(if pixels[7 - bit] > 0 { '1' } else { '0' });
    }
    res.push_str(", ");
    res
}

// Example custom build script.
fn main() {
    // Tell Cargo that if the given file changes, to rerun this build script.
    println!("cargo::rerun-if-changed=../assets/pcb.png");
    let decoder = png::Decoder::new(BufReader::new(File::open("../assets/pcb.png").unwrap()));
    let mut reader = decoder.read_info().unwrap();
    let mut buf = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut buf).unwrap();
    let bytes = &buf[..info.buffer_size()];

    println!("cargo::warning=bytes={}", bytes.len());
    assert_eq!(info.width, 256);
    assert_eq!(info.height, 256);
    assert_eq!(bytes.len(), 65536);
    //assert_eq!(&byte_to_string(&[0, 0, 0, 0, 0, 0, 0, 1]), "0b10000000, ");
    assert_eq!(&byte_to_string(&[0, 0, 0, 0, 0, 0, 0, 1]), "0b00000001, ");

    let mut content = "const IMAGE: &[u8] = &[\n".to_string();
    for x in 0..256 {
        for y in 0..32 {
            let v = byte_to_string(&bytes[x * 256 + y * 8..x * 256 + (y + 1) * 8]);
            content.push_str(&v);
        }
        content.push_str("\n");
    }
    content.push_str("];");
    content.push_str("const IMAGE_WIDTH: u32 = 256;");
    let out_dir = env::var_os("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("pcb.rs");
    fs::write(&dest_path, content).unwrap();
}
