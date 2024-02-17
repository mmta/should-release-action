use version_compare::{ compare_to, Cmp };
use serde_derive::Deserialize;
use std::process::Command;
use regex::Regex;
use anyhow::{ Result, Context, anyhow };

// these should match the input and output variable names in the action.yml
const INPUT_FILE_VAR: &str = "INPUT_FILE_PATH";
const INPUT_TAG_VAR: &str = "INPUT_RELEASE_TAG";
const OUTPUT_VERSION_VAR: &str = "current_version";
const OUTPUT_SHOULD_RELEASE_VAR: &str = "should_release";

const DEFAULT_TAG_VAR: &str = "0";

const VERSION_FILE: &str = "version.txt";
const PACKAGE_JSON: &str = "package.json";
const CARGO_TOML: &str = "Cargo.toml";

#[derive(Deserialize)]
struct Config {
    package: Option<Package>,
    workspace: Option<Workspace>,
}

#[derive(Deserialize, Default)]
struct Workspace {
    package: Package,
}

#[derive(Deserialize, Default)]
struct Package {
    version: String,
}

fn set_github_output(key: &str, value: &str) -> Result<()> {
    let cmd_text = format!("echo {}={} >> $GITHUB_OUTPUT", key, value);
    println!("running: {}", cmd_text);
    Command::new("/usr/bin/bash").arg("-c").arg(cmd_text).output()?;
    Ok(())
}

fn parse_env(env_var: &str) -> Result<String, std::env::VarError> {
    match std::env::var(env_var) {
        Ok(v) => {
            println!("found input environment variable {}: {}", env_var, v);
            Ok(v)
        }
        Err(e) => {
            println!("warning: unable to read input {}, reason: {}", env_var, e);
            Err(e)
        }
    }
}
fn parse_input_tag(env_var: &str) -> String {
    let input_var = match parse_env(env_var) {
        Ok(v) => {
            if v.is_empty() { DEFAULT_TAG_VAR.to_string() } else { v }
        }
        Err(_) => DEFAULT_TAG_VAR.to_string(),
    };
    let re = Regex::new(r"^[^0-9]*").unwrap();
    re.replace(&input_var, "").to_string()
}

fn parse_version(content: String) -> Result<(String, String)> {
    let mut file_type = CARGO_TOML;
    let version = if let Ok(config) = toml::from_str::<Config>(&content) {
        // for Cargo.toml
        if let Some(workspace) = config.workspace {
            workspace.package.version
        } else {
            let v = config.package.ok_or(
                // will not happen, deserialize will fail if there is no package.version
                anyhow!(
                    "cannot find package.version or workspace.package.version in the input file"
                )
            )?;
            v.version
        }
    } else if let Ok(package) = serde_json::from_str::<Package>(&content) {
        // for package.json
        file_type = PACKAGE_JSON;
        package.version
    } else {
        // for version.txt
        file_type = VERSION_FILE;
        content
    };
    let v = version_compare::Version
        ::from(&version)
        .ok_or(anyhow!("cannot parse semver version from {}", version))?;

    Ok((v.as_str().trim().to_owned(), file_type.to_owned()))
}

fn check_should_release(version_in_file: &str, latest_tag: &str) -> bool {
    compare_to(version_in_file, latest_tag, Cmp::Gt).unwrap_or_default()
}

fn main() -> Result<()> {
    let path = parse_env(INPUT_FILE_VAR)?;
    println!(
        "Provided configuration:\n\nInput file: {}\nInput variable name: {} (defaults to: {})\nOutput variable names: {}, {}\n",
        &path,
        INPUT_TAG_VAR,
        DEFAULT_TAG_VAR,
        OUTPUT_VERSION_VAR,
        OUTPUT_SHOULD_RELEASE_VAR
    );

    println!("Execution logs:\n");

    let input_tag = parse_input_tag(INPUT_TAG_VAR);
    println!("\nusing {}: {}\n", INPUT_TAG_VAR, input_tag);

    let content = std::fs::read_to_string(&path).context("cannot read content of the input file")?;

    let (version, file_type) = parse_version(content)?;

    println!(
        "using {}: {} (read from {} treated as a {} file)",
        OUTPUT_VERSION_VAR,
        version,
        path,
        file_type
    );

    let should_release = check_should_release(&version, &input_tag);

    let n = input_tag.parse().unwrap_or(0);
    let print = if n > 9000 { input_tag + " (it's over 9000!!)" } else { input_tag };

    println!(
        "\nthe result of comparing {} to {}: {} = {}\n",
        print,
        version,
        OUTPUT_SHOULD_RELEASE_VAR,
        should_release
    );

    for v in [
        (OUTPUT_VERSION_VAR, version.clone()),
        (OUTPUT_SHOULD_RELEASE_VAR, should_release.to_string()),
    ] {
        println!("setting Github output: {} = {}", v.0, v.1);
        set_github_output(v.0, &v.1)?;
    }
    Ok(())
}

