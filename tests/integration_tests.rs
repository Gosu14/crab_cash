#[cfg(test)]
#[allow(unused_imports)]
use crab_cash::engine::AccountSnapshot;
use csv::Trim;
use std::{fs, fs::File, path::PathBuf, process::Command};

#[test]
fn test_integration() {
    let files_dir = PathBuf::from("./tests/files");

    // Get folder count
    for entry in fs::read_dir(&files_dir)
        .expect("cannot read files_dir")
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
    {
        let case_dir = entry.path();

        // Build paths to input/output CSVs
        let input_path = case_dir.join("input.csv");
        let expected_output_path = case_dir.join("output.csv");

        assert!(input_path.exists());
        assert!(expected_output_path.exists());

        // Execute the cargo run command
        let output = Command::new("cargo")
            .arg("run")
            .arg("--")
            .arg(input_path)
            .output()
            .expect("failed to execute cargo run");

        assert!(output.status.success());

        let stdout = String::from_utf8(output.stdout).unwrap();

        // Create a CSV parser that reads from the output
        let mut rdr = csv::ReaderBuilder::new()
            .trim(Trim::All)
            .from_reader(stdout.as_bytes());

        // Deserialise into a Vec of AccountSnapshot
        let mut generated_res: Vec<AccountSnapshot> = vec![];

        for record in rdr.deserialize() {
            generated_res.push(record.unwrap());
        }

        // Create a CSV parser that reads from the expected file
        let file: File = File::open(expected_output_path).unwrap();
        let mut expected_rdr = csv::ReaderBuilder::new().trim(Trim::All).from_reader(file);

        // Deserialise into a Vec of AccountSnapshot
        let mut expected_res: Vec<AccountSnapshot> = vec![];

        for record in expected_rdr.deserialize() {
            expected_res.push(record.unwrap());
        }

        // Sorting to avoid issues with order
        generated_res.sort();
        expected_res.sort();

        assert_eq!(generated_res, expected_res);
    }
}
