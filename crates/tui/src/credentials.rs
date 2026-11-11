//! Bounded archive-unlock input and owned credentials for one reader request.
//! Secret-bearing types intentionally do not implement `Debug` or `Display`.

use crossterm::event::KeyCode;
use duplicity_core::reader::{
    CryptoCredentials, CryptoReadContext, OpenPgpLimits, OpenPgpPassword, SecretExport,
};
use std::{fs::File, io::Read, path::Path};

pub const PASSPHRASE_LIMIT: usize = 4096;
pub const PATH_INPUT_LIMIT: usize = 4096;
pub const KEY_EXPORT_LIMIT: usize = 16 * 1024 * 1024;

pub enum ReadCredentials {
    Symmetric {
        message_passphrase: OpenPgpPassword,
    },
    PublicKey {
        secret_export: Vec<u8>,
        key_passphrase: OpenPgpPassword,
    },
    Signed {
        secret_export: Vec<u8>,
        key_passphrase: OpenPgpPassword,
        signer_export: Vec<u8>,
        primary_fingerprint: [u8; 20],
    },
}

impl ReadCredentials {
    pub fn with_context<R>(&self, f: impl FnOnce(&CryptoReadContext<'_>) -> R) -> R {
        let crypto_limits = OpenPgpLimits::default();
        let evidence_limits = duplicity_core::reader::EvidenceLimits::default();
        match self {
            Self::Symmetric { message_passphrase } => {
                let credentials = CryptoCredentials {
                    message_passwords: std::slice::from_ref(message_passphrase),
                    ..CryptoCredentials::default()
                };
                let context = CryptoReadContext {
                    credentials,
                    crypto_limits,
                    evidence_limits,
                };
                f(&context)
            }
            Self::PublicKey {
                secret_export,
                key_passphrase,
            } => {
                let exports = [SecretExport {
                    bytes: secret_export,
                    password: Some(key_passphrase),
                }];
                let credentials = CryptoCredentials {
                    secret_exports: &exports,
                    ..CryptoCredentials::default()
                };
                let context = CryptoReadContext {
                    credentials,
                    crypto_limits,
                    evidence_limits,
                };
                f(&context)
            }
            Self::Signed {
                secret_export,
                key_passphrase,
                signer_export,
                primary_fingerprint,
            } => {
                let secret_exports = [SecretExport {
                    bytes: secret_export,
                    password: Some(key_passphrase),
                }];
                let signer_exports = [signer_export.as_slice()];
                let permitted_signers = [*primary_fingerprint];
                let credentials = CryptoCredentials {
                    secret_exports: &secret_exports,
                    signer_exports: &signer_exports,
                    permitted_signers: &permitted_signers,
                    require_signature: true,
                    ..CryptoCredentials::default()
                };
                let context = CryptoReadContext {
                    credentials,
                    crypto_limits,
                    evidence_limits,
                };
                f(&context)
            }
        }
    }
}

enum Profile {
    PublicKey,
    Signed,
}

enum Stage {
    Choose,
    SecretPath {
        profile: Profile,
        input: String,
    },
    KeyPassphrase {
        profile: Profile,
        key: Vec<u8>,
        input: String,
    },
    SignerPath {
        key: Vec<u8>,
        key_passphrase: OpenPgpPassword,
        input: String,
    },
    Fingerprint {
        key: Vec<u8>,
        key_passphrase: OpenPgpPassword,
        signer: Vec<u8>,
        input: String,
    },
    MessagePassphrase {
        input: String,
    },
}

pub enum PromptResult {
    Pending,
    Cancelled,
    Complete(Option<ReadCredentials>),
}

pub struct CredentialPrompt {
    stage: Stage,
    notice: Option<&'static str>,
}

impl CredentialPrompt {
    pub fn new() -> Self {
        Self {
            stage: Stage::Choose,
            notice: None,
        }
    }

    pub fn prompt_text(&self) -> &'static str {
        match self.stage {
            Stage::Choose => "Choose unlock profile: s symmetric, p public-key, g signed, n plain.",
            Stage::SecretPath { .. } => {
                "Secret-key export file path (visible; file contents stay hidden):"
            }
            Stage::KeyPassphrase { .. } => {
                "Key passphrase (input hidden; no mask or length is shown):"
            }
            Stage::SignerPath { .. } => {
                "Signer public-key export file path (visible; file contents stay hidden):"
            }
            Stage::Fingerprint { .. } => {
                "Permitted primary fingerprint (40 hexadecimal characters):"
            }
            Stage::MessagePassphrase { .. } => {
                "Message passphrase (input hidden; no mask or length is shown):"
            }
        }
    }

