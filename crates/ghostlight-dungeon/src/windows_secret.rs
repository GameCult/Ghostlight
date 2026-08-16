use anyhow::{Context, Result, anyhow};
use std::{fs, path::Path, slice};
use windows::Win32::{
    Foundation::{HLOCAL, LocalFree},
    Security::Cryptography::{CRYPT_INTEGER_BLOB, CryptUnprotectData},
};
use zeroize::Zeroizing;

/// Decrypts a machine-scoped DPAPI blob. DPAPI authenticates the blob and
/// Windows owns the returned allocation; the plaintext is copied once into a
/// zeroizing UTF-8 buffer and the Windows allocation is immediately released.
pub fn unprotect_machine_utf8(path: impl AsRef<Path>) -> Result<String> {
    let mut protected = fs::read(path.as_ref())
        .with_context(|| format!("read DPAPI secret {}", path.as_ref().display()))?;
    if protected.is_empty() {
        return Err(anyhow!("DPAPI secret blob is empty"));
    }
    let input = CRYPT_INTEGER_BLOB {
        cbData: u32::try_from(protected.len()).context("DPAPI blob is too large")?,
        pbData: protected.as_mut_ptr(),
    };
    let mut output = CRYPT_INTEGER_BLOB::default();
    unsafe {
        CryptUnprotectData(&input, None, None, None, None, 0, &mut output)
            .context("CryptUnprotectData failed")?;
    }
    if output.pbData.is_null() || output.cbData == 0 {
        return Err(anyhow!("DPAPI returned no plaintext"));
    }
    let plaintext = unsafe { slice::from_raw_parts(output.pbData, output.cbData as usize) };
    let zeroizing = Zeroizing::new(plaintext.to_vec());
    unsafe {
        let _ = LocalFree(Some(HLOCAL(output.pbData.cast())));
    }
    let value = String::from_utf8(zeroizing.to_vec()).context("DPAPI secret is not UTF-8")?;
    if value.trim().is_empty() {
        return Err(anyhow!("DPAPI secret plaintext is empty"));
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_absent_blob_without_secret_material() {
        assert!(unprotect_machine_utf8("definitely-absent.dpapi").is_err());
    }
}
