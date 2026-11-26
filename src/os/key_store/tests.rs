use super::*;
use std::cell::RefCell;
use std::collections::HashMap;

/// A store in memory: no test touches the real Keychain.
#[derive(Default)]
struct Fake {
    items: RefCell<HashMap<String, String>>,
    reads: RefCell<Vec<String>>,
    changes: RefCell<Vec<String>>,
    broken: bool,
}

impl Fake {
    fn with(account: &str, value: &str) -> Self {
        let fake = Fake::default();
        fake.items.borrow_mut().insert(account.into(), value.into());
        fake
    }
}

impl KeyStore for Fake {
    fn get(&self, account: &str) -> Result<Option<String>> {
        self.reads.borrow_mut().push(account.into());
        if self.broken {
            bail!("locked");
        }
        Ok(self.items.borrow().get(account).cloned())
    }
    fn set(&self, account: &str, value: &str) -> Result<()> {
        self.changes.borrow_mut().push(format!("set {account}"));
        self.items.borrow_mut().insert(account.into(), value.into());
        Ok(())
    }
    fn remove(&self, account: &str) -> Result<()> {
        self.changes.borrow_mut().push(format!("remove {account}"));
        self.items.borrow_mut().remove(account);
        Ok(())
    }
}

fn saved(provider: &str, text: &[u8], store: &Fake) -> Result<String> {
    save(provider, false, &mut &text[..], store)
}

#[test]
fn a_key_in_the_request_wins_and_nothing_is_read() {
    let store = Fake::with("OPENAI_API_KEY", "stored");
    assert_eq!(resolve("openai", Some("given".into()), false, &store).as_deref(), Some("given"));
    assert!(store.reads.borrow().is_empty());
}

#[test]
fn the_stored_key_is_used_when_the_request_has_none() {
    let store = Fake::with("OPENAI_API_KEY", "stored");
    assert_eq!(resolve("openai", None, false, &store).as_deref(), Some("stored"));
    assert_eq!(resolve("openai", Some(String::new()), false, &store).as_deref(), Some("stored"));
}

#[test]
fn an_alias_reads_the_same_account() {
    let store = Fake::with("ANTHROPIC_API_KEY", "stored");
    assert_eq!(resolve("claude", None, false, &store).as_deref(), Some("stored"));
}

#[test]
fn custom_keyless_and_unknown_providers_read_nothing() {
    let store = Fake::with("OPENAI_API_KEY", "stored");
    for name in ["custom", "ollama", "vllm", "nonexistent", "", "OPENAI_API_KEY", "../openai", "openai\0"] {
        assert_eq!(resolve(name, None, false, &store), None, "{name:?}");
    }
    assert_eq!(resolve(&"a".repeat(65_536), None, false, &store), None);
    assert!(store.reads.borrow().is_empty());
}

#[test]
fn an_agent_command_gets_no_stored_key() {
    let store = Fake::with("OPENAI_API_KEY", "stored");
    assert_eq!(resolve("openai", None, true, &store), None);
    assert!(store.reads.borrow().is_empty());
}

#[test]
fn a_store_that_cannot_be_read_gives_no_key() {
    let store = Fake { broken: true, ..Fake::default() };
    assert_eq!(resolve("openai", None, false, &store), None);
}

#[test]
fn saving_trims_the_final_newline_and_removing_is_repeatable() {
    let store = Fake::default();
    assert_eq!(saved("groq", b"  gsk-test\r\n", &store).unwrap(), "GROQ_API_KEY");
    assert_eq!(store.items.borrow()["GROQ_API_KEY"], "gsk-test");
    forget("groq", false, &store).unwrap();
    forget("groq", false, &store).unwrap();
    assert!(store.items.borrow().is_empty());
}

#[test]
fn bad_keys_are_refused_without_being_stored() {
    let store = Fake::default();
    let long = vec![b'a'; MAX_KEY_BYTES + 1];
    for bad in [&b""[..], b"\n", b"two words", b"line\nbreak", b"nul\0byte", b"\xff\xfe", &long] {
        assert!(saved("openai", bad, &store).is_err());
    }
    assert!(store.items.borrow().is_empty());
    assert!(saved("openai", &vec![b'a'; MAX_KEY_BYTES], &store).is_ok());
}

#[test]
fn an_error_never_repeats_the_key() {
    let error = saved("openai", b"sk-secret value", &Fake::default()).unwrap_err().to_string();
    assert!(!error.contains("sk-secret"), "{error}");
}

#[test]
fn an_agent_command_cannot_change_keys() {
    let store = Fake::with("OPENAI_API_KEY", "stored");
    assert!(save("openai", true, &mut &b"theirs"[..], &store).is_err());
    assert!(forget("openai", true, &store).is_err());
    assert_eq!(store.items.borrow()["OPENAI_API_KEY"], "stored");
}

#[test]
fn a_plain_http_provider_has_no_stored_key() {
    let store = Fake::with("NINE_ROUTER_API_KEY", "stored");
    assert_eq!(resolve("9router", None, false, &store), None);
    assert!(store.reads.borrow().is_empty());
    assert!(saved("9router", b"key", &store).is_err());
}

#[test]
fn text_typed_instead_of_a_provider_is_not_repeated() {
    // The commands themselves use the real Keychain, so only their guard is called here.
    for error in [
        saved("sk-typed-by-mistake", b"key", &Fake::default()).unwrap_err(),
        refuse_extra(&["sk-typed-by-mistake".to_string()]).unwrap_err(),
    ] {
        assert!(!error.to_string().contains("sk-typed"), "{error}");
    }
}

#[test]
fn saving_replaces_the_old_item_instead_of_updating_it() {
    let store = Fake::with("OPENAI_API_KEY", "old");
    saved("openai", b"new", &store).unwrap();
    assert_eq!(*store.changes.borrow(), ["remove OPENAI_API_KEY", "set OPENAI_API_KEY"]);
    store.changes.borrow_mut().clear();
    assert!(saved("openai", b"two words", &store).is_err());
    assert!(store.changes.borrow().is_empty(), "a refused key leaves the old one alone");
}

#[test]
fn a_stored_value_that_is_not_a_key_is_not_sent() {
    for bad in ["", "two words", "\u{feff}sk", "sk\u{200b}x", "khóa"] {
        assert_eq!(resolve("openai", None, false, &Fake::with("OPENAI_API_KEY", bad)), None, "{bad:?}");
    }
    assert_eq!(resolve("openai", None, false, &Fake::with("OPENAI_API_KEY", "sk-x\n")).as_deref(), Some("sk-x"));
}

#[test]
fn custom_and_keyless_providers_cannot_be_saved() {
    for name in ["custom", "ollama", "nonexistent"] {
        assert!(saved(name, b"key", &Fake::default()).is_err(), "{name}");
    }
}

/// Writes one made-up item to this user's real Keychain and removes it again.
/// Run on purpose: cargo test real_keychain -- --ignored
#[cfg(target_os = "macos")]
#[test]
#[ignore = "touches the real Keychain"]
fn real_keychain_round_trip() {
    let account = "YANA_SELFTEST_ACCOUNT";
    let store = NativeKeyStore;
    store.remove(account).unwrap();
    assert_eq!(store.get(account).unwrap(), None);
    store.set(account, "made-up-value").unwrap();
    assert_eq!(store.get(account).unwrap().as_deref(), Some("made-up-value"));
    store.set(account, "replaced").unwrap();
    assert_eq!(store.get(account).unwrap().as_deref(), Some("replaced"));
    store.remove(account).unwrap();
    assert_eq!(store.get(account).unwrap(), None);
}
