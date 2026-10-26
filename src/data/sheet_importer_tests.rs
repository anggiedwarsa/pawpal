//! Unit tests for data/sheet_importer.rs.

use super::*;

#[test]
fn parses_rows_fps_and_speed() {
    let rows = parse("walk=9@10/30,sit=0+1@5,idle=2,broken");
    assert_eq!(rows.len(), 3);
    assert_eq!((rows[0].action.as_str(), rows[0].rows.as_slice(), rows[0].fps, rows[0].speed), ("walk", &[9][..], 10.0, 30.0));
    assert_eq!((rows[1].rows.as_slice(), rows[1].fps, rows[1].speed), (&[0, 1][..], 5.0, 0.0));
    assert_eq!(rows[2].fps, 8.0, "fps defaults to 8");
}

#[test]
fn missing_sheet_is_an_error() {
    let err = import_sheet(Path::new("/nonexistent/sheet.png"), "x", &parse("walk=0"), None).unwrap_err();
    assert!(err.contains("sprite sheet"));
}
