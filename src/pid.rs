#[derive(Debug, Clone, Copy)]
struct ImageFlags { flags: u32 }

impl ImageFlags {
    fn use_transparency(&self) -> bool {
        self.flags & 0x01 != 0
    }

    fn use_video_memory(&self) -> bool {
        self.flags & 0x02 != 0
    }

    fn use_system_memory(&self) -> bool {
        self.flags & 0x04 != 0
    }

    fn is_fliped_horizontally(&self) -> bool {
        self.flags & 0x08 != 0
    }

    fn is_fliped_vertically(&self) -> bool {
        self.flags & 0x10 != 0
    }

    fn compression_method(&self) -> CompressionMethod {
        if self.flags & 0x20 == 0 {
            CompressionMethod::Default
        } else {
            CompressionMethod::RunLengthEncoding
        }
    }

    fn has_lights(&self) -> bool {
        self.flags & 0x40 != 0
    }

    fn has_palette(&self) -> bool {
        self.flags & 0x80 != 0
    }
}

struct Buffer {
    data: Vec<u8>
}

impl Buffer {
    fn new(size: usize) -> Buffer {
        Buffer { 
            data: Vec::<u8>::with_capacity(size),
        }
    }

    fn write_u8(&mut self, n: usize, b: u8) {
        self.data[n] = b;
    }

    fn write_u32_le(&mut self, n: usize, u: u32) {
        let bytes = u.to_le_bytes();
        for i in 0..4 {
            self.data[n + i] = bytes[i];
        }
    }
}

struct PidDataCursor<'a> {
    data: &'a [u8],
    offset: usize,
}

impl <'a> PidDataCursor<'a> {
    fn next_u8(&mut self) -> u8 {
        let output = self.data[self.offset];
        self.offset += 1;
        output
    }

    fn next_u32_le(&mut self) -> u32 {
        let output = u32::from_le_bytes(self.data[self.offset..self.offset + 4].try_into().unwrap());
        self.offset += 4;
        output
    }

    fn next_i32_le(&mut self) -> i32 {
        let output = i32::from_le_bytes(self.data[self.offset..self.offset + 4].try_into().unwrap());
        self.offset += 4;
        output
    }
}


#[derive(Clone, Copy, Debug)]
struct Rgb {
    r: u8,
    g: u8,
    b: u8
}

#[derive(Debug)]
struct PidImage {
    id: i32,
    flags: ImageFlags,
    width: u32,
    height: u32,
    user_values: [i32; 4],
    pixels: Vec<u8>,
    palette: Option<[Rgb; 256]>,
}

#[derive(Debug)]
enum CompressionMethod { Default, RunLengthEncoding }


fn decompress_default(data: &mut PidDataCursor, pixels: &mut Buffer, pixels_count: usize) {
    let mut pixel = 0;
    while pixel < pixels_count {
        let n: u8;
        let b: u8;
        let a = data.next_u8();
        if a > 192 {
            n = a - 192;
            b = data.next_u8();
        } else {
            n = 1;
            b = a;
        }
        for _ in 0..n {
            pixels.write_u8(pixel, b);
            pixel += 1;
        }
    }
}

fn decompress_run_length_encoding(data: &mut PidDataCursor, pixels: &mut Buffer, pixels_count: usize) {
    let mut pixel = 0;
    while pixel < pixels_count {
        let a = data.next_u8();
        if a > 128 {
            let j = a - 128;
            for _ in 0..j {
                pixels.write_u8(pixel, 0);
                pixel += 1;
            }
        } else {
            for _ in 0..a {
                let b = data.next_u8();
                pixels.write_u8(pixel, b);
                pixel += 1;
            }
        }
    }
}

pub fn decode_pid(data: &[u8]) -> PidImage {
    let mut cur = PidDataCursor { data, offset: 0 };
    let id = cur.next_i32_le();

    // test
    let flags = ImageFlags { flags: cur.next_u32_le() };
    let width = cur.next_u32_le();
    let height = cur.next_u32_le();
    // end test
    let mut user_values: [i32; 4] = [0; 4];
    user_values[0] = cur.next_i32_le();
    user_values[1] = cur.next_i32_le();
    user_values[2] = cur.next_i32_le();
    user_values[3] = cur.next_i32_le();
    let pixels_count = (width * height) as usize;
    let mut pixels = Buffer::new(pixels_count);

    match flags.compression_method() {
        CompressionMethod::Default => decompress_default(&mut cur, &mut pixels, pixels_count),
        CompressionMethod::RunLengthEncoding => decompress_run_length_encoding(&mut cur, &mut pixels, pixels_count),
    }

    let palette = if flags.has_palette() {
        let mut p: [Rgb; 256] = [Rgb { r: 0, g: 0, b: 0}; 256];
        for c in &mut p {
            c.r = cur.next_u8();
            c.g = cur.next_u8();
            c.b = cur.next_u8();
        }
        Some(p)
    } else {
        None
    };
    
    PidImage { id, flags, width, height, user_values, pixels: pixels.data, palette }
}