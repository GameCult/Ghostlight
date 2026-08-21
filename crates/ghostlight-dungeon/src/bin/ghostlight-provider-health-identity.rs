use anyhow::{Result, bail};
use cultnet_rs::{
    GameCultProviderHealthIdentity, enroll_service_identity_at, open_service_identity_at,
};
use std::path::PathBuf;

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let action = args.next().unwrap_or_default();
    let path = args.next().map(PathBuf::from);
    if args.next().is_some()
        || path.is_none()
        || !matches!(action.as_str(), "enroll" | "public-key-hex")
    {
        bail!("Usage: ghostlight-provider-health-identity enroll|public-key-hex <private-store>");
    }
    let path = path.expect("validated above");
    let signer = if action == "enroll" {
        enroll_service_identity_at::<GameCultProviderHealthIdentity>(&path)?
    } else {
        open_service_identity_at::<GameCultProviderHealthIdentity>(&path)?
    };
    println!("{}", hex(&signer.entry().public_key));
    Ok(())
}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(DIGITS[(byte >> 4) as usize] as char);
        output.push(DIGITS[(byte & 0x0f) as usize] as char);
    }
    output
}
