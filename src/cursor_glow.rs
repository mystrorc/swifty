use std::env;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::os::unix::fs as unix_fs;
use std::path::{Path, PathBuf};

// Xcursor binary constants
const XCURSOR_MAGIC: u32 = 0x72756358;

#[allow(dead_code)]
#[derive(Clone)]
struct ImageChunk {
    header_size: u32,
    chunk_type: u32,
    chunk_subtype: u32,
    version: u32,
    width: u32,
    height: u32,
    xhot: u32,
    yhot: u32,
    delay: u32,
    pixels: Vec<u8>,
}

pub fn parse_hex_color(hex: &str) -> (u8, u8, u8) {
    let clean = hex.trim_start_matches('#');
    if clean.len() >= 6 {
        let r = u8::from_str_radix(&clean[0..2], 16).unwrap_or(0);
        let g = u8::from_str_radix(&clean[2..4], 16).unwrap_or(210);
        let b = u8::from_str_radix(&clean[4..6], 16).unwrap_or(255);
        (r, g, b)
    } else {
        (0, 210, 255) // Default neon cyan
    }
}

pub fn ensure_glow_theme(base_theme: &str, color_hex: &str, radius: usize) -> Result<String, String> {
    let (r, g, b) = parse_hex_color(color_hex);
    let clean_hex = format!("{r:02x}{g:02x}{b:02x}");
    let theme_name = format!("{base_theme}-glow-{clean_hex}-r{radius}");

    let home = env::var("HOME").unwrap_or_default();
    let theme_dir = PathBuf::from(&home).join(".local/share/icons").join(&theme_name);
    let cursors_dir = theme_dir.join("cursors");

    // If left_ptr already exists, theme is cached and ready
    if cursors_dir.join("left_ptr").exists() {
        return Ok(theme_name);
    }

    // Locate source cursor directory
    let src_candidates = [
        PathBuf::from(&home).join(".local/share/icons").join(base_theme).join("cursors"),
        PathBuf::from("/usr/share/icons").join(base_theme).join("cursors"),
    ];

    let src_dir = src_candidates.into_iter().find(|p| p.is_dir()).ok_or_else(|| {
        format!("Source cursor theme '{base_theme}' not found")
    })?;

    fs::create_dir_all(&cursors_dir).map_err(|e| e.to_string())?;

    // Create index.theme
    let index_theme = format!(
        "[Icon Theme]\nName={theme_name}\nComment={base_theme} with custom luminous glow halo\nInherits={base_theme}\n"
    );
    let _ = fs::write(theme_dir.join("index.theme"), index_theme);

    let entries: Vec<_> = fs::read_dir(&src_dir)
        .map_err(|e| e.to_string())?
        .filter_map(|e| e.ok())
        .collect();

    let num_threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4).min(8);
    let chunk_size = (entries.len() + num_threads - 1) / num_threads;

    std::thread::scope(|s| {
        for chunk in entries.chunks(chunk_size) {
            let cursors_dir_ref = &cursors_dir;
            s.spawn(move || {
                for entry in chunk {
                    let path = entry.path();
                    let file_name = entry.file_name();
                    let dst_path = cursors_dir_ref.join(file_name);

                    // If already exists (e.g. from partial run), continue
                    if dst_path.exists() {
                        continue;
                    }

                    if let Ok(meta) = fs::symlink_metadata(&path) {
                        if meta.is_symlink() {
                            if let Ok(target) = fs::read_link(&path) {
                                let _ = fs::remove_file(&dst_path);
                                let _ = unix_fs::symlink(target, &dst_path);
                            }
                        } else if meta.is_file() {
                            let _ = process_cursor_file(&path, &dst_path, (r, g, b), radius);
                        }
                    }
                }
            });
        }
    });

    Ok(theme_name)
}

fn process_cursor_file(src: &Path, dst: &Path, glow_rgb: (u8, u8, u8), radius: usize) -> Result<(), String> {
    let chunks = read_xcursor_file(src)?;
    let processed: Vec<ImageChunk> = chunks
        .iter()
        .map(|c| process_image_chunk(c, glow_rgb, radius))
        .collect();
    write_xcursor_file(dst, &processed)?;
    Ok(())
}

