//! The one shared place for provider API keys, used by every Yana app.
//!
//! Boundary: keys are written, removed and read only here, and no command prints
//! a value. A key is read for two purposes: to start or resume a chat turn with
//! the catalog provider it belongs to, and to fetch that provider's model list
//! (`models.rs`). Both addresses are fixed in this binary. A
//! `custom` provider never gets a stored key, because its address comes from the
//! caller. Nor does a provider reached over plain `http`.
//!
//! Limits, so nobody reads more into this than it gives:
//! - Commands and servers this runtime's agent starts are marked and refused
//!   here, but a command can remove the marker, and other agents' shells and
//!   hooks never set it. The control levels do not approve `security`,
//!   `yana-rt os` or `yana-rt chat` when invoked directly by those names;
//!   matching names can be got around, and project code an approved or
//!   level-approved command runs can reach whatever this user's processes can.
//! - Only the headless chat protocol and the model listing read the store. The
//!   terminal chat, task runner, remote adapter and failover routes still use
//!   environment variables.
//!
//! Storage is the macOS Keychain, service `yana-ai`, account = the provider's key
//! variable name, the convention `platform::macos::secrets` already checks for
//! presence. `set` replaces the item, so it does not keep the access list of an
//! item made earlier by another tool.

use anyhow::{bail, Result};
use std::collections::BTreeSet;
use std::io::{IsTerminal, Read};
use std::path::{Path, PathBuf};

/// Exit status of `set --if-absent` when a key was already there.
const ALREADY_STORED_EXIT: i32 = 3;
const REMOVAL_RECORD_NAME: &str = "credentials-removed.json";

/// Keychain service label, shared with the presence check.
#[cfg(target_os = "macos")]
const SERVICE: &str = "yana-ai";
/// Longer than any provider key; anything bigger is a mistake or an attack.
const MAX_KEY_BYTES: usize = 8 * 1024;
/// Providers whose fixed address is plain `http`: any program holding that local
/// port would receive the key, so theirs is sent only when a request carries it.
const PLAIN_HTTP_PROVIDERS: &[&str] = &["9router"];
/// Switches the stored keys off, for tests and scripts that must not use them.
const OPT_OUT_ENV: &str = "YANA_NO_KEY_STORE";
/// How long a Keychain read may wait, which includes a person answering the
/// macOS permission dialog.
#[cfg(target_os = "macos")]
const READ_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(60);

pub(crate) trait KeyStore {
    fn get(&self, account: &str) -> Result<Option<String>>;
    fn set(&self, account: &str, value: &str) -> Result<()>;
    /// Saves only when the account has no item, in one step. False when it has one.
    fn add(&self, account: &str, value: &str) -> Result<bool>;
    /// Removing a key that is not there is not an error.
    fn remove(&self, account: &str) -> Result<()>;
}

pub(crate) struct NativeKeyStore;

#[cfg(target_os = "macos")]
impl KeyStore for NativeKeyStore {
    fn get(&self, account: &str) -> Result<Option<String>> {
        let (sender, receiver) = std::sync::mpsc::channel();
        let wanted = account.to_string();
        // The call blocks while a permission dialog is open; the thread is left
        // behind on timeout and ends with the process.
        std::thread::spawn(move || {
            let _ = sender.send(security_framework::passwords::get_generic_password(SERVICE, &wanted));
        });
        let Ok(read) = receiver.recv_timeout(READ_TIMEOUT) else {
            bail!("the Keychain did not answer for {account}; allow access in the macOS dialog and try again");
        };
        match read {
            Ok(bytes) => match String::from_utf8(bytes) {
                Ok(value) => Ok(Some(value)),
                // The decode error would carry the bytes, so it is not passed on.
                Err(_) => bail!("the stored key for {account} is not text"),
            },
            Err(error) if error.code() == ITEM_NOT_FOUND => Ok(None),
            Err(error) => bail!("the Keychain refused to read {account} (status {})", error.code()),
        }
    }

    fn set(&self, account: &str, value: &str) -> Result<()> {
        security_framework::passwords::set_generic_password(SERVICE, account, value.as_bytes())
            .map_err(|error| anyhow::anyhow!("the Keychain refused to save {account} (status {})", error.code()))
    }

    fn add(&self, account: &str, value: &str) -> Result<bool> {
        let keychain = security_framework::os::macos::keychain::SecKeychain::default()
            .map_err(|error| anyhow::anyhow!("the Keychain could not be opened (status {})", error.code()))?;
        match keychain.add_generic_password(SERVICE, account, value.as_bytes()) {
            Ok(()) => Ok(true),
            Err(error) if error.code() == DUPLICATE_ITEM => Ok(false),
            Err(error) => bail!("the Keychain refused to save {account} (status {})", error.code()),
        }
    }

