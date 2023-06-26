use anyhow::Result;
use solhat::hotpixel;

#[test]
fn test_hotpixel_parse() -> Result<()> {
    let testfile = "tests/testdata/hotpixels.toml";
    let hpm = hotpixel::load_hotpixel_map(testfile)?;
    assert_eq!(hpm.hotpixels.len(), 4);
    assert_eq!(hpm.sensor_width, 1936);
    assert_eq!(hpm.sensor_height, 1216);
    Ok(())
}
