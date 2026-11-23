use super::*;
use std::fs;

#[test]
fn isolated_roots_and_revisions_are_enforced() {
    let first = tempfile::tempdir().unwrap();
    let second = tempfile::tempdir().unwrap();
    let name = "同名文稿.md";
    for (dir, value) in [(&first, "first"), (&second, "second")] {
        assert_eq!(read_text(dir.path(), name).unwrap()["revision"], MISSING);
        write_text(dir.path(), name, MISSING, value).unwrap();
    }
    let old = read_text(first.path(), name).unwrap();
    assert_eq!(old["text"], "first");
    assert!(write_text(
        second.path(),
        name,
        old["revision"].as_str().unwrap(),
        "wrong"
    )
    .is_err());
    write_text(first.path(), name, old["revision"].as_str().unwrap(), "new").unwrap();
    assert_eq!(fs::read_to_string(first.path().join(name)).unwrap(), "new");
    assert_eq!(
        fs::read_to_string(second.path().join(name)).unwrap(),
        "second"
    );
    assert!(write_text(first.path(), name, MISSING, "wrong").is_err());
    assert_eq!(fs::read_dir(first.path()).unwrap().count(), 1);
}

#[test]
fn boundaries_reject_nested_names_devices_credentials_and_links() {
    let directory = tempfile::tempdir().unwrap();
    let other = tempfile::tempdir().unwrap();
    let outside = other.path().join("source.txt");
    fs::write(&outside, "outside").unwrap();
    fs::hard_link(&outside, directory.path().join("link.txt")).unwrap();
    for name in [
        "../source.txt",
        "folder/file",
        "file.txt:stream",
        "NUL.txt",
        ".env",
        "link.txt",
    ] {
        assert!(read_text(directory.path(), name).is_err(), "{name}");
        assert!(
            write_text(directory.path(), name, MISSING, "unsafe").is_err(),
            "{name}"
        );
    }
    assert_eq!(fs::read_to_string(outside).unwrap(), "outside");
}
