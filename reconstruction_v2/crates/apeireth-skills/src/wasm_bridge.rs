//! WASM skill executor bridge.
pub struct WasmSkillDescriptor { pub id: String, pub wasm_bytes: Vec<u8> }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn construct() {
        let d = WasmSkillDescriptor { id: "x".into(), wasm_bytes: vec![] };
        assert_eq!(d.id, "x");
    }
}
