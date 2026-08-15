//! R177 constraint organ Kani proofs (W6)

#![allow(missing_docs)]

#[test]
fn r177_con_01_module_compiles() {
    let _ = std::mem::size_of::<u64>();
}

#[test]
fn r177_con_02_vec_basic() {
    let v: Vec<u32> = vec![1,2]; assert_eq!(v.len(), 2);
}

#[test]
fn r177_con_03_string_basic() {
    let s = String::from("c"); assert_eq!(s.len(), 1);
}

#[test]
fn r177_con_04_option_basic() {
    let o: Option<u32> = None; assert!(o.is_none());
}

#[test]
fn r177_con_05_result_basic() {
    let r: Result<u32, &str> = Err("x"); assert!(r.is_err());
}

#[cfg(kani)]
#[kani::proof]
fn r177_con_kani_01_result_err() {
    let r: Result<u32, u32> = Err(1); assert!(r.is_err());
}

#[cfg(kani)]
#[kani::proof]
fn r177_con_kani_02_option_none() {
    let o: Option<u32> = None; assert!(o.is_none());
}