    fn remove(&self, account: &str) -> Result<()> {
        match security_framework::passwords::delete_generic_password(SERVICE, account) {
            Ok(()) => Ok(()),
            Err(error) if error.code() == ITEM_NOT_FOUND => Ok(()),
            Err(error) => bail!("the Keychain refused to remove {account} (status {})", error.code()),
        }
    }
}

/// `errSecItemNotFound`.
#[cfg(target_os = "macos")]
const ITEM_NOT_FOUND: i32 = -25300;
/// `errSecDuplicateItem`.
#[cfg(target_os = "macos")]
const DUPLICATE_ITEM: i32 = -25299;

#[cfg(not(target_os = "macos"))]
impl KeyStore for NativeKeyStore {
    fn get(&self, _account: &str) -> Result<Option<String>> {
        Ok(None)
    }
    fn set(&self, _account: &str, _value: &str) -> Result<()> {
        bail!("storing keys is only available on macOS; elsewhere the app sends the key with each request")
    }
    fn add(&self, _account: &str, _value: &str) -> Result<bool> {
        bail!("storing keys is only available on macOS; elsewhere the app sends the key with each request")
    }
    fn remove(&self, _account: &str) -> Result<()> {
        bail!("storing keys is only available on macOS")
    }
}

/// The Keychain account for a catalog provider that takes a key. Aliases resolve
/// to the same account. Unknown names, `custom` and keyless providers are refused.
fn account_for(provider: &str) -> Result<String> {
    // The name is not repeated: someone may have typed a key where the name goes.
    let Ok(selected) = crate::model::catalog::try_select_provider(provider) else {
        bail!("unknown provider; `yana-rt os credential status` lists the provider IDs");
    };
    if PLAIN_HTTP_PROVIDERS.contains(&selected.name()) {
        bail!("provider '{}' is reached over plain http, so its key is not stored", selected.name());
    }
    if !selected.requires_key() || selected.env_var().is_empty() {
        bail!("provider '{}' takes no key", selected.name());
    }
    Ok(selected.env_var().to_string())
}

fn inside_agent_command() -> bool {
    std::env::var_os(crate::capability::command::AGENT_CHILD_ENV).is_some()
}

/// The key a turn starts with: the one in the request, else the stored one.
pub(crate) fn key_for_turn(provider: &str, requested: Option<String>) -> Option<String> {
    let blocked = inside_agent_command() || std::env::var_os(OPT_OUT_ENV).is_some();
    resolve(provider, requested, blocked, &NativeKeyStore)
}

fn resolve(provider: &str, requested: Option<String>, blocked: bool, store: &dyn KeyStore) -> Option<String> {
    if let Some(key) = requested.filter(|value| !value.is_empty()) {
        return Some(key);
    }
    if blocked {
        return None;
    }
    // `custom`, keyless and unknown providers have no account, so nothing is read.
    let account = account_for(provider).ok()?;
    match store.get(&account) {
        // An item made by another tool may hold a stray newline or other text.
        Ok(found) => found.and_then(|value| clean_key(&value).map_err(|why| eprintln!("[keys] {account}: {why}")).ok()),
        Err(error) => {
            eprintln!("[keys] {error}");
            None
        }
    }
}

/// Reads one key, trimmed of surrounding blank space and the final newline.
fn read_key(input: &mut dyn Read) -> Result<String> {
    let mut bytes = Vec::new();
    input.take(MAX_KEY_BYTES as u64 + 1).read_to_end(&mut bytes)?;
    if bytes.len() > MAX_KEY_BYTES {
        bail!("the key is longer than {MAX_KEY_BYTES} bytes");
    }
    let Ok(text) = String::from_utf8(bytes) else {
        bail!("the key is not text");
    };
    clean_key(&text)
}

/// A key as it can be sent in a header: printable ASCII with no spaces.
fn clean_key(text: &str) -> Result<String> {
    let key = text.trim();
    if key.is_empty() {
        bail!("no key was given");
    }
    if !key.chars().all(|c| c.is_ascii_graphic()) {
        bail!("a key can only hold printable ASCII characters, with no spaces or line breaks");
    }
    Ok(key.to_string())
}