#[cfg(test)]
mod test {
    use std::{ env, io::Write };
    use super::*;
    #[test]
    fn test_parse_input_tag() {
        for (input, expected) in [
            ("v1.2.3", "1.2.3"),
            ("1.2.3", "1.2.3"),
            ("package-1.2.3", "1.2.3"),
            ("name_1.2.3", "1.2.3"),
        ] {
            env::set_var(INPUT_TAG_VAR, input);
            assert_eq!(parse_input_tag(INPUT_TAG_VAR), expected);
        }
        for (input, expected) in [
            ("1.2.3.", "1.2.3"),
            ("1.2.3-alpha", "1.2.3"),
            ("1.2.3-beta", "1.2.3"),
            ("1.2.3-rc", "1.2.3"),
        ] {
            env::set_var(INPUT_TAG_VAR, input);
            assert_ne!(parse_input_tag(INPUT_TAG_VAR), expected);
        }
        assert_eq!(parse_input_tag("NON_EXISTING_VAR"), DEFAULT_TAG_VAR);

        env::set_var("EMPTY_VAR", "");
        assert_eq!(parse_input_tag("EMPTY_VAR"), DEFAULT_TAG_VAR);
    }
    #[test]
    fn test_parse_cargo_toml() {
        let input = r#"
        [package]
        name = "test"
        version = "1.2.3"
        "#;
        let expected = "1.2.3";
        let (ver, file_type) = parse_version(input.to_string()).unwrap();
        assert_eq!(ver, expected);
        assert!(file_type == CARGO_TOML);
        let input =
            r#"
        [workspace]
        resolver = "2"

        [workspace.package]
        version = "1.2.3"
        "#;
        let expected = "1.2.3";
        let (ver, _) = parse_version(input.to_string()).unwrap();
        assert_eq!(ver, expected);
        let input =
            r#"
        [workspace]
        resolver = "2"
        [workspace.package]
        foo = "bar"
        "#;
        let (_, file_type) = parse_version(input.to_string()).unwrap();
        // deserialize should fail when there is no version field, and the catch-all type will be VERSION_FILE
        assert_eq!(file_type, VERSION_FILE);
    }
    #[test]
    fn test_parse_package_json() {
        let input = r#"
        { 
            "version": "1.2.3"
        }"#;
        let expected = "1.2.3";
        let (ver, _) = parse_version(input.to_string()).unwrap();
        assert_eq!(ver, expected);
    }
    #[test]
    fn test_parse_version_txt() {
        let input = r#"
        1.2.3
        "#;
        let expected = "1.2.3";
        assert_eq!(parse_version(input.to_string()).unwrap().0, expected);
    }
    #[test]
    fn test_check_should_release() {
        assert!(check_should_release("1.2.3", "1.2.2"));
        assert!(check_should_release("1.2.3", "1.2.2-beta"));
        assert!(check_should_release("1.2.3", "ver-1.2.2"));
        assert!(check_should_release("1.2.3", "v1.2.2"));
        assert!(check_should_release("v1.2.3", "1.2.2"));
        assert!(check_should_release("1.2.3", ""));

        assert!(!check_should_release("1.2.3", "1.2.3"));
        assert!(!check_should_release("1.2.3", "1.2.4"));
        assert!(!check_should_release("1.2.3", "v1.2.4"));
        assert!(!check_should_release("v1.2.3", "1.2.4"));
        assert!(check_should_release("1.2.3", ""));
    }
    #[test]
    fn test_github_output() {
        let file = tempfile::NamedTempFile::new().unwrap();
        let path = file.into_temp_path();
        let p = path.to_string_lossy().into_owned();
        env::set_var("GITHUB_OUTPUT", p.clone());

        set_github_output("test", "value").unwrap();
        let content = std::fs::read_to_string(p.clone()).unwrap();
        assert!(content.contains("test=value"));

        set_github_output("again", "once").unwrap();
        let content = std::fs::read_to_string(p).unwrap();
        assert!(content.contains("again=once"));
    }
    #[test]
    fn test_main() {
        env::set_var(INPUT_TAG_VAR, "v1.2.3");
        let res = main();
        assert!(res.is_err());

        let mut file = tempfile::NamedTempFile::new().unwrap();
        file.write_all(b"1.2.3").unwrap();

        let path = file.into_temp_path();
        let p = path.to_string_lossy().into_owned();

        env::set_var(INPUT_FILE_VAR, p);
        let res = main();
        assert!(res.is_ok());
    }
}
