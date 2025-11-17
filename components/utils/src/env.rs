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

const KNOWN_VALUES_TO_MASK: &[&str] = &[
    // various certs
    "-----BEGIN",
    // openai
    "sk-",
];

pub fn get_masked_env_vars() -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();

    for (key, value) in std::env::vars() {
        if KNOWN_ENV_VAR_TO_MASK
            .iter()
            .any(|k| k.eq_ignore_ascii_case(&key))
            || KNOWN_VALUES_TO_MASK.iter().any(|v| value.contains(*v))
        {
            out.insert(key, "***".to_owned());
            continue;
        }
        out.insert(key, value);
    }

    out
}
