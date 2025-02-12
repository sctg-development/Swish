/// Hey there!
/// As you can see, Im a real noob in Rust and dev in general, so please be kind with me.
/// I hope someone with no skill issues could refactor the wole code base and make it readable and maintainable.
/// Sorry for the mess x) at least it seems to work for now \o/
mod api;
mod ca_bundle;
mod errors;
mod swissfiles;
use ca_bundle::get_cert_bundle;
use log::LevelFilter;
use simple_logger::SimpleLogger;
use std::path::PathBuf;
use std::{env, os};
use swissfiles::uploadparameters::UploadParameters;
use swissfiles::Swissfiles;

use clap::Parser;
use errors::SwishError;
use regex::Regex;

#[derive(clap::Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// could be a file or a folder or a link
    file: String,

    /// Sets the password for the file(s) downloaded / uploaded
    #[arg(short, long, value_name = "password")]
    password: Option<String>,

    /// Define the password for derivating the AES encryption key for the file(s) uploaded (will be uploaded as a 7z encrypted)
    #[arg(short = 'a', long, value_name = "aes")]
    aes_password: Option<String>,

    /// Define the message for the file(s) uploaded
    #[arg(short, long, value_name = "Hello World")]
    message: Option<String>,

    /// Define the max number of downloads for the file(s) uploaded
    #[arg(short, long, value_name = "250", value_parser = validate_number_download)]
    number_download: Option<String>,

    /// Define the number of days the file(s) will be available for download
    #[arg(short, long, value_name = "30", value_parser = validate_duration)]
    duration: Option<String>,

    /// Define an output directory for the downloaded files
    #[arg(short, long, value_name = "output")]
    output: Option<String>,

    /// Use insecure tls connection
    #[arg(short, long, value_name = "file")]
    insecure: bool,

    /// Show the bundled ca root
    #[arg(short, long)]
    ca_root: bool,

    #[arg(short, long, value_name = "file", help = env!("HELP_CA_ROOT") )]
    system_ca_bundle: bool,

    /// Enable verbose mode
    #[arg(short, long)]
    verbose: bool,

    /// Enable curl verbose mode
    #[arg(short = 'w', long)]
    curl_verbose: bool,
}

fn main() -> Result<(), SwishError> {
    let cli = Cli::parse();

    // Initialize logger
    let mut logger = SimpleLogger::new();
    if cli.verbose {
        logger = logger.with_level(LevelFilter::Debug);
    } else {
        logger = logger.with_level(LevelFilter::Info);
    }

    if cli.curl_verbose {
        env::set_var("CURL_VERBOSE", "1");
    }
    if cli.ca_root {
        println!("{}", get_cert_bundle());
        return Ok(());
    }

    logger.init().unwrap();

    let arg = cli.file;

    let insecure = cli.insecure;
    // Set the insecure flag as an environment variable
    if insecure {
        env::set_var("CURL_CA_BUNDLE", "");
        env::set_var("CURL_INSECURE", "1");
    }

    let system_ca_bundle = cli.system_ca_bundle;
    // Set the system CA bundle flag as an environment variable
    if system_ca_bundle {
        env::set_var("CURL_USE_INTERNAL_CA_BUNDLE", "0");
    } else {
        env::set_var("CURL_USE_INTERNAL_CA_BUNDLE", "1");
    }

    //check if the arg is a link
    if is_swisstransfer_link(&arg) {
        //Construct the swissfiles from the link
        let swissfiles = Swissfiles::new_remotefiles(&arg, cli.password.as_deref())?;

        //Download the files
        swissfiles.download(cli.output.map(PathBuf::from).as_ref())?;

        return Ok(());
    }
    //check if the arg is a path
    if path_exists(&arg) {
        let mut path = PathBuf::from(&arg);

        if let Some(aes_key) = cli.aes_password.clone() {
            // Create a temporary 7z archive using the standard os temp directory and suffixing the file name with .7z
            let original_extension = path.extension().unwrap_or_default().to_str().unwrap_or("");
            let mut extension = String::new();
            extension = if original_extension.len() == 0 {
                format!("{}7z",original_extension)
            }else{
                format!("{}.7z",original_extension)
            };
            let compressed_file = env::temp_dir()
                .join(&arg)
                .with_extension(std::ffi::OsStr::new(&extension));
            sevenz_rust::compress_to_path_encrypted(path, compressed_file.clone(), aes_key.as_str().into())
                .expect("compress ok");
            path = compressed_file.clone();
        }

        let mut params = UploadParameters::default();

        if let Some(password) = cli.password {
            params.password = password;
        }

        if let Some(message) = cli.message {
            params.message = message;
        }

        if let Some(number_download) = cli.number_download {
            params.number_of_download = number_download.parse().unwrap();
        }

        if let Some(duration) = cli.duration {
            params.duration = duration.parse().unwrap();
        }

        let local_files = Swissfiles::new_localfiles(path.clone(), &params)?;
        let download_link = local_files.upload()?;
        println!("Download link: {}", download_link);

        if cli.aes_password.is_some() {
            // Delete the temporary 7z archive
            std::fs::remove_file(path).expect("remove ok");
        }

        return Ok(());
    }

    Err(SwishError::InvalidUrl { url: arg })
}

