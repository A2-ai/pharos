use std::collections::BTreeMap;

const KNOWN_ENV_VAR_TO_MASK: &[&str] = &[
    "GITHUB_TOKEN",
    "GH_TOKEN",
    "GHE_TOKEN",
    "GITHUB_OAUTH",
    "RS_SESSION_SSL_CERT_KEY",
    "RS_SESSION_SSL_CERT",
    "RS_SESSION_RPC_COOKIE",
    "RS_PORT_TOKEN",
    "RS_MONITOR_SHARED_SECRET",
    "RS_SESSION_SERVER_RPC_SECRET",
    "RSTUDIO_SESSION_RSA_PRIVATE_KEY",
    "RSTUDIO_SESSION_RSA_PUBLIC_KEY",
    "RSTUDIO_SIGNING_KEY",
    "POSITRON_LICENSE_KEY",
    "POSITRON_SUPERVISOR_CONNECTION_FILE",
    "AWS_ACCESS_KEY_ID",
    "AWS_SECRET_ACCESS_KEY",
    "AWS_SESSION_TOKEN",
];

const SENSITIVE_NAME_PATTERNS: &[&str] = &[
    "TOKEN",
    "SECRET",
    "PASSWORD",
    "PASSWD",
    "PASSPHRASE",
    "CREDENTIAL",
    "PRIVATE",
    "KEY",
    "AUTH",
    "COOKIE",
];

const KNOWN_VALUES_TO_MASK: &[&str] = &[
    // various certs / private keys
    "-----BEGIN",
    // openai
    "sk-",
];

fn is_sensitive(key: &str, value: &str) -> bool {
    let upper_key = key.to_ascii_uppercase();
    KNOWN_ENV_VAR_TO_MASK
        .iter()
        .any(|k| k.eq_ignore_ascii_case(key))
        || SENSITIVE_NAME_PATTERNS
            .iter()
            .any(|p| upper_key.contains(p))
        || KNOWN_VALUES_TO_MASK.iter().any(|v| value.contains(*v))
}

pub fn get_masked_env_vars() -> BTreeMap<String, String> {
    std::env::vars()
        .map(|(key, value)| {
            if is_sensitive(&key, &value) {
                (key, "***".to_owned())
            } else {
                (key, value)
            }
        })
        .collect()
}
