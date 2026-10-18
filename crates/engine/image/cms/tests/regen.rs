// Copyright (c) 2026 ArtCraft Team and the PhotoCraft contributors.
// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Modified 2026-10-18 for the Apeireth workspace: crate identifiers, tooling knob
// names and product references renamed to neutral names; generated metadata producer
// strings neutralised; scope adaptations recorded in
// docs/04-internal/license-port/gate1-license.md. Original license terms unchanged.
//! Regenerates the shipped synthetic CMYK profile when `APEIRETH_IMAGE_REGEN_PROFILES=1`:
//!
//! ```sh
//! APEIRETH_IMAGE_REGEN_PROFILES=1 cargo test -p apeireth-image-cms --release --test regen
//! ```
//!
//! Without the variable the test only checks that the generator's model still matches the
//! shipped AToB table (cheap), so model edits are not forgotten.

use apeireth_image_cms::synth::{CmykModel, CmykParams};
use apeireth_image_cms::{Builtin, Intent, Profile, Transform};

#[test]
fn regen_or_check_coated_cmyk() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/profiles/coated-cmyk-synth.icc");
    if std::env::var_os("APEIRETH_IMAGE_REGEN_PROFILES").is_some() {
        let t = std::time::Instant::now();
        let p = apeireth_image_cms::synth::coated_cmyk();
        let bytes = p.to_bytes();
        std::fs::write(path, &*bytes).unwrap();
        eprintln!("wrote {} bytes in {:?}", bytes.len(), t.elapsed());
        return;
    }
    // The shipped AToB (colorimetric) must reproduce the model.
    let shipped = Profile::parse(&std::fs::read(path).unwrap()).unwrap();
    let lab = Builtin::LabD50.profile();
    let t = Transform::new(&shipped, lab, Intent::RelativeColorimetric, false).unwrap();
    let model = CmykModel::coated(CmykParams::default().tvi);
    let mut worst = 0.0f64;
    for c in [0.0, 0.3, 0.7, 1.0] {
        for m in [0.0, 0.45, 1.0] {
            for y in [0.0, 0.55, 1.0] {
                for k in [0.0, 0.5, 1.0] {
                    let mut o = [0.0f32; 3];
                    t.eval(&[c as f32, m as f32, y as f32, k as f32], &mut o);
                    let got = [o[0] as f64 * 100.0, o[1] as f64 * 255.0 - 128.0, o[2] as f64 * 255.0 - 128.0];
                    let want = model.lab([c, m, y, k]);
                    worst = worst.max(apeireth_image_cms::math::delta_e76(got, want));
                }
            }
        }
    }
    assert!(worst < 1.0, "shipped CMYK profile is stale (max ΔE {worst}); regenerate it");
}
