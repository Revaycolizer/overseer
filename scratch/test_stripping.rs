fn strip_grounding_markers(text: &str) -> String {
    let re = regex::Regex::new(r"\[E[0-9]+\]").unwrap();
    re.replace_all(text, "").to_string().replace("  ", " ").trim().to_string()
}

fn main() {
    let test_cases = vec![
        ("Malaria is a disease [E1].", "Malaria is a disease."),
        ("Symptoms include fever [E2] and chills [E3].", "Symptoms include fever and chills."),
        ("Another claim [E123] with double  spaces.", "Another claim with double spaces."),
    ];

    for (input, expected) in test_cases {
        let result = strip_grounding_markers(input);
        if result == expected {
            println!("PASS: '{}' -> '{}'", input, result);
        } else {
            println!("FAIL: '{}' -> '{}' (expected '{}')", input, result, expected);
        }
    }
}
