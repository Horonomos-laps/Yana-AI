use super::*;
use std::io::Write;

fn expect(kind: OutputKind) -> OutputExpectation {
    OutputExpectation { kind, min_bytes: 1, max_bytes: 1024 * 1024 }
}

fn put(dir: &tempfile::TempDir, name: &str, bytes: &[u8]) -> std::path::PathBuf {
    let path = dir.path().join(name);
    fs::write(&path, bytes).unwrap();
    path
}

fn png() -> Vec<u8> {
    let mut v = PNG_SIGNATURE.to_vec();
    v.extend_from_slice(&[0; 20]);
    v.extend_from_slice(&PNG_IEND_TRAILER);
    v
}

fn wav(declared_extra: i64) -> Vec<u8> {
    let body = [0u8; 32];
    let declared = (4 + body.len() as i64 + declared_extra) as u32;
    let mut v = b"RIFF".to_vec();
    v.extend_from_slice(&declared.to_le_bytes());
    v.extend_from_slice(b"WAVE");
    v.extend_from_slice(&body);
    v
}

fn zip_bytes(entries: &[&str]) -> Vec<u8> {
    let mut cursor = std::io::Cursor::new(Vec::new());
    let mut writer = zip::ZipWriter::new(&mut cursor);
    for name in entries {
        writer.start_file(*name, zip::write::SimpleFileOptions::default()).unwrap();
        writer.write_all(b"hello").unwrap();
    }
    writer.finish().unwrap();
    cursor.into_inner()
}

fn ok(kind: OutputKind, name: &str, bytes: &[u8]) -> OutputReport {
    let dir = tempfile::tempdir().unwrap();
    verify_output(&put(&dir, name, bytes), expect(kind), None).unwrap()
}

fn rejects(kind: OutputKind, name: &str, bytes: &[u8]) -> String {
    let dir = tempfile::tempdir().unwrap();
    verify_output(&put(&dir, name, bytes), expect(kind), None).unwrap_err().to_string()
}

#[test]
fn accepts_a_well_formed_file_of_every_kind() {
    let mut jpeg = vec![0xFF, 0xD8, 0xFF, 0xE0];
    jpeg.extend_from_slice(&[1; 10]);
    jpeg.extend_from_slice(&[0xFF, 0xD9]);
    let mut gif = b"GIF89a".to_vec();
    gif.extend_from_slice(&[0; 8]);
    gif.push(0x3B);
    let mut pdf = b"%PDF-1.7\n".to_vec();
    pdf.extend_from_slice(b"body\n%%EOF\n");
    let mut mp4 = vec![0, 0, 0, 0x18];
    mp4.extend_from_slice(b"ftypisom");
    mp4.extend_from_slice(&[0; 8]);
    assert_eq!(ok(OutputKind::Png, "a.png", &png()).kind, OutputKind::Png);
    ok(OutputKind::Jpeg, "a.jpg", &jpeg);
    ok(OutputKind::Gif, "a.gif", &gif);
    ok(OutputKind::Pdf, "a.pdf", &pdf);
    ok(OutputKind::Mp4, "a.mp4", &mp4);
    ok(OutputKind::Wav, "a.wav", &wav(0));
    ok(OutputKind::Zip, "a.zip", &zip_bytes(&["a.txt"]));
}

#[test]
fn report_carries_real_size_and_hash() {
    let report = ok(OutputKind::Png, "a.png", &png());
    assert_eq!(report.bytes, png().len() as u64);
    assert_eq!(report.evidence.byte_count, Some(report.bytes));
    assert_eq!(report.evidence.sha256.as_ref().map(String::len), Some(64));
}

#[test]
fn rejects_truncated_files() {
    let full = png();
    assert!(rejects(OutputKind::Png, "t.png", &full[..full.len() - 4]).contains("IEND"));
    assert!(rejects(OutputKind::Pdf, "t.pdf", b"%PDF-1.7\nbody").contains("%%EOF"));
    assert!(rejects(OutputKind::Jpeg, "t.jpg", &[0xFF, 0xD8, 0xFF, 0xE0, 1, 2]).contains("end-of-image"));
    assert!(rejects(OutputKind::Gif, "t.gif", b"GIF89a\x00\x00\x00").contains("trailer"));
    assert!(rejects(OutputKind::Wav, "t.wav", &wav(10)).contains("declares"));
}

#[test]
fn rejects_wrong_format_and_empty_and_zero_entry_zip() {
    assert!(rejects(OutputKind::Png, "x.png", b"<html>not an image</html>").contains("PNG"));
    assert!(rejects(OutputKind::Pdf, "x.pdf", &png()).contains("PDF header"));
    assert!(rejects(OutputKind::Mp4, "x.mp4", b"short").contains("ftyp"));
    assert!(rejects(OutputKind::Png, "empty.png", b"").contains("minimum"));
    assert!(rejects(OutputKind::Zip, "bad.zip", b"PK-not-a-zip").contains("zip"));
    assert!(rejects(OutputKind::Zip, "none.zip", &zip_bytes(&[])).contains("no entries"));
}

#[test]
fn enforces_size_bounds_on_both_sides() {
    let dir = tempfile::tempdir().unwrap();
    let path = put(&dir, "a.png", &png());
    let small = OutputExpectation { kind: OutputKind::Png, min_bytes: 10_000, max_bytes: 1 << 20 };
    assert!(verify_output(&path, small, None).unwrap_err().to_string().contains("below"));
    let big = OutputExpectation { kind: OutputKind::Png, min_bytes: 1, max_bytes: 10 };
    assert!(matches!(verify_output(&path, big, None), Err(CapabilityError::TooLarge { .. })));
}

#[test]
fn max_length_input_does_not_crash() {
    let mut data = PNG_SIGNATURE.to_vec();
    data.extend(std::iter::repeat(0u8).take(65_536));
    data.extend_from_slice(&PNG_IEND_TRAILER);
    ok(OutputKind::Png, "big.png", &data);
}

#[test]
fn refuses_symlink_directory_and_missing_path() {
    let dir = tempfile::tempdir().unwrap();
    let real = put(&dir, "real.png", &png());
    let link = dir.path().join("link.png");
    std::os::unix::fs::symlink(&real, &link).unwrap();
    assert!(matches!(verify_output(&link, expect(OutputKind::Png), None), Err(CapabilityError::NotAFile { .. })));
    assert!(matches!(verify_output(dir.path(), expect(OutputKind::Png), None), Err(CapabilityError::NotAFile { .. })));
    let missing = dir.path().join("nope.png");
    assert!(matches!(verify_output(&missing, expect(OutputKind::Png), None), Err(CapabilityError::NotFound { .. })));
}

#[test]
fn null_bytes_and_path_traversal_text_in_name_are_just_data() {
    let dir = tempfile::tempdir().unwrap();
    let path = put(&dir, "..%2f..%2fetc.png", &png());
    assert!(verify_output(&path, expect(OutputKind::Png), None).is_ok());
    assert!(rejects(OutputKind::Png, "n.png", b"\x00\x00\x00\x00\x00\x00\x00\x00\x00").contains("PNG"));
}