fn save(provider: &str, from_agent: bool, input: &mut dyn Read, store: &dyn KeyStore) -> Result<String> {
    refuse_agent(from_agent)?;
    let account = account_for(provider)?;
    let key = read_key(input)?;
    // Replaced, not updated: an update would keep an older item's access list.
    store.remove(&account)?;
    store.set(&account, &key).map_err(|error| error.context("the earlier key was removed, so enter the key again"))?;
    Ok(account)
}

/// As `save`, but an existing key is left as it is: an app moving its older copy
/// must not replace one saved since. The flag says whether the key was saved.
fn save_if_absent(provider: &str, from_agent: bool, input: &mut dyn Read, store: &dyn KeyStore) -> Result<(String, bool)> {
    refuse_agent(from_agent)?;
    let account = account_for(provider)?;
    let key = read_key(input)?;
    let added = store.add(&account, &key)?;
    Ok((account, added))
}

/// Where removals are remembered, so an app can ignore a copy it kept of a key
/// the person has since removed. It holds account names only, never values.
fn removal_record() -> Option<PathBuf> {
    std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".yana").join(REMOVAL_RECORD_NAME))
}

/// An unreadable or damaged record reads as empty.
pub(crate) fn removed_accounts(file: &Path) -> BTreeSet<String> {
    std::fs::read(file).ok().and_then(|bytes| serde_json::from_slice(&bytes).ok()).unwrap_or_default()
}

pub(crate) fn recorded_removals() -> BTreeSet<String> {
    removal_record().map(|file| removed_accounts(&file)).unwrap_or_default()
}

/// Adds the account to the record, or takes it out. Replaced in one rename.
fn record_removal(file: &Path, account: &str, removed: bool) -> Result<()> {
    let mut accounts = removed_accounts(file);
    let changed = if removed { accounts.insert(account.to_string()) } else { accounts.remove(account) };
    if !changed {
        return Ok(());
    }
    let parent = file.parent().ok_or_else(|| anyhow::anyhow!("the removal record has no directory"))?;
    std::fs::create_dir_all(parent)?;
    let staged = file.with_extension(format!("tmp.{}", std::process::id()));
    std::fs::write(&staged, serde_json::to_vec(&accounts)?)?;
    std::fs::rename(&staged, file)?;
    Ok(())
}

fn note(account: &str, removed: bool) -> Result<()> {
    match removal_record() {
        Some(file) => record_removal(&file, account, removed),
        None => bail!("HOME is not set"),
    }
}

fn forget(provider: &str, from_agent: bool, store: &dyn KeyStore) -> Result<String> {
    refuse_agent(from_agent)?;
    let account = account_for(provider)?;
    store.remove(&account)?;
    Ok(account)
}

fn refuse_agent(from_agent: bool) -> Result<()> {
    if from_agent {
        bail!("this was started by an agent's command; a person manages keys in a Yana app or their own terminal");
    }
    Ok(())
}

fn refuse_extra(extra: &[String]) -> Result<()> {
    if !extra.is_empty() {
        bail!("only the provider ID goes on the command line; a key is read from a pipe");
    }
    Ok(())
}

/// `yana-rt os credential set [--if-absent] <provider>`. The key comes from a pipe, never from an
/// argument (visible to other processes) or a terminal (it would be echoed).
pub(crate) fn cmd_set(provider: &str, if_absent: bool, extra: &[String]) -> Result<()> {
    refuse_extra(extra)?;
    let stdin = std::io::stdin();
    if stdin.is_terminal() {
        bail!("pipe the key into this command, for example from a password manager; typing it here would show it");
    }
    let (input, from_agent) = (&mut stdin.lock(), inside_agent_command());
    let account = if if_absent {
        let (account, added) = save_if_absent(provider, from_agent, input, &NativeKeyStore)?;
        if !added {
            eprintln!("A key is already stored for {provider} ({account}); it was left as it is.");
            std::process::exit(ALREADY_STORED_EXIT);
        }
        account
    } else {
        save(provider, from_agent, input, &NativeKeyStore)?
    };
    note(&account, false).map_err(|error| error.context("the key was saved, but the removal record was not updated"))?;
    println!("Saved the key for {provider} ({account}).");
    Ok(())
}

/// `yana-rt os credential remove <provider>`.
pub(crate) fn cmd_remove(provider: &str, extra: &[String]) -> Result<()> {
    refuse_extra(extra)?;
    let account = forget(provider, inside_agent_command(), &NativeKeyStore)?;
    note(&account, true).map_err(|error| error.context("the key was removed, but the removal was not recorded"))?;
    println!("No key is stored for {provider} ({account}) now.");
    Ok(())
}

pub(crate) mod models;
#[cfg(test)]
mod tests;
