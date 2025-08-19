use rust_yaml::Yaml;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let yaml_content = r"
name: 'rust-yaml'
version: '0.1.0'
features:
  - fast
  - safe
  - reliable
config:
  debug: true
  max_depth: 100
";

    let yaml = Yaml::new();
    let parsed = yaml.load_str(yaml_content)?;

    println!("Parsed YAML: {parsed:#?}");

    // Test dumping
    let mut output = Vec::new();
    yaml.dump(&parsed, &mut output)?;
    let output_str = String::from_utf8_lossy(&output);
    println!("Dumped YAML:\n{output_str}");

    Ok(())
}
