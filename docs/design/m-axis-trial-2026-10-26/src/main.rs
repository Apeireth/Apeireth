//! M 轴 · 3D 高斯溅射 **隔离试验田**（D19 裁定「暂不立项 ⇒ 后续若推进先做隔离试验田」）。
//!
//! **验收标准（抄 D19 侦察报告 §10 的 A1/A2/A4）**：
//! A1 固定 `.ply`（**自建**，不用上游示例资产）+ 固定相机（pos/yaw/pitch/fov/z 全写死）
//!    + 1280×720 + `Rgba8UnormSrgb` + **不开窗口**（全链路无 `Surface`）；
//! A2 **同机同驱动连续两次运行，PNG 字节一致**（SHA-256 相等）；
//!    若不成立，降级为「≥99.9% 像素差 ≤1 LSB 且 PSNR ≥ 50 dB」；
//! A4 画面非空：alpha 覆盖率 ≥ 5%，点云包围盒中心落在画面中心 ±5%。
//!
//! **纪律**：本目录是**独立 workspace**（`[workspace]` 自带）⇒ 不进主仓依赖图、
//! 不碰主 `Cargo.lock`、不碰 `deny.toml`。
//! **诚实边界**：本程序只证明「**能不能 headless 出帧、能不能字节可复现**」，
//! **不**证明「应该搬进主仓」—— 搬入的代价（依赖闭包 / deny 冲突 / vendored 隔离）
//! 由 D19 侦察报告单独记账。

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use glam::{Vec3, UVec2};
use wgpu_3dgs_core::{
    GaussianDisplayMode, GaussianMaxStdDev, GaussianShDegree, Gaussians, GaussiansSource,
};
use wgpu_3dgs_viewer::Viewer;

const W: u32 = 1280;
const H: u32 = 720;
const N: usize = 2000;

/// **自建固定点云**：确定性球面壳（位置/颜色全由索引算出 ⇒ 无随机 ⇒ 可复现）。
fn write_ply(path: &Path) {
    let mut out = BufWriter::new(File::create(path).expect("create ply"));
    writeln!(out, "ply").unwrap();
    writeln!(out, "format ascii 1.0").unwrap();
    writeln!(out, "element vertex {N}").unwrap();
    writeln!(out, "property float x\nproperty float y\nproperty float z").unwrap();
    writeln!(out, "property float nx\nproperty float ny\nproperty float nz").unwrap();
    writeln!(out, "property float f_dc_0\nproperty float f_dc_1\nproperty float f_dc_2").unwrap();
    for i in 0..45 {
        writeln!(out, "property float f_rest_{i}").unwrap();
    }
    writeln!(out, "property float opacity").unwrap();
    writeln!(out, "property float scale_0\nproperty float scale_1\nproperty float scale_2").unwrap();
    writeln!(out, "property float rot_0\nproperty float rot_1\nproperty float rot_2\nproperty float rot_3").unwrap();
    writeln!(out, "end_header").unwrap();

    // SH DC 系数常数：color = 0.5 + C0 * f_dc ⇒ 反解 f_dc = (color - 0.5) / C0
    const C0: f32 = 0.282_094_79;
    // 3DGS 约定：opacity 存 logit、scale 存 log ⇒ 下面写的是**存值**，不是实值
    const OPACITY_STORED: f32 = 2.197_224_6; // ln(0.9 / 0.1)
    const SCALE_STORED: f32 = -3.912_023; // ln(0.02)

    for i in 0..N {
        // Fibonacci 球面（确定性、无随机）
        let t = (i as f32 + 0.5) / (N as f32);
        let phi = std::f32::consts::TAU * ((i as f32) * 0.618_034).fract();
        let cos_theta = 1.0 - 2.0 * t;
        let sin_theta = (1.0 - cos_theta * cos_theta).max(0.0).sqrt();
        let (x, y, z) = (sin_theta * phi.cos(), cos_theta, sin_theta * phi.sin());
        // 半径 1.0 的壳 + 轻微起伏（固定公式）
        let r = 1.0 + 0.05 * ((i % 17) as f32 / 17.0);
        let (px, py, pz) = (x * r, y * r, z * r);
        // 颜色：按经度做色相环（确定性）
        let hue = phi / std::f32::consts::TAU;
        let (cr, cg, cb) = hsv_to_rgb(hue, 0.75, 1.0);

        write!(out, "{px:.6} {py:.6} {pz:.6} ").unwrap();
        write!(out, "0.0 0.0 0.0 ").unwrap();
        write!(
            out,
            "{:.6} {:.6} {:.6} ",
            (cr - 0.5) / C0,
            (cg - 0.5) / C0,
            (cb - 0.5) / C0
        )
        .unwrap();
        for _ in 0..45 {
            write!(out, "0.0 ").unwrap();
        }
        write!(out, "{OPACITY_STORED:.6} ").unwrap();
        write!(out, "{SCALE_STORED:.6} {SCALE_STORED:.6} {SCALE_STORED:.6} ").unwrap();
        writeln!(out, "1.0 0.0 0.0 0.0").unwrap();
    }
    out.flush().unwrap();
}

fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (f32, f32, f32) {
    let i = (h * 6.0).floor();
    let f = h * 6.0 - i;
    let p = v * (1.0 - s);
    let q = v * (1.0 - f * s);
    let t = v * (1.0 - (1.0 - f) * s);
    match (i as i32) % 6 {
        0 => (v, t, p),
        1 => (q, v, p),
        2 => (p, v, t),
        3 => (p, q, v),
        4 => (t, p, v),
        _ => (v, p, q),
    }
}

fn main() {
    let out_png = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/tmp/m-axis-trial/out.png".to_string());
    let ply = Path::new("/tmp/m-axis-trial/fixed.ply");
    write_ply(ply);
    println!("[trial] 自建固定 PLY: {}（{N} 个高斯）", ply.display());

    let gaussians = Gaussians::read_from_file(ply, GaussiansSource::Ply).expect("read ply");
    println!("[trial] 读入高斯数: {}", gaussians.len());

    // ── 设备：优先真 GPU，退到 CPU（llvmpipe / lavapipe）—— 两条路都试
    let instance = wgpu::Instance::default();
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::None,
        force_fallback_adapter: false,
        compatible_surface: None,
        apply_limit_buckets: false,
    }))
    .expect("request adapter");
    let info = adapter.get_info();
    println!(
        "[trial] adapter: {:?} / {:?} / backend {:?}",
        info.name, info.device_type, info.backend
    );
    println!(
        "[trial] limits: max_storage_buffer_binding_size={} max_buffer_size={} max_compute_workgroup_size_x={}",
        adapter.limits().max_storage_buffer_binding_size,
        adapter.limits().max_buffer_size,
        adapter.limits().max_compute_workgroup_size_x
    );
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("trial"),
        required_features: wgpu::Features::empty(),
        // ⚠️ **不能用 downlevel_webgl2_defaults()**：它的 storage-buffer 上限为 0
        // ⇒ Viewer::new 报 ModelSizeExceedsDeviceLimit { device_limit: 0 }（实测）。
        // 用 adapter 的实际 limits（headless 也拿得到）。
        required_limits: adapter.limits(),
        memory_hints: wgpu::MemoryHints::default(),
        trace: wgpu::Trace::Off,
        experimental_features: wgpu::ExperimentalFeatures::disabled(),
    }))
    .expect("request device");
    device.on_uncaptured_error(std::sync::Arc::new(|err| {
        eprintln!("[trial] wgpu 未捕获错误: {err}");
    }));

    // ── 离屏目标纹理（**无 Surface** ⇒ headless）
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("offscreen"),
        size: wgpu::Extent3d {
            width: W,
            height: H,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

    // ── Viewer（内部做 3 次 WESL 编译 + 管线创建）
    let mut viewer = Viewer::<wgpu_3dgs_viewer::DefaultGaussianPod>::new(
        &device,
        wgpu::TextureFormat::Rgba8UnormSrgb,
        &gaussians,
    )
    .expect("viewer new");

    // ── 固定相机（全部写死）
    let mut camera = wgpu_3dgs_viewer::Camera::new(0.1..1e4, 60f32.to_radians());
    // ⚠️ 相机约定（源码 camera.rs `get_forward`）：forward = (cos(pitch)·sin(yaw),
    // sin(pitch), cos(pitch)·cos(yaw)) ⇒ **yaw=0 时朝 +Z 望**。
    // ⇒ 相机必须放在 **-Z** 才能看向原点（放 +Z 会背对 ⇒ 全黑帧，实测踩到）。
    camera.pos = Vec3::new(0.0, 0.0, -3.0);
    camera.yaw = 0.0;
    camera.pitch = 0.0;
    viewer.update_camera(&queue, &camera, UVec2::new(W, H));

    viewer.update_gaussian_transform(
        &queue,
        1.0,
        GaussianDisplayMode::Splat,
        GaussianShDegree::new(3).expect("sh degree"),
        false,
        GaussianMaxStdDev::new(2.0).expect("max std dev"),
    );

    // ── 渲染
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    viewer.render(&mut encoder, &view);

    // ── 回读（**库无纹理回读接口 ⇒ 这一截要我们自己写**）
    let bytes_per_row = W * 4;
    assert_eq!(bytes_per_row % 256, 0, "copy_texture_to_buffer 要求行宽 256B 对齐");
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("readback"),
        size: (bytes_per_row * H) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(bytes_per_row),
                rows_per_image: Some(H),
            },
        },
        wgpu::Extent3d {
            width: W,
            height: H,
            depth_or_array_layers: 1,
        },
    );
    queue.submit([encoder.finish()]);

    // ── 取像素 + 写 PNG
    let slice = buffer.slice(..);
    let (tx, rx) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |r| {
        let _ = tx.send(r);
    });
    device.poll(wgpu::PollType::wait_indefinitely()).expect("poll");
    rx.recv().expect("map").expect("map ok");
    let data = slice.get_mapped_range().expect("mapped range");
    let mut out = Vec::with_capacity((W * H * 4) as usize);
    for y in 0..H {
        let start = (y * bytes_per_row) as usize;
        out.extend_from_slice(&data[start..start + (W * 4) as usize]);
    }
    // ⚠️ 必须先释放映射视图再 unmap（wgpu 会拒绝："still has accessible mapped views"）
    drop(data);
    buffer.unmap();

    // A4：画面非空（覆盖率 + 画面中心命中）
    let mut nonzero = 0u32;
    let mut center_hit = 0u32;
    let mut center_total = 0u32;
    for (idx, px) in out.chunks_exact(4).enumerate() {
        let x = (idx as u32) % W;
        let y = (idx as u32) / W;
        let lit = px[0] > 0 || px[1] > 0 || px[2] > 0;
        if lit {
            nonzero += 1;
        }
        if (x as i64 - W as i64 / 2).abs() <= (W as i64 / 40)
            && (y as i64 - H as i64 / 2).abs() <= (H as i64 / 40)
        {
            center_total += 1;
            if lit {
                center_hit += 1;
            }
        }
    }
    let coverage = nonzero as f64 / (W * H) as f64;
    let center_cov = center_hit as f64 / center_total.max(1) as f64;
    println!(
        "[trial] A4 画面非空：非黑像素占比 {coverage:.4}（门 ≥0.05）｜中心区命中占比 {center_cov:.4}"
    );

    let file = File::create(&out_png).expect("create png");
    let mut encoder = png::Encoder::new(BufWriter::new(file), W, H);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().expect("png header");
    writer.write_image_data(&out).expect("png data");
    writer.finish().expect("png finish");
    println!("[trial] PNG 已写: {out_png}");
}
