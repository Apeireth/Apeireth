use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenFrame {
    pub width: u32,
    pub height: u32,
    pub timestamp_ms: u64,
    pub perceptual_hash: u64,
    pub changed_regions_count: usize,
}

pub struct ScreenCapture {
    last_hash: u64,
    last_frame: Option<ScreenFrame>,
    diff_threshold: f64,
}

impl ScreenCapture {
    pub fn new(diff_threshold: f64) -> Self {
        Self {
            last_hash: 0,
            last_frame: None,
            diff_threshold,
        }
    }

    /// Computes 64-bit perceptual hash (pHash) from raw RGB/grayscale image buffer
    pub fn compute_phash(pixels: &[u8], width: u32, height: u32) -> u64 {
        if pixels.is_empty() || width == 0 || height == 0 {
            return 0;
        }

        // Fast 8x8 block mean hashing
        let block_w = (width / 8).max(1);
        let block_h = (height / 8).max(1);
        let mut block_means = [0f64; 64];

        for by in 0..8 {
            for bx in 0..8 {
                let mut sum = 0u64;
                let mut count = 0u64;
                for y in (by * block_h)..((by + 1) * block_h).min(height) {
                    for x in (bx * block_w)..((bx + 1) * block_w).min(width) {
                        let idx = (y * width + x) as usize;
                        if idx < pixels.len() {
                            sum += pixels[idx] as u64;
                            count += 1;
                        }
                    }
                }
                block_means[(by * 8 + bx) as usize] = if count > 0 { sum as f64 / count as f64 } else { 0.0 };

            }
        }

        let overall_mean: f64 = block_means.iter().sum::<f64>() / 64.0;
        let mut hash = 0u64;
        for (i, &mean) in block_means.iter().enumerate() {
            if mean > overall_mean {
                hash |= 1u64 << i;
            }
        }
        hash
    }

    /// Hamming distance between two 64-bit perceptual hashes
    pub fn hamming_distance(h1: u64, h2: u64) -> u32 {
        (h1 ^ h2).count_ones()
    }

    /// Evaluates if new frame has significant visual change
    pub fn process_frame(&mut self, pixels: &[u8], width: u32, height: u32, timestamp_ms: u64) -> (ScreenFrame, bool) {
        let hash = Self::compute_phash(pixels, width, height);
        let dist = Self::hamming_distance(self.last_hash, hash);
        let change_ratio = dist as f64 / 64.0;
        let is_significant_change = change_ratio >= self.diff_threshold || self.last_frame.is_none();

        let frame = ScreenFrame {
            width,
            height,
            timestamp_ms,
            perceptual_hash: hash,
            changed_regions_count: if is_significant_change { dist as usize } else { 0 },
        };

        if is_significant_change {
            self.last_hash = hash;
            self.last_frame = Some(frame.clone());
        }

        (frame, is_significant_change)
    }

    /// Captures the actual physical primary monitor screen
    #[cfg(target_os = "windows")]
    pub fn capture_native_screen() -> Option<(Vec<u8>, u32, u32)> {
        use std::ptr::null_mut;
        use winapi::um::wingdi::{
            BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDIBits,
            SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, SRCCOPY,
        };
        use winapi::um::winuser::{GetDC, GetSystemMetrics, ReleaseDC, SM_CXSCREEN, SM_CYSCREEN};

        unsafe {
            let width = GetSystemMetrics(SM_CXSCREEN);
            let height = GetSystemMetrics(SM_CYSCREEN);
            if width <= 0 || height <= 0 {
                return None;
            }

            let hdc_screen = GetDC(null_mut());
            if hdc_screen.is_null() {
                return None;
            }

            let hdc_mem = CreateCompatibleDC(hdc_screen);
            if hdc_mem.is_null() {
                ReleaseDC(null_mut(), hdc_screen);
                return None;
            }

            let hbm = CreateCompatibleBitmap(hdc_screen, width, height);
            if hbm.is_null() {
                DeleteDC(hdc_mem);
                ReleaseDC(null_mut(), hdc_screen);
                return None;
            }

            let old_bm = SelectObject(hdc_mem, hbm as *mut _);
            BitBlt(hdc_mem, 0, 0, width, height, hdc_screen, 0, 0, SRCCOPY);

            let mut bi: BITMAPINFO = std::mem::zeroed();
            bi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
            bi.bmiHeader.biWidth = width;
            bi.bmiHeader.biHeight = -height;
            bi.bmiHeader.biPlanes = 1;
            bi.bmiHeader.biBitCount = 24;
            bi.bmiHeader.biCompression = BI_RGB;

            let row_size = ((width * 3 + 3) & !3) as usize;
            let mut buffer = vec![0u8; row_size * height as usize];

            let lines = GetDIBits(
                hdc_mem,
                hbm,
                0,
                height as u32,
                buffer.as_mut_ptr() as *mut _,
                &mut bi,
                DIB_RGB_COLORS,
            );

            SelectObject(hdc_mem, old_bm);
            DeleteObject(hbm as *mut _);
            DeleteDC(hdc_mem);
            ReleaseDC(null_mut(), hdc_screen);

            if lines > 0 {
                Some((buffer, width as u32, height as u32))
            } else {
                None
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    pub fn capture_native_screen() -> Option<(Vec<u8>, u32, u32)> {
        None
    }
}