fn process_image_chunk(
    chunk: &ImageChunk,
    glow_rgb: (u8, u8, u8),
    glow_radius: usize,
) -> ImageChunk {
    let w = chunk.width as usize;
    let h = chunk.height as usize;
    if w == 0 || h == 0 || chunk.pixels.len() < w * h * 4 {
        return chunk.clone();
    }

    // Dynamic padding scaled to cursor dimension and chosen radius
    let pad = ((w as f32 * 0.22).round() as usize).max(glow_radius + 2).min(32);
    let new_w = w + 2 * pad;
    let new_h = h + 2 * pad;
    let new_xhot = chunk.xhot + pad as u32;
    let new_yhot = chunk.yhot + pad as u32;

    let mut grid_a = vec![0.0f32; new_w * new_h];
    let mut orig_pixels = vec![[0u8; 4]; new_w * new_h];

    for y in 0..h {
        let row_src = y * w * 4;
        let row_dst = (y + pad) * new_w + pad;
        for x in 0..w {
            let src_idx = row_src + x * 4;
            let b = chunk.pixels[src_idx];
            let g = chunk.pixels[src_idx + 1];
            let r = chunk.pixels[src_idx + 2];
            let a = chunk.pixels[src_idx + 3];

            let dst_idx = row_dst + x;
            orig_pixels[dst_idx] = [b, g, r, a];
            grid_a[dst_idx] = a as f32 / 255.0;
        }
    }

    // Gaussian blur kernel
    let sigma = (pad as f32 * 0.45).max(1.5);
    let k_radius = (sigma * 2.2).ceil() as usize;
    let mut kernel = Vec::with_capacity(2 * k_radius + 1);
    let mut k_sum = 0.0f32;
    for i in -(k_radius as isize)..=(k_radius as isize) {
        let val = (-((i * i) as f32) / (2.0 * sigma * sigma)).exp();
        kernel.push(val);
        k_sum += val;
    }
    for k in &mut kernel {
        *k /= k_sum;
    }

    // Horizontal pass
    let mut temp_h = vec![0.0f32; new_w * new_h];
    for y in 0..new_h {
        let row_offset = y * new_w;
        for x in 0..new_w {
            let mut sum = 0.0f32;
            for (ki, &weight) in kernel.iter().enumerate() {
                let kx = x as isize + ki as isize - k_radius as isize;
                if kx >= 0 && (kx as usize) < new_w {
                    sum += grid_a[row_offset + kx as usize] * weight;
                }
            }
            temp_h[row_offset + x] = sum;
        }
    }

    // Vertical pass
    let mut blur_a = vec![0.0f32; new_w * new_h];
    for y in 0..new_h {
        for x in 0..new_w {
            let mut sum = 0.0f32;
            for (ki, &weight) in kernel.iter().enumerate() {
                let ky = y as isize + ki as isize - k_radius as isize;
                if ky >= 0 && (ky as usize) < new_h {
                    sum += temp_h[ky as usize * new_w + x] * weight;
                }
            }
            blur_a[y * new_w + x] = sum;
        }
    }

    // Composite: crisp original pointer OVER smooth luminous halo
    let (gr, gg, gb) = glow_rgb;
    let mut out_pixels = Vec::with_capacity(new_w * new_h * 4);

    for idx in 0..(new_w * new_h) {
        let [ob, og, or_, oa] = orig_pixels[idx];
        let oa_f = oa as f32 / 255.0;
        let ga_f = (blur_a[idx] * 1.85).min(1.0);

        let (fb, fg, fr, fa) = if oa_f >= 0.99 {
            (ob, og, or_, oa)
        } else if oa_f > 0.0 {
            let out_a = oa_f + ga_f * (1.0 - oa_f);
            let safe_a = out_a.max(0.001);
            let r = ((or_ as f32 * oa_f + gr as f32 * ga_f * (1.0 - oa_f)) / safe_a).clamp(0.0, 255.0) as u8;
            let g = ((og as f32 * oa_f + gg as f32 * ga_f * (1.0 - oa_f)) / safe_a).clamp(0.0, 255.0) as u8;
            let b = ((ob as f32 * oa_f + gb as f32 * ga_f * (1.0 - oa_f)) / safe_a).clamp(0.0, 255.0) as u8;
            let a = (out_a * 255.0).min(255.0) as u8;
            (b, g, r, a)
        } else {
            let a = (ga_f * 255.0).min(255.0) as u8;
            (gb, gg, gr, a)
        };

        out_pixels.push(fb);
        out_pixels.push(fg);
        out_pixels.push(fr);
        out_pixels.push(fa);
    }

    ImageChunk {
        header_size: 36,
        chunk_type: chunk.chunk_type,
        chunk_subtype: chunk.chunk_subtype,
        version: chunk.version,
        width: new_w as u32,
        height: new_h as u32,
        xhot: new_xhot,
        yhot: new_yhot,
        delay: chunk.delay,
        pixels: out_pixels,
    }
}

