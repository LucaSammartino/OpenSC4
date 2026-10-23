//! Base cSCStringDetokenizer, rz-strings.md §§6–8. Resource management remains
//! foreign. No city bindings, locale offsets, persistence or implicit recursion.

use std::collections::BTreeMap;
use std::sync::Arc;

use super::rz_string::c_prefix;
use super::{Released, RzString, StringFault, StringSource, IID_STRING, IID_UNKNOWN};

pub const IID_DETOKENIZER: u32 = 0x8A2C_3240;

type Callback = dyn Fn(&RzString, &mut RzString, u32) -> bool + Send + Sync;

/// Identity is the unsigned native function address (or a deterministic portable
/// identity). Context is unowned data: no COM reference operations are performed.
#[derive(Clone)]
pub struct ReplacementMethod {
    pub identity: u32,
    pub context: u32,
    pub callback: Arc<Callback>,
}

impl ReplacementMethod {
    pub fn new(
        identity: u32,
        context: u32,
        callback: impl Fn(&RzString, &mut RzString, u32) -> bool + Send + Sync + 'static,
    ) -> Self {
        Self {
            identity,
            context,
            callback: Arc::new(callback),
        }
    }

    fn call(&self, token: &RzString, output: &mut RzString) -> bool {
        (self.callback)(token, output, self.context)
    }
}

pub struct StringRetrieval<H> {
    pub success: bool,
    pub interface: Option<H>,
}

/// The caller supplies the resource manager; it is not implemented by this owner.
pub trait ResourceStrings {
    type Key;
    type Handle: StringSource;
    fn retrieve(&mut self, key: &Self::Key, iid: u32, arg3: u32, arg4: u32) -> StringRetrieval<Self::Handle>;
    fn release(&mut self, handle: Self::Handle);
}

/// Models slots 9 -> 13 and parser -> 14. Overrides implement this trait and
/// may call the public base parser with themselves as the virtual receiver.
pub trait DetokenizerDispatch {
    fn convert_token(&mut self, token: &RzString, output: &mut RzString) -> Result<bool, StringFault>;

    fn perform_detokenize(&mut self, input: &RzString, output: &mut RzString) -> Result<bool, StringFault> {
        perform_detokenize(self, input, output)
    }

    fn detokenize(&mut self, input: &RzString, output: &mut RzString) -> Result<bool, StringFault> {
        self.perform_detokenize(input, output)
    }

    /// Rust cannot borrow the same object mutably and immutably. This entry
    /// captures the native alias case by snapshotting before virtual dispatch.
    fn detokenize_in_place(&mut self, text: &mut RzString) -> Result<bool, StringFault> {
        let snapshot = text.clone();
        self.perform_detokenize(&snapshot, text)
    }

    fn detokenize_resource<R: ResourceStrings>(
        &mut self,
        resources: &mut R,
        key: &R::Key,
        output: &mut RzString,
    ) -> Result<bool, StringFault> {
        let retrieval = resources.retrieve(key, IID_STRING, 0, 0);
        let result = if retrieval.success {
            let handle = retrieval
                .interface
                .as_ref()
                .expect("successful retrieval requires a live interface");
            self.perform_detokenize(&RzString::from_bytes(handle.bytes()), output)
        } else {
            Ok(false)
        };
        // Release non-null output even on retrieval failure, and also on a
        // portable Unsupported result. Do not Release anything not acquired.
        if let Some(handle) = retrieval.interface {
            resources.release(handle);
        }
        result
    }
}

#[derive(Default)]
pub struct Detokenizer {
    refs: u32,
    values: Vec<(RzString, RzString)>,
    methods: Vec<(RzString, ReplacementMethod)>,
    unknown: BTreeMap<u32, ReplacementMethod>,
    html_clean: u8,
}

// strnicmp over the common counted prefix stops at NUL, then key ordering
// breaks ties by ORIGINAL length. Non-ASCII collation is explicitly RZS2.
fn key_equal(a: &[u8], b: &[u8]) -> Result<bool, StringFault> {
    for (&a, &b) in a.iter().zip(b) {
        if a >= 0x80 || b >= 0x80 {
            return Err(StringFault::Unsupported("RZS2"));
        }
        if !a.eq_ignore_ascii_case(&b) {
            return Ok(false);
        }
        if a == 0 {
            break;
        }
    }
    Ok(a.len() == b.len())
}

fn lookup<T>(entries: &[(RzString, T)], name: &[u8]) -> Result<Option<usize>, StringFault> {
    for (index, (key, _)) in entries.iter().enumerate() {
        if key_equal(key.bytes(), name)? {
            return Ok(Some(index));
        }
    }
    Ok(None)
}

impl Detokenizer {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn ref_count(&self) -> u32 {
        self.refs
    }
    pub fn query_interface(&mut self, iid: u32) -> bool {
        if iid == IID_UNKNOWN || iid == IID_DETOKENIZER {
            self.add_ref();
            true
        } else {
            false
        }
    }
    pub fn query_interface_into<T>(&mut self, iid: u32, output: &mut T, identity: T) -> bool {
        if self.query_interface(iid) {
            *output = identity;
            true
        } else {
            false
        }
    }
    pub fn add_ref(&mut self) -> u32 {
        self.refs = self.refs.wrapping_add(1);
        self.refs
    }
    /// As with other portable COM owners, Destroy instructs the holder to drop.
    pub fn release(&mut self) -> Released {
        if self.refs > 1 {
            self.refs -= 1;
            Released::Alive(self.refs)
        } else {
            Released::Destroy
        }
    }
    pub fn init(&mut self) -> bool {
        true
    }
    pub fn shutdown(&mut self) -> bool {
        true
    }
    pub fn set_html_clean_mode(&mut self, value: u8) {
        self.html_clean = value;
    }
    pub fn html_clean_mode(&self) -> u8 {
        self.html_clean
    }

