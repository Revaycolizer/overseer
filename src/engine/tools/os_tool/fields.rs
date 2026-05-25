pub fn escape_powershell_single_quoted(value: &str) -> String {
    value.replace('\'', "''")
}
