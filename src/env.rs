//! Fns to read variables from the environment more conveniently and help other functions figure
//! out what environment they're running in.

use std::{env, fmt, sync::LazyLock};

use tracing::{debug, warn};

const SECRET_LOG_BLACKLIST: [&str; 0] = [];

pub static ENV_CONFIG: LazyLock<EnvConfig> = LazyLock::new(get_env_config);

fn obfuscate_if_secret(blacklist: &[&str], key: &str, value: &str) -> String {
    if blacklist.contains(&key) {
        let mut last_four = value.to_string();
        last_four.drain(0..value.len().saturating_sub(4));
        format!("****{last_four}")
    } else {
        value.to_string()
    }
}

/// Get an environment variable, encoding found or missing as Option, and panic otherwise.
pub fn get_env_var(key: &str) -> Option<String> {
    let var = match env::var(key) {
        Err(env::VarError::NotPresent) => None,
        Err(e) => panic!("{e}"),
        Ok(var) => Some(var),
    };

    if let Some(ref existing_var) = var {
        let output = obfuscate_if_secret(&SECRET_LOG_BLACKLIST, key, existing_var);
        debug!("env var {key}: {output}");
    } else {
        debug!("env var {key} requested but not found")
    };

    var
}

/// Get an environment variable we can't run without.
pub fn get_env_var_unsafe(key: &str) -> String {
    get_env_var(key).unwrap_or_else(|| panic!("{key} should be in env"))
}

pub fn get_env_bool(key: &str) -> Option<bool> {
    get_env_var(key).map(|var| match var.to_lowercase().as_str() {
        "true" => true,
        "false" => false,
        "t" => true,
        "f" => false,
        "1" => true,
        "0" => false,
        str => panic!("invalid bool value {str} for {key}"),
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Network {
    Mainnet,
    Goerli,
    Holesky,
    Hoodi,
}

impl fmt::Display for Network {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Network::Mainnet => write!(f, "mainnet"),
            Network::Goerli => write!(f, "goerli"),
            Network::Holesky => write!(f, "holesky"),
            Network::Hoodi => write!(f, "hoodi"),
        }
    }
}

pub fn get_network() -> Network {
    let network_str = get_env_var("NETWORK");
    match network_str {
        None => {
            warn!("no NETWORK in env, assuming Mainnet");
            Network::Mainnet
        }
        Some(str) => match str.to_lowercase().as_ref() {
            "mainnet" => Network::Mainnet,
            "goerli" => Network::Goerli,
            "holesky" => Network::Holesky,
            "hoodi" => Network::Hoodi,
            _ => panic!("NETWORK present: {str}, but not one of [mainnet, goerli, holesky, hoodi], panicking!"),
        },
    }
}

#[derive(Debug, Clone)]
pub struct EnvConfig {
    pub beacon_url: String,
    pub bind_public_interface: bool,
    pub execution_node_url: String,
    pub network: Network,
}

fn get_env_config() -> EnvConfig {
    dotenvy::dotenv().ok();

    EnvConfig {
        beacon_url: get_env_var("BEACON_URL").expect("BEACON_URL not set"),
        bind_public_interface: get_env_bool("BIND_PUBLIC_INTERFACE").unwrap_or(true),
        execution_node_url: get_env_var("EXECUTION_NODE_URL").expect("EXECUTION_NODE_URL not set"),
        network: get_network(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic]
    fn test_get_env_var_unsafe_panics() {
        get_env_var_unsafe("DOESNT_EXIST");
    }

    #[test]
    fn test_get_env_var_unsafe() {
        let test_key = "TEST_KEY_UNSAFE";
        let test_value = "my-env-value";
        std::env::set_var(test_key, test_value);
        assert_eq!(get_env_var_unsafe(test_key), test_value);
    }

    #[test]
    fn test_get_env_var_safe_some() {
        let test_key = "TEST_KEY_SAFE_SOME";
        let test_value = "my-env-value";
        std::env::set_var(test_key, test_value);
        assert_eq!(get_env_var(test_key), Some(test_value.to_string()));
    }

    #[test]
    fn test_get_env_var_safe_none() {
        let key = get_env_var("DOESNT_EXIST");
        assert!(key.is_none());
    }

    #[test]
    fn test_get_env_bool_not_there() {
        let flag = get_env_bool("DOESNT_EXIST");
        assert_eq!(flag, None);
    }

    #[test]
    fn test_get_env_bool_true() {
        let test_key = "TEST_KEY_BOOL_TRUE";
        let test_value = "true";
        std::env::set_var(test_key, test_value);
        assert_eq!(get_env_bool(test_key), Some(true));
    }

    #[test]
    fn test_get_env_bool_true_upper() {
        let test_key = "TEST_KEY_BOOL_TRUE2";
        let test_value = "TRUE";
        std::env::set_var(test_key, test_value);
        assert_eq!(get_env_bool(test_key), Some(true));
    }

    #[test]
    fn test_get_env_bool_false() {
        let test_key = "TEST_KEY_BOOL_FALSE";
        let test_value = "false";
        std::env::set_var(test_key, test_value);
        assert_eq!(get_env_bool(test_key), Some(false));
    }

    #[test]
    fn test_obfuscate_if_secret() {
        let secret_key = "SECRET_KEY";
        let blacklist = vec![secret_key];
        assert_eq!(
            obfuscate_if_secret(&blacklist, secret_key, "my_secret_value"),
            "****alue"
        );

        let normal_key = "NORMAL_KEY";
        assert_eq!(
            obfuscate_if_secret(&blacklist, normal_key, "my_normal_value"),
            "my_normal_value"
        );
    }

    #[test]
    fn test_get_network() {
        std::env::set_var("NETWORK", "mainnet");
        assert_eq!(get_network(), Network::Mainnet);

        std::env::set_var("NETWORK", "goerli");
        assert_eq!(get_network(), Network::Goerli);

        std::env::set_var("NETWORK", "Mainnet");
        assert_eq!(get_network(), Network::Mainnet);

        std::env::set_var("NETWORK", "Goerli");
        assert_eq!(get_network(), Network::Goerli);

        std::env::remove_var("NETWORK");
        assert_eq!(get_network(), Network::Mainnet);
    }

    #[test]
    #[ignore = "this test breaks NETWORK for parallel tests"]
    fn test_get_network_panics() {
        std::env::set_var("NETWORK", "invalid_network");
        get_network();
    }
}
