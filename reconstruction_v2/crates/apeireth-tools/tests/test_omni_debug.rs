use apeireth_tools::vision::OmniParser;

#[test]
fn test_omni_debug() {
    let elements = OmniParser::detect_live_elements(1707, 1067);
    println!("DEBUG: Total detected live elements = {}", elements.len());
    for el in &elements {
        println!("- [{}] {:?} \"{}\" at {:?}", el.id, el.element_type, el.label, el.bbox);
    }
}