    pub fn visible_input(&self) -> Option<&str> {
        match &self.stage {
            Stage::Choose => None,
            Stage::SecretPath { input, .. }
            | Stage::SignerPath { input, .. }
            | Stage::Fingerprint { input, .. } => Some(input),
            Stage::KeyPassphrase { .. } | Stage::MessagePassphrase { .. } => None,
        }
    }

    pub fn notice(&self) -> Option<&'static str> {
        self.notice
    }

    pub fn accepts_secret_input(&self) -> bool {
        matches!(
            self.stage,
            Stage::KeyPassphrase { .. } | Stage::MessagePassphrase { .. }
        )
    }

    pub fn input_limit(&self) -> usize {
        if self.accepts_secret_input() {
            PASSPHRASE_LIMIT
        } else {
            PATH_INPUT_LIMIT
        }
    }

    //= docs/parsing-requirements.md#restore-ui-015
    //= type=implementation
    //# When the frontend supports OpenPGP archives, it MUST collect archive-unlock credentials only through an explicit no-echo interaction before snapshot construction and keep that interaction separate from destination selection, restore preview, and confirmation.
    //= docs/parsing-requirements.md#restore-ui-015
    //= type=implementation
    //# The frontend MUST NOT accept OpenPGP passphrases or exported key bytes through argv or environment values, display them, retain them in browser history, or copy them into catalog objects, snapshots, authentication evidence, receipts, diagnostics, or logs.
    //= docs/parsing-requirements.md#restore-ui-015
    //= type=implementation
    //# The passphrase prompt MUST NOT show a mask, length, or other input-dependent indication.
    //= docs/parsing-requirements.md#restore-ui-015
    //= type=implementation
    //# It MUST accept at most 4096 UTF-8 bytes and check a candidate character's encoded byte length before allocating/appending it, rejecting an over-limit character without growing the input.
    pub fn handle(&mut self, key_code: KeyCode) -> PromptResult {
        self.notice = None;
        if key_code == KeyCode::Esc {
            return PromptResult::Cancelled;
        }
        match &mut self.stage {
            Stage::Choose => match key_code {
                KeyCode::Char('s') => {
                    self.stage = Stage::MessagePassphrase {
                        input: String::new(),
                    };
                    PromptResult::Pending
                }
                KeyCode::Char('p') => {
                    self.stage = Stage::SecretPath {
                        profile: Profile::PublicKey,
                        input: String::new(),
                    };
                    PromptResult::Pending
                }
                KeyCode::Char('g') => {
                    self.stage = Stage::SecretPath {
                        profile: Profile::Signed,
                        input: String::new(),
                    };
                    PromptResult::Pending
                }
                KeyCode::Char('n') => PromptResult::Complete(None),
                _ => {
                    self.notice = Some("Choose s, p, g, or n; Esc cancels.");
                    PromptResult::Pending
                }
            },
            Stage::SecretPath { profile, input } => match edit_visible(input, key_code) {
                Edit::Continue => PromptResult::Pending,
                Edit::Submit => match read_key_export(input) {
                    Ok(key) => {
                        let profile = std::mem::replace(profile, Profile::PublicKey);
                        self.stage = Stage::KeyPassphrase {
                            profile,
                            key,
                            input: String::new(),
                        };
                        PromptResult::Pending
                    }
                    Err(notice) => {
                        self.notice = Some(notice);
                        PromptResult::Pending
                    }
                },
                Edit::Cancel => PromptResult::Cancelled,
            },
            Stage::KeyPassphrase {
                profile,
                key: secret_key,
                input,
            } => match edit_secret(input, key_code) {
                Edit::Continue => PromptResult::Pending,
                Edit::Submit => {
                    let profile = std::mem::replace(profile, Profile::PublicKey);
                    let secret_export = std::mem::take(secret_key);
                    let key_passphrase = OpenPgpPassword::from(std::mem::take(input));
                    match profile {
                        Profile::PublicKey => {
                            PromptResult::Complete(Some(ReadCredentials::PublicKey {
                                secret_export,
                                key_passphrase,
                            }))
                        }
                        Profile::Signed => {
                            self.stage = Stage::SignerPath {
                                key: secret_export,
                                key_passphrase,
                                input: String::new(),
                            };
                            PromptResult::Pending
                        }
                    }
                }
                Edit::Cancel => PromptResult::Cancelled,
            },
            Stage::SignerPath {
                key: secret_key,
                key_passphrase,
                input,
            } => match edit_visible(input, key_code) {
                Edit::Continue => PromptResult::Pending,
                Edit::Submit => match read_key_export(input) {
                    Ok(signer) => {
                        self.stage = Stage::Fingerprint {
                            key: std::mem::take(secret_key),
                            key_passphrase: std::mem::take(key_passphrase),
                            signer,
                            input: String::new(),
                        };
                        PromptResult::Pending
                    }
                    Err(notice) => {
                        self.notice = Some(notice);
                        PromptResult::Pending
                    }
                },
                Edit::Cancel => PromptResult::Cancelled,
            },
            Stage::Fingerprint {
                key: secret_key,
                key_passphrase,
                signer,
                input,
            } => match edit_visible(input, key_code) {
                Edit::Continue => PromptResult::Pending,
                Edit::Submit => match parse_fingerprint(input) {
                    Some(primary_fingerprint) => {
                        PromptResult::Complete(Some(ReadCredentials::Signed {
                            secret_export: std::mem::take(secret_key),
                            key_passphrase: std::mem::take(key_passphrase),
                            signer_export: std::mem::take(signer),
                            primary_fingerprint,
                        }))
                    }
                    None => {
                        self.notice =
                            Some("Fingerprint must be exactly 40 hexadecimal characters.");
                        PromptResult::Pending
                    }
                },
                Edit::Cancel => PromptResult::Cancelled,
            },
            Stage::MessagePassphrase { input } => match edit_secret(input, key_code) {
                Edit::Continue => PromptResult::Pending,
                Edit::Submit => PromptResult::Complete(Some(ReadCredentials::Symmetric {
                    message_passphrase: OpenPgpPassword::from(std::mem::take(input)),
                })),
                Edit::Cancel => PromptResult::Cancelled,
            },
        }
    }
}