    pub fn set_token_replacement_value(
        &mut self,
        name: &impl StringSource,
        value: &impl StringSource,
    ) -> Result<(), StringFault> {
        // Validate even an empty registry: otherwise non-ASCII registration
        // could succeed and only fail later depending on insertion order.
        if name.bytes().iter().any(|&b| b >= 0x80) {
            return Err(StringFault::Unsupported("RZS2"));
        }
        if let Some(index) = lookup(&self.values, name.bytes())? {
            self.values[index].1.copy(value);
        } else {
            self.values
                .push((RzString::from_bytes(name.bytes()), RzString::from_bytes(value.bytes())));
        }
        Ok(())
    }

    pub fn set_token_replacement_method(
        &mut self,
        name: &impl StringSource,
        method: ReplacementMethod,
    ) -> Result<(), StringFault> {
        if name.bytes().iter().any(|&b| b >= 0x80) {
            return Err(StringFault::Unsupported("RZS2"));
        }
        if let Some(index) = lookup(&self.methods, name.bytes())? {
            self.methods[index].1 = method;
        } else {
            self.methods.push((RzString::from_bytes(name.bytes()), method));
        }
        Ok(())
    }

    pub fn add_unknown_token_replacement_method(&mut self, method: ReplacementMethod, enable: bool) {
        if enable {
            self.unknown.entry(method.identity).or_insert(method);
        } else {
            self.unknown.remove(&method.identity);
        }
    }
}

impl DetokenizerDispatch for Detokenizer {
    fn convert_token(&mut self, token: &RzString, output: &mut RzString) -> Result<bool, StringFault> {
        let accepted = if let Some(index) = lookup(&self.values, token.bytes())? {
            output.copy(&self.values[index].1);
            true
        } else {
            let mut accepted = false;
            if let Some(index) = lookup(&self.methods, token.bytes())? {
                accepted = self.methods[index].1.call(token, output);
            }
            if !accepted {
                for method in self.unknown.values() {
                    if method.call(token, output) {
                        accepted = true;
                        break;
                    }
                }
            }
            if !accepted {
                output.copy(token);
            }
            accepted
        };
        if self.html_clean != 0 {
            let mut clean = Vec::new();
            for &byte in c_prefix(output.bytes()) {
                clean.extend_from_slice(match byte {
                    b'"' => b"&quot;",
                    b'&' => b"&amp;",
                    b'<' => b"&lt;",
                    b'>' => b"&gt;",
                    _ => {
                        clean.push(byte);
                        continue;
                    }
                });
            }
            output.from_char(Some(&clean), clean.len() as u32);
        }
        Ok(accepted)
    }
}

/// Base parser, also usable by subclasses so ConvertToken overrides are observed.
pub fn perform_detokenize<D: DetokenizerDispatch + ?Sized>(
    dispatch: &mut D,
    input: &RzString,
    output: &mut RzString,
) -> Result<bool, StringFault> {
    let original = input.bytes();
    let first_hash = match original.iter().position(|&b| b == b'#') {
        None => {
            output.copy(input);
            return Ok(true);
        }
        Some(index) => index,
    };
    let mut snapshot = Vec::with_capacity(original.len());
    let mut i = 0;
    while i < original.len() {
        if original[i] == b'#' && original.get(i + 1) == Some(&b'#') {
            snapshot.push(0);
            i += 2;
        } else {
            snapshot.push(original[i]);
            i += 1;
        }
    }
    let mut assembled = Vec::new();
    let mut literal_start = 0;
    // No earlier pair can shift the first original hash: there were no hashes
    // before it. Its position remains an opener even if now a NUL marker.
    let mut opening = Some(first_hash);
    let mut success = true;
    // A single output object is reused across conversions, as well as callbacks.
    let mut replacement = RzString::new();
    while let Some(start) = opening {
        let end = match snapshot[start + 1..].iter().position(|&b| b == b'#') {
            Some(relative) => start + 1 + relative,
            None => break,
        };
        assembled.extend_from_slice(&snapshot[literal_start..start]);
        let mut token = RzString::from_bytes(&snapshot[start + 1..end]);
        token.trim();
        // Do not short-circuit on false: every complete token is visited.
        success &= dispatch.convert_token(&token, &mut replacement)?;
        assembled.extend_from_slice(replacement.bytes());
        literal_start = end + 1;
        opening = snapshot[literal_start..]
            .iter()
            .position(|&b| b == b'#')
            .map(|n| n + literal_start);
    }
    assembled.extend_from_slice(&snapshot[literal_start..]);
    for b in &mut assembled {
        if *b == 0 {
            *b = b'#';
        }
    }
    output.from_char(Some(&assembled), assembled.len() as u32);
    Ok(success)
}