fn read_xcursor_file(path: &Path) -> Result<Vec<ImageChunk>, String> {
    let mut file = File::open(path).map_err(|e| e.to_string())?;
    let mut data = Vec::new();
    file.read_to_end(&mut data).map_err(|e| e.to_string())?;

    if data.len() < 16 {
        return Err("File too short".into());
    }

    let magic = u32::from_le_bytes(data[0..4].try_into().unwrap());
    if magic != XCURSOR_MAGIC {
        return Err("Not an Xcursor file".into());
    }

    let ntoc = u32::from_le_bytes(data[12..16].try_into().unwrap()) as usize;
    if data.len() < 16 + ntoc * 12 {
        return Err("Corrupt TOC".into());
    }

    let mut chunks = Vec::with_capacity(ntoc);
    for i in 0..ntoc {
        let toc_offset = 16 + i * 12;
        let chunk_pos = u32::from_le_bytes(data[toc_offset + 8..toc_offset + 12].try_into().unwrap()) as usize;

        if chunk_pos + 36 > data.len() {
            continue;
        }

        let hdr_size = u32::from_le_bytes(data[chunk_pos..chunk_pos + 4].try_into().unwrap());
        let ct = u32::from_le_bytes(data[chunk_pos + 4..chunk_pos + 8].try_into().unwrap());
        let cst = u32::from_le_bytes(data[chunk_pos + 8..chunk_pos + 12].try_into().unwrap());
        let ver = u32::from_le_bytes(data[chunk_pos + 12..chunk_pos + 16].try_into().unwrap());
        let width = u32::from_le_bytes(data[chunk_pos + 16..chunk_pos + 20].try_into().unwrap());
        let height = u32::from_le_bytes(data[chunk_pos + 20..chunk_pos + 24].try_into().unwrap());
        let xhot = u32::from_le_bytes(data[chunk_pos + 24..chunk_pos + 28].try_into().unwrap());
        let yhot = u32::from_le_bytes(data[chunk_pos + 28..chunk_pos + 32].try_into().unwrap());
        let delay = u32::from_le_bytes(data[chunk_pos + 32..chunk_pos + 36].try_into().unwrap());

        let pixel_bytes = (width * height * 4) as usize;
        let pixel_start = chunk_pos + 36;
        let pixel_end = pixel_start + pixel_bytes;

        if pixel_end > data.len() {
            continue;
        }

        let pixels = data[pixel_start..pixel_end].to_vec();

        chunks.push(ImageChunk {
            header_size: hdr_size,
            chunk_type: ct,
            chunk_subtype: cst,
            version: ver,
            width,
            height,
            xhot,
            yhot,
            delay,
            pixels,
        });
    }

    Ok(chunks)
}

fn write_xcursor_file(path: &Path, chunks: &[ImageChunk]) -> Result<(), String> {
    let mut header = Vec::with_capacity(16);
    header.extend_from_slice(&XCURSOR_MAGIC.to_le_bytes());
    header.extend_from_slice(&16u32.to_le_bytes()); // Header size
    header.extend_from_slice(&0x00010000u32.to_le_bytes()); // Version 1
    header.extend_from_slice(&(chunks.len() as u32).to_le_bytes()); // TOC count

    let toc_size = chunks.len() * 12;
    let mut current_pos = 16 + toc_size;

    let mut toc = Vec::with_capacity(toc_size);
    let mut chunk_data = Vec::new();

    for chunk in chunks {
        toc.extend_from_slice(&chunk.chunk_type.to_le_bytes());
        toc.extend_from_slice(&chunk.chunk_subtype.to_le_bytes());
        toc.extend_from_slice(&(current_pos as u32).to_le_bytes());

        let chunk_start = chunk_data.len();
        chunk_data.extend_from_slice(&36u32.to_le_bytes());
        chunk_data.extend_from_slice(&chunk.chunk_type.to_le_bytes());
        chunk_data.extend_from_slice(&chunk.chunk_subtype.to_le_bytes());
        chunk_data.extend_from_slice(&chunk.version.to_le_bytes());
        chunk_data.extend_from_slice(&chunk.width.to_le_bytes());
        chunk_data.extend_from_slice(&chunk.height.to_le_bytes());
        chunk_data.extend_from_slice(&chunk.xhot.to_le_bytes());
        chunk_data.extend_from_slice(&chunk.yhot.to_le_bytes());
        chunk_data.extend_from_slice(&chunk.delay.to_le_bytes());
        chunk_data.extend_from_slice(&chunk.pixels);

        let chunk_len = chunk_data.len() - chunk_start;
        current_pos += chunk_len;
    }

    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    let mut out_file = File::create(path).map_err(|e| e.to_string())?;
    out_file.write_all(&header).map_err(|e| e.to_string())?;
    out_file.write_all(&toc).map_err(|e| e.to_string())?;
    out_file.write_all(&chunk_data).map_err(|e| e.to_string())?;

    Ok(())
}