impl Default for CredentialPrompt {
    fn default() -> Self {
        Self::new()
    }
}

enum Edit {
    Continue,
    Submit,
    Cancel,
}

fn edit_visible(input: &mut String, key: KeyCode) -> Edit {
    match key {
        KeyCode::Enter => Edit::Submit,
        KeyCode::Backspace => {
            input.pop();
            Edit::Continue
        }
        KeyCode::Char(ch) if !ch.is_control() => {
            append_candidate(input, ch, PATH_INPUT_LIMIT);
            Edit::Continue
        }
        KeyCode::Esc => Edit::Cancel,
        _ => Edit::Continue,
    }
}

fn edit_secret(input: &mut String, key: KeyCode) -> Edit {
    match key {
        KeyCode::Enter => Edit::Submit,
        KeyCode::Backspace => {
            input.pop();
            Edit::Continue
        }
        KeyCode::Char(ch) if !ch.is_control() => {
            append_candidate(input, ch, PASSPHRASE_LIMIT);
            Edit::Continue
        }
        KeyCode::Esc => Edit::Cancel,
        _ => Edit::Continue,
    }
}

fn append_candidate(input: &mut String, candidate: char, limit: usize) -> bool {
    if input.len().saturating_add(candidate.len_utf8()) > limit {
        return false;
    }
    input.push(candidate);
    true
}

//= docs/parsing-requirements.md#restore-ui-015
//= type=implementation
//# Exported key material MUST be read from explicitly selected files; SDK/storage credentials remain with the configured provider and outside this archive-unlock interaction.
//= docs/parsing-requirements.md#restore-ui-015
//= type=implementation
//# Before reading an exported key file, the frontend MUST verify that it is a regular file whose metadata length is at most 16 MiB, then read with a 16 MiB-plus-one-byte cap and reject overflow if the read exceeds the cap, including when the file grows during the read.
fn read_key_export(path: &str) -> Result<Vec<u8>, &'static str> {
    if path.is_empty() {
        return Err("Select a regular key-export file.");
    }
    let path = Path::new(path);
    #[cfg(unix)]
    let file = {
        use rustix::fs::{open, Mode, OFlags};
        let fd = open(
            path,
            OFlags::RDONLY | OFlags::CLOEXEC | OFlags::NOFOLLOW | OFlags::NONBLOCK,
            Mode::empty(),
        )
        .map_err(|_| "Could not open selected key-export file.")?;
        File::from(fd)
    };
    #[cfg(not(unix))]
    let file = File::open(path).map_err(|_| "Could not open selected key-export file.")?;
    let metadata = file
        .metadata()
        .map_err(|_| "Could not inspect selected key-export file.")?;
    if !metadata.is_file() {
        return Err("Selected key-export path is not a regular file.");
    }
    if metadata.len() > KEY_EXPORT_LIMIT as u64 {
        return Err("Selected key-export file exceeds the 16 MiB limit.");
    }
    let mut bytes = Vec::new();
    file.take(KEY_EXPORT_LIMIT as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "Could not read selected key-export file.")?;
    if bytes.len() > KEY_EXPORT_LIMIT {
        return Err("Selected key-export file exceeds the 16 MiB limit.");
    }
    Ok(bytes)
}