fn is_swisstransfer_link(link: &str) -> bool {
    let re = Regex::new(r"^https://www\.swisstransfer\.com/d/[a-f0-9]{8}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{12}$").unwrap();
    re.is_match(link)
}

fn path_exists(path: &str) -> bool {
    //str is a file or folder
    PathBuf::from(path).exists()
}

fn validate_number_download(val: &str) -> Result<String, String> {
    let number = val.parse::<u16>().map_err(|_| "Must be a valid number")?;
    if number < 1 || number > 250 {
        Err(String::from(
            "Number of downloads must be between 1 and 250",
        ))
    } else {
        Ok(val.to_string())
    }
}

fn validate_duration(val: &str) -> Result<String, String> {
    let number = val.parse::<u32>().map_err(|_| "Must be a valid number")?;
    if [1, 7, 15, 30].contains(&number) {
        Ok(val.to_string())
    } else {
        Err(String::from("Duration must be 1, 7, 15 or 30"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_swisstransfer_link() {
        let link = "https://www.swisstransfer.com/d/8b3b3b3b-3b3b-3b3b-3b3b-3b3b3b3b3b3b";
        assert_eq!(is_swisstransfer_link(link), true);
        let link = "http://www.swisstransfer.com/d/8b3b3b3b-3b3b-3b3b-3b3b-3b3b3b3b3b3b/";
        assert_eq!(is_swisstransfer_link(link), false);
        let link = "https://www.swisstransfer.ch/d/8b3b3b3b-3b3b-3b3b-3b3b-3b3b3b3b3b3b/";
        assert_eq!(is_swisstransfer_link(link), false);
        let link = "www.swisstransfer.com/d/8b3b3b3b-3b3b-3b3b-3b3b-3b3b3b3b3b3b";
        assert_eq!(is_swisstransfer_link(link), false);
        let link = "https://www.swisstransfer.com/8b3b3b3b-3b3b-3b3b-3b3b-3b3b3b3b3b3b";
        assert_eq!(is_swisstransfer_link(link), false);
    }

    #[test]
    fn test_path_exists() {
        let path = "Cargo.toml";
        assert_eq!(path_exists(path), true);
        let path = "Cargo.toml2";
        assert_eq!(path_exists(path), false);
    }

    #[test]
    fn test_validate_number_download() {
        let number = "250";
        assert_eq!(validate_number_download(number), Ok(number.to_string()));
        let number = "251";
        assert_eq!(
            validate_number_download(number),
            Err(String::from(
                "Number of downloads must be between 1 and 250"
            ))
        );
        let number = "0";
        assert_eq!(
            validate_number_download(number),
            Err(String::from(
                "Number of downloads must be between 1 and 250"
            ))
        );
        let number = "a";
        assert_eq!(
            validate_number_download(number),
            Err(String::from("Must be a valid number"))
        );
    }

    #[test]
    fn test_validate_duration() {
        let duration = "30";
        assert_eq!(validate_duration(duration), Ok(duration.to_string()));
        let duration = "31";
        assert_eq!(
            validate_duration(duration),
            Err(String::from("Duration must be 1, 7, 15 or 30"))
        );
        let duration = "0";
        assert_eq!(
            validate_duration(duration),
            Err(String::from("Duration must be 1, 7, 15 or 30"))
        );
        let duration = "a";
        assert_eq!(
            validate_duration(duration),
            Err(String::from("Must be a valid number"))
        );
    }
}
