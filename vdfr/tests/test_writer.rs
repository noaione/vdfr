use std::path::PathBuf;

fn get_tests_dir() -> PathBuf {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is not set");
    let tests_dir = std::path::Path::new(&manifest_dir).join("tests");

    assert!(
        tests_dir.exists(),
        "tests directory does not exist: {}",
        tests_dir.display()
    );

    tests_dir
}

fn read_input_output(test_name: &str) -> (Vec<u8>, String) {
    let tests_dir = get_tests_dir();
    let input_dir = tests_dir.join("input");
    let output_dir = tests_dir.join("output");

    let input_file = input_dir.join(format!("{}.vdf", test_name));
    let output_file = output_dir.join(format!("{}.json", test_name));

    let input = std::fs::read(&input_file).unwrap();
    let output = std::fs::read_to_string(&output_file).unwrap();

    (input, output)
}

fn compare_standard_kv_write(test_name: &str) {
    let (input, expected_output) = read_input_output(test_name);

    let serde_parsed: serde_json::Value = serde_json::from_str(&expected_output).unwrap();
    let vdf_parsed = vdfr::parser::parse_keyvalues(&input).unwrap();

    let unserde_expect = serde_json::to_string(&serde_parsed).unwrap();
    let mut cursor_writer = std::io::Cursor::new(Vec::new());
    vdfr::writer::write_keyvalues(&mut cursor_writer, &vdf_parsed).unwrap();
    let data = cursor_writer.into_inner();
    let parse_vdf_again = vdfr::parser::parse_keyvalues(&data).unwrap();
    let unserde_vdf = serde_json::to_string(&parse_vdf_again).unwrap();

    assert_eq!(unserde_expect, unserde_vdf);
}

#[test]
fn test_widestring_write() {
    compare_standard_kv_write("widestring");
}