fn parse_fingerprint(input: &str) -> Option<[u8; 20]> {
    if input.len() != 40 {
        return None;
    }
    let mut fingerprint = [0; 20];
    let (pairs, remainder) = input.as_bytes().as_chunks::<2>();
    if !remainder.is_empty() {
        return None;
    }
    for (index, pair) in pairs.iter().enumerate() {
        let hi = hex(pair[0])?;
        let lo = hex(pair[1])?;
        fingerprint[index] = hi * 16 + lo;
    }
    Some(fingerprint)
}

fn hex(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn type_text(prompt: &mut CredentialPrompt, text: &str) {
        for ch in text.chars() {
            assert!(matches!(
                prompt.handle(KeyCode::Char(ch)),
                PromptResult::Pending
            ));
        }
    }

    #[test]
    //= docs/parsing-requirements.md#restore-ui-015
    //= type=test
    //# The passphrase prompt MUST NOT show a mask, length, or other input-dependent indication. It MUST accept at most 4096 UTF-8 bytes and check a candidate character's encoded byte length before allocating/appending it, rejecting an over-limit character without growing the input.
    //= docs/parsing-requirements.md#restore-ui-015
    //= type=test
    //# When the frontend supports OpenPGP archives, it MUST collect archive-unlock credentials only through an explicit no-echo interaction before snapshot construction and keep that interaction separate from destination selection, restore preview, and confirmation.
    //= docs/parsing-requirements.md#restore-ui-015
    //= type=test
    //# The frontend MUST NOT accept OpenPGP passphrases or exported key bytes through argv or environment values, display them, retain them in browser history, or copy them into catalog objects, snapshots, authentication evidence, receipts, diagnostics, or logs.
    fn passphrase_is_never_rendered_and_utf8_limit_checks_before_append() {
        let secret = "é".repeat(PASSPHRASE_LIMIT / 2);
        let mut prompt = CredentialPrompt::new();
        prompt.handle(KeyCode::Char('s'));
        type_text(&mut prompt, &secret);
        assert!(prompt.accepts_secret_input());
        assert_eq!(prompt.visible_input(), None);
        assert_eq!(
            prompt.prompt_text(),
            "Message passphrase (input hidden; no mask or length is shown):"
        );
        assert_eq!(prompt.input_limit(), PASSPHRASE_LIMIT);
        prompt.handle(KeyCode::Char('x'));
        assert_eq!(prompt.visible_input(), None);
        assert!(matches!(
            prompt.handle(KeyCode::Enter),
            PromptResult::Complete(Some(_))
        ));

        let mut boundary = CredentialPrompt::new();
        boundary.handle(KeyCode::Char('s'));
        type_text(&mut boundary, &"a".repeat(PASSPHRASE_LIMIT - 1));
        boundary.handle(KeyCode::Char('é'));
        assert_eq!(boundary.visible_input(), None);
        assert_eq!(boundary.input_limit(), PASSPHRASE_LIMIT);
        let mut internal = "a".repeat(PASSPHRASE_LIMIT - 1);
        assert!(!append_candidate(&mut internal, 'é', PASSPHRASE_LIMIT));
        assert_eq!(internal.len(), PASSPHRASE_LIMIT - 1);
    }

    #[test]
    //= docs/parsing-requirements.md#restore-ui-015
    //= type=test
    //# A visible selected-file path MUST accept at most 4096 UTF-8 bytes, be byte-escaped for terminal rendering, and render within a fixed bounded field.
    fn visible_path_validator_accepts_limit_and_rejects_overflow_without_opening_it() {
        let mut prompt = CredentialPrompt::new();
        prompt.handle(KeyCode::Char('p'));
        type_text(&mut prompt, &"a".repeat(PATH_INPUT_LIMIT));
        assert_eq!(prompt.visible_input().unwrap().len(), PATH_INPUT_LIMIT);
        prompt.handle(KeyCode::Char('b'));
        assert_eq!(prompt.visible_input().unwrap().len(), PATH_INPUT_LIMIT);
        assert!(prompt.prompt_text().contains("visible"));
    }

    #[test]
    //= docs/parsing-requirements.md#restore-ui-015
    //= type=test
    //# Before reading an exported key file, the frontend MUST verify that it is a regular file whose metadata length is at most 16 MiB, then read with a 16 MiB-plus-one-byte cap and reject overflow if the read exceeds the cap, including when the file grows during the read.
    //= docs/parsing-requirements.md#restore-ui-015
    //= type=test
    //# Exported key material MUST be read from explicitly selected files; SDK/storage credentials remain with the configured provider and outside this archive-unlock interaction.
    fn selected_key_export_requires_regular_file_and_enforces_read_cap() {
        use std::{
            fs,
            sync::atomic::{AtomicUsize, Ordering},
        };
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let root = std::env::temp_dir().join(format!(
            "tui-credential-path-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        let regular = root.join("export.asc");
        fs::write(&regular, b"public fixture key export").unwrap();
        assert_eq!(
            read_key_export(regular.to_str().unwrap()).unwrap(),
            b"public fixture key export"
        );
        assert!(read_key_export(root.to_str().unwrap())
            .unwrap_err()
            .contains("regular file"));
        let oversized = root.join("oversized.asc");
        let file = File::create(&oversized).unwrap();
        file.set_len(KEY_EXPORT_LIMIT as u64 + 1).unwrap();
        assert!(read_key_export(oversized.to_str().unwrap())
            .unwrap_err()
            .contains("16 MiB"));
        let exact_limit = root.join("exact-limit.asc");
        File::create(&exact_limit)
            .unwrap()
            .set_len(KEY_EXPORT_LIMIT as u64)
            .unwrap();
        assert_eq!(
            read_key_export(exact_limit.to_str().unwrap())
                .unwrap()
                .len(),
            KEY_EXPORT_LIMIT
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    //= docs/parsing-requirements.md#parse-env-009
    //= type=test
    //# The adapter MUST support caller-supplied passphrases and exported supported secret keys without invoking external key agents or crypto applications.
    //= docs/parsing-requirements.md#parse-env-021
    //= type=test
    //# The reader integration MUST borrow caller-supplied credential and authorization inputs without cloning secret exports or passphrases into catalog objects, object names, snapshots, evidence receipts, diagnostics or logs.
    fn profile_builds_borrowed_context_with_exact_policy() {
        let symmetric = ReadCredentials::Symmetric {
            message_passphrase: OpenPgpPassword::from("never-render-this"),
        };
        symmetric.with_context(|context| {
            assert_eq!(context.credentials.message_passwords.len(), 1);
            assert!(context.credentials.secret_exports.is_empty());
            assert!(context.credentials.signer_exports.is_empty());
            assert!(!context.credentials.require_signature);
        });
        let public_key = ReadCredentials::PublicKey {
            secret_export: b"secret-key-bytes".to_vec(),
            key_passphrase: OpenPgpPassword::from("private"),
        };
        public_key.with_context(|context| {
            assert!(context.credentials.message_passwords.is_empty());
            assert_eq!(context.credentials.secret_exports.len(), 1);
            assert!(context.credentials.signer_exports.is_empty());
            assert!(!context.credentials.require_signature);
        });
        let signed = ReadCredentials::Signed {
            secret_export: b"secret-key-bytes".to_vec(),
            key_passphrase: OpenPgpPassword::from("private"),
            signer_export: b"public-key-bytes".to_vec(),
            primary_fingerprint: [0x5a; 20],
        };
        signed.with_context(|context| {
            assert!(context.credentials.message_passwords.is_empty());
            assert_eq!(context.credentials.secret_exports.len(), 1);
            assert_eq!(context.credentials.signer_exports.len(), 1);
            assert_eq!(context.credentials.permitted_signers, [[0x5a; 20]]);
            assert!(context.credentials.require_signature);
        });
    }

    #[test]
    fn fingerprint_parser_requires_exact_40_hex_bytes() {
        assert_eq!(
            parse_fingerprint("d06847bac3b117c46bb68bda6f23173bad8a7656"),
            Some([
                0xd0, 0x68, 0x47, 0xba, 0xc3, 0xb1, 0x17, 0xc4, 0x6b, 0xb6, 0x8b, 0xda, 0x6f, 0x23,
                0x17, 0x3b, 0xad, 0x8a, 0x76, 0x56,
            ])
        );
        assert_eq!(parse_fingerprint("xyz"), None);
    }
}
