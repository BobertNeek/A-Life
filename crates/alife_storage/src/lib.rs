//! Optional process-local logical-byte quotas for exclusively owned output trees.
//!
//! All writers in a registered tree must use these primitives. This is not a
//! filesystem sandbox against another process. Existing files are immutable;
//! replacement uses a new staged file and rename. A retained handle keeps worker
//! writes covered after the initiating stack frame exits. With no live quota,
//! wrappers immediately use the ordinary filesystem operations.
use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::{self, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex, MutexGuard, OnceLock, Weak,
    },
};

static ACTIVE: AtomicUsize = AtomicUsize::new(0);
static REGISTRY: OnceLock<Mutex<Vec<Weak<Inner>>>> = OnceLock::new();

#[derive(Debug, Clone)]
pub struct OutputBudget(Arc<Inner>);
impl PartialEq for OutputBudget {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for OutputBudget {}

#[derive(Debug)]
struct Inner {
    root: PathBuf,
    limit: u64,
    state: Mutex<State>,
}
impl Drop for Inner {
    fn drop(&mut self) {
        ACTIVE.fetch_sub(1, Ordering::Release);
    }
}

#[derive(Debug, Clone)]
struct Entry {
    bytes: u64,
    writers: usize,
    immutable: bool,
}
#[derive(Debug, Default)]
struct State {
    entries: BTreeMap<PathBuf, Entry>,
    used: u64,
    held: u64,
    high_water: u64,
    poisoned: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BudgetUsage {
    pub limit: u64,
    pub used: u64,
    pub reserved: u64,
    pub high_water: u64,
}

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}
fn exhausted() -> io::Error {
    io::Error::new(
        io::ErrorKind::StorageFull,
        "aggregate temporary output budget exhausted before write",
    )
}
fn lock<T>(m: &Mutex<T>) -> io::Result<MutexGuard<'_, T>> {
    m.lock()
        .map_err(|_| invalid("output-budget mutex poisoned"))
}

#[cfg(windows)]
fn reparse(m: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    m.file_attributes() & 0x400 != 0
}
#[cfg(not(windows))]
fn reparse(m: &fs::Metadata) -> bool {
    m.file_type().is_symlink()
}

/// Resolve existing prefixes through the OS, including Windows verbatim paths.
fn resolved(path: &Path) -> io::Result<PathBuf> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };
    let mut pending = Vec::new();
    let mut at = absolute.as_path();
    loop {
        match fs::symlink_metadata(at) {
            Ok(_) => {
                let mut value = fs::canonicalize(at)?;
                for part in pending.iter().rev() {
                    value.push(part);
                }
                return Ok(value);
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                pending.push(
                    at.file_name()
                        .ok_or_else(|| invalid("output path lacks an existing prefix"))?
                        .to_os_string(),
                );
                at = at
                    .parent()
                    .ok_or_else(|| invalid("output path lacks a parent"))?;
            }
            Err(e) => return Err(e),
        }
    }
}

fn scan(root: &Path, entries: &mut BTreeMap<PathBuf, Entry>) -> io::Result<u64> {
    let mut total = 0_u64;
    for item in fs::read_dir(root)? {
        let path = item?.path();
        let m = fs::symlink_metadata(&path)?;
        if reparse(&m) {
            return Err(invalid("output-budget tree contains a reparse point"));
        }
        let bytes = if m.is_dir() {
            scan(&path, entries)?
        } else if m.is_file() {
            entries.insert(
                fs::canonicalize(&path)?,
                Entry {
                    bytes: m.len(),
                    writers: 0,
                    immutable: true,
                },
            );
            m.len()
        } else {
            return Err(invalid("output-budget tree contains a special file"));
        };
        total = total
            .checked_add(bytes)
            .ok_or_else(|| invalid("output byte count overflow"))?;
    }
    Ok(total)
}

impl OutputBudget {
    pub fn register(root: impl AsRef<Path>, limit: u64) -> io::Result<Self> {
        if limit == 0 {
            return Err(exhausted());
        }
        let m = fs::symlink_metadata(root.as_ref())?;
        if !m.is_dir() || reparse(&m) {
            return Err(invalid("output budget requires an owned regular directory"));
        }
        let root = fs::canonicalize(root)?;
        let mut registry = lock(REGISTRY.get_or_init(|| Mutex::new(Vec::new())))?;
        registry.retain(|old| old.strong_count() != 0);
        for old in registry.iter().filter_map(Weak::upgrade) {
            if root.starts_with(&old.root) || old.root.starts_with(&root) {
                return Err(invalid("overlapping output budgets"));
            }
        }
        let mut state = State::default();
        state.used = scan(&root, &mut state.entries)?;
        state.high_water = state.used;
        if state.used > limit {
            return Err(exhausted());
        }
        ACTIVE.fetch_add(1, Ordering::Release);
        let value = Self(Arc::new(Inner {
            root,
            limit,
            state: Mutex::new(state),
        }));
        registry.push(Arc::downgrade(&value.0));
        Ok(value)
    }
    pub fn usage(&self) -> io::Result<BudgetUsage> {
        let s = lock(&self.0.state)?;
        Ok(BudgetUsage {
            limit: self.0.limit,
            used: s.used,
            reserved: s.held,
            high_water: s.high_water,
        })
    }
    pub fn root(&self) -> &Path {
        &self.0.root
    }
    /// Held capacity is unavailable to ordinary writes, e.g. recovery headroom.
    pub fn reserve(&self, bytes: u64) -> io::Result<Reservation> {
        let mut s = lock(&self.0.state)?;
        self.check(&s, bytes)?;
        s.held = s.held.checked_add(bytes).ok_or_else(exhausted)?;
        Ok(Reservation {
            budget: self.clone(),
            bytes,
        })
    }
    fn check(&self, s: &State, additional: u64) -> io::Result<()> {
        if s.poisoned {
            return Err(invalid("output accounting failed; refusing further writes"));
        }
        if s.used
            .checked_add(s.held)
            .and_then(|n| n.checked_add(additional))
            .is_none_or(|n| n > self.0.limit)
        {
            return Err(exhausted());
        }
        Ok(())
    }
    fn writable(&self, path: &Path) -> io::Result<()> {
        let key = resolved(path)?;
        let s = lock(&self.0.state)?;
        self.check(&s, 0)?;
        if s.entries.get(&key).is_some_and(|e| e.immutable) {
            return Err(invalid(
                "budgeted immutable file requires staged replacement",
            ));
        }
        if s.entries.get(&key).is_some_and(|e| e.writers != 0) {
            return Err(invalid("output already has an active writer"));
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct Reservation {
    budget: OutputBudget,
    bytes: u64,
}
impl Drop for Reservation {
    fn drop(&mut self) {
        if let Ok(mut s) = self.budget.0.state.lock() {
            s.held = s.held.saturating_sub(self.bytes);
        }
    }
}

/// Retain this handle in cloned asynchronous stores, not only in thread locals.
pub fn budget_for(path: impl AsRef<Path>) -> io::Result<Option<OutputBudget>> {
    if ACTIVE.load(Ordering::Acquire) == 0 {
        return Ok(None);
    }
    let path = path.as_ref();
    let key = resolved(path)?;
    let registry = lock(REGISTRY.get_or_init(|| Mutex::new(Vec::new())))?;
    for inner in registry.iter().filter_map(Weak::upgrade) {
        let mut belongs = key.starts_with(&inner.root);
        // A descendant reparse point can resolve out of the root. Recognize its
        // existing root ancestor rather than silently treating it as unscoped.
        if !belongs {
            belongs = path
                .ancestors()
                .any(|p| fs::canonicalize(p).is_ok_and(|p| p == inner.root));
        }
        if belongs {
            let m = fs::symlink_metadata(&inner.root)?;
            if reparse(&m) || !m.is_dir() || fs::canonicalize(&inner.root)? != inner.root {
                return Err(invalid("registered output root changed"));
            }
            let absolute = if path.is_absolute() {
                path.to_path_buf()
            } else {
                std::env::current_dir()?.join(path)
            };
            for ancestor in absolute.ancestors() {
                if let Ok(m) = fs::symlink_metadata(ancestor) {
                    if reparse(&m) {
                        return Err(invalid("budgeted output aliases are refused"));
                    }
                }
            }
            if !key.starts_with(&inner.root) {
                return Err(invalid("output escaped its registered root"));
            }
            return Ok(Some(OutputBudget(inner)));
        }
    }
    Ok(None)
}

pub struct BudgetWriter<W> {
    inner: W,
    budget: Option<OutputBudget>,
    path: PathBuf,
    position: u64,
    append: bool,
}
/// Wrap a non-append handle. Append handles must use `append_writer`, because
/// seeking an OS append handle does not change where its next write lands.
/// The quota belongs inside a buffer: accepting a buffered inner writer would
/// release reservations before its bytes reached disk.
/// ```compile_fail
/// let file = std::fs::File::create("not-executed").unwrap();
/// let buffered = std::io::BufWriter::new(file);
/// let _ = alife_storage::writer("not-executed", buffered);
/// ```
pub fn writer(path: impl AsRef<Path>, inner: File) -> io::Result<BudgetWriter<File>> {
    writer_inner(path, inner)
}
// Production factories only supply unbuffered File or &mut File. This private
// generic seam permits tests to inject partial underlying I/O errors.
fn writer_inner<W: Write + Seek>(
    path: impl AsRef<Path>,
    mut inner: W,
) -> io::Result<BudgetWriter<W>> {
    let budget = budget_for(path.as_ref())?;
    let mut key = path.as_ref().to_path_buf();
    let mut position = 0;
    if let Some(b) = &budget {
        key = resolved(path.as_ref())?;
        b.writable(&key)?;
        let m = fs::metadata(&key)?;
        if !m.is_file() {
            return Err(invalid("budgeted writer is not a file"));
        }
        position = inner.stream_position()?;
        let mut s = lock(&b.0.state)?;
        b.check(&s, 0)?;
        if s.entries
            .get(&key)
            .is_some_and(|e| e.immutable || e.writers != 0)
        {
            return Err(invalid("output already has an active or immutable writer"));
        }
        if let Some(old) = s.entries.get(&key) {
            if m.len() > old.bytes {
                return Err(invalid("file grew outside output accounting"));
            }
            let released = old.bytes - m.len();
            s.used -= released;
        } else if m.len() != 0 {
            return Err(invalid("unaccounted nonempty output file"));
        }
        let entry = s.entries.entry(key.clone()).or_insert(Entry {
            bytes: 0,
            writers: 0,
            immutable: false,
        });
        entry.bytes = m.len();
        entry.writers += 1;
    }
    Ok(BudgetWriter {
        inner,
        budget,
        path: key,
        position,
        append: false,
    })
}
pub fn append_writer(path: impl AsRef<Path>, inner: File) -> io::Result<BudgetWriter<File>> {
    let mut value = writer_inner(path, inner)?;
    value.append = true;
    Ok(value)
}
impl<W> BudgetWriter<W> {
    pub fn get_ref(&self) -> &W {
        &self.inner
    }
}
impl BudgetWriter<File> {
    pub fn sync_all(&self) -> io::Result<()> {
        self.inner.sync_all()
    }
    pub fn metadata(&self) -> io::Result<fs::Metadata> {
        self.inner.metadata()
    }
}
impl<W> Drop for BudgetWriter<W> {
    fn drop(&mut self) {
        if let Some(b) = &self.budget {
            if let Ok(mut s) = b.0.state.lock() {
                if let Some(e) = s.entries.get_mut(&self.path) {
                    e.writers = e.writers.saturating_sub(1);
                }
            }
        }
    }
}
impl<W: Write + Seek> Write for BudgetWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let Some(b) = &self.budget else {
            return self.inner.write(buf);
        };
        let mut s = lock(&b.0.state)?;
        let old = s
            .entries
            .get(&self.path)
            .ok_or_else(|| invalid("output file accounting missing"))?
            .bytes;
        let end = self
            .position
            .checked_add(buf.len() as u64)
            .ok_or_else(exhausted)?;
        let growth = if self.append {
            buf.len() as u64
        } else {
            end.saturating_sub(old)
        };
        b.check(&s, growth)?;
        s.used += growth;
        s.high_water = s.high_water.max(s.used);
        let result = self.inner.write(buf);
        let actual = match fs::metadata(&self.path) {
            Ok(m) if m.len() <= old + growth => m.len(),
            _ => {
                s.poisoned = true;
                old + growth
            }
        };
        if let Ok(position) = self.inner.stream_position() {
            self.position = position;
        } else {
            s.poisoned = true;
        }
        s.used = s.used - (old + growth) + actual;
        s.entries
            .get_mut(&self.path)
            .expect("entry held under lock")
            .bytes = actual;
        if s.poisoned {
            return Err(invalid("output accounting could not verify write"));
        }
        result
    }
    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}
impl<W: Seek> Seek for BudgetWriter<W> {
    fn seek(&mut self, p: SeekFrom) -> io::Result<u64> {
        let next = self.inner.seek(p)?;
        self.position = next;
        Ok(next)
    }
}

pub fn write_all(path: impl AsRef<Path>, file: &mut File, bytes: &[u8]) -> io::Result<()> {
    writer_inner(path, file)?.write_all(bytes)
}
/// Open/truncate and register under the same accounting lock. Another scoped
/// writer cannot be truncated between admission and handle registration.
fn create(path: &Path) -> io::Result<BudgetWriter<File>> {
    let Some(b) = budget_for(path)? else {
        return writer(path, File::create(path)?);
    };
    let key = resolved(path)?;
    let mut s = lock(&b.0.state)?;
    b.check(&s, 0)?;
    if s.entries
        .get(&key)
        .is_some_and(|e| e.immutable || e.writers != 0)
    {
        return Err(invalid("cannot truncate immutable or active output"));
    }
    if let Ok(m) = fs::metadata(path) {
        if s.entries.get(&key).is_none_or(|e| e.bytes != m.len()) && m.len() != 0 {
            return Err(invalid("cannot truncate unaccounted output"));
        }
    }
    let file = File::create(path)?;
    if let Some(old) = s.entries.remove(&key) {
        s.used -= old.bytes;
    }
    s.entries.insert(
        key.clone(),
        Entry {
            bytes: 0,
            writers: 1,
            immutable: false,
        },
    );
    drop(s);
    Ok(BudgetWriter {
        inner: file,
        budget: Some(b),
        path: key,
        position: 0,
        append: false,
    })
}
pub fn write(path: impl AsRef<Path>, bytes: impl AsRef<[u8]>) -> io::Result<()> {
    create(path.as_ref())?.write_all(bytes.as_ref())
}
pub fn copy(source: impl AsRef<Path>, destination: impl AsRef<Path>) -> io::Result<u64> {
    if budget_for(destination.as_ref())?.is_none() {
        return fs::copy(source, destination);
    }
    if resolved(source.as_ref())? == resolved(destination.as_ref())? {
        return Err(invalid("output copy would overwrite its source"));
    }
    let mut input = File::open(source.as_ref())?;
    let permissions = input.metadata()?.permissions();
    let mut output = create(destination.as_ref())?;
    let count = io::copy(&mut input, &mut output)?;
    output.flush()?;
    fs::set_permissions(destination, permissions)?;
    Ok(count)
}
pub fn hard_link(source: impl AsRef<Path>, destination: impl AsRef<Path>) -> io::Result<()> {
    let source = source.as_ref();
    let destination = destination.as_ref();
    let Some(b) = budget_for(destination)? else {
        return fs::hard_link(source, destination);
    };
    if budget_for(source)?.as_ref() != Some(&b) {
        return Err(invalid("cross-budget hard link refused; copy instead"));
    }
    let src = resolved(source)?;
    let dst = resolved(destination)?;
    let bytes = fs::metadata(source)?.len();
    let mut s = lock(&b.0.state)?;
    if destination.exists() {
        return fs::hard_link(source, destination);
    }
    b.check(&s, bytes)?;
    if s.entries.get(&src).is_none_or(|e| e.bytes != bytes) {
        return Err(invalid("unaccounted hard-link source"));
    }
    if s.entries.get(&src).is_some_and(|e| e.writers != 0) {
        return Err(invalid("cannot publish a file with an active writer"));
    }
    fs::hard_link(source, destination)?;
    s.used += bytes;
    s.high_water = s.high_water.max(s.used);
    if let Some(e) = s.entries.get_mut(&src) {
        e.immutable = true;
    }
    s.entries.insert(
        dst,
        Entry {
            bytes,
            writers: 0,
            immutable: true,
        },
    );
    Ok(())
}

/// Account an existing platform-specific atomic rename without replacing its
/// durability primitive. Both paths must belong to the same quota (or neither).
pub fn replace_with(
    source: &Path,
    destination: &Path,
    replace: impl FnOnce() -> io::Result<()>,
) -> io::Result<()> {
    let a = budget_for(source)?;
    let b = budget_for(destination)?;
    let Some(b) = b else {
        if a.is_some() {
            return Err(invalid("cross-budget rename refused"));
        }
        return replace();
    };
    if a.as_ref() != Some(&b) {
        return Err(invalid("cross-budget rename refused"));
    }
    let src = resolved(source)?;
    let dst = resolved(destination)?;
    let mut s = lock(&b.0.state)?;
    b.check(&s, 0)?;
    verify_accounted_tree(&src, &s)?;
    if destination.exists() {
        verify_accounted_tree(&dst, &s)?;
    }
    let moved = s
        .entries
        .iter()
        .filter(|(p, _)| p.starts_with(&src))
        .map(|(p, e)| (p.clone(), e.clone()))
        .collect::<Vec<_>>();
    if moved.iter().any(|(_, e)| e.writers != 0) {
        return Err(invalid("cannot rename an active output writer"));
    }
    replace()?;
    let removed = s
        .entries
        .keys()
        .filter(|p| p.starts_with(&dst) && !p.starts_with(&src))
        .cloned()
        .collect::<Vec<_>>();
    for p in removed {
        if let Some(e) = s.entries.remove(&p) {
            s.used -= e.bytes;
        }
    }
    for (p, mut e) in moved {
        s.entries.remove(&p);
        e.immutable = true;
        s.entries.insert(
            dst.join(
                p.strip_prefix(&src)
                    .map_err(|_| invalid("rename prefix changed"))?,
            ),
            e,
        );
    }
    Ok(())
}
fn verify_accounted_tree(path: &Path, state: &State) -> io::Result<()> {
    let metadata = fs::symlink_metadata(path)?;
    if reparse(&metadata) {
        return Err(invalid("output rename contains a reparse point"));
    }
    if metadata.is_dir() {
        for entry in fs::read_dir(path)? {
            verify_accounted_tree(&entry?.path(), state)?;
        }
    } else if !metadata.is_file()
        || state
            .entries
            .get(path)
            .is_none_or(|e| e.bytes != metadata.len())
    {
        return Err(invalid("unaccounted output rename source or destination"));
    }
    Ok(())
}
pub fn rename(source: impl AsRef<Path>, destination: impl AsRef<Path>) -> io::Result<()> {
    let a = source.as_ref();
    let b = destination.as_ref();
    replace_with(a, b, || fs::rename(a, b))
}
pub fn remove_file(path: impl AsRef<Path>) -> io::Result<()> {
    let path = path.as_ref();
    let Some(b) = budget_for(path)? else {
        return fs::remove_file(path);
    };
    let key = resolved(path)?;
    let mut s = lock(&b.0.state)?;
    if s.entries.get(&key).is_some_and(|e| e.writers != 0) {
        return Err(invalid("cannot remove an active output writer"));
    }
    fs::remove_file(path)?;
    if let Some(e) = s.entries.remove(&key) {
        s.used -= e.bytes;
    }
    Ok(())
}
pub fn remove_dir_all(path: impl AsRef<Path>) -> io::Result<()> {
    remove_tree_with(path.as_ref(), |p| fs::remove_dir_all(p))
}
fn remove_tree_with(path: &Path, remove: impl FnOnce(&Path) -> io::Result<()>) -> io::Result<()> {
    let Some(b) = budget_for(path)? else {
        return remove(path);
    };
    let key = resolved(path)?;
    let mut s = lock(&b.0.state)?;
    let names = s
        .entries
        .keys()
        .filter(|p| p.starts_with(&key))
        .cloned()
        .collect::<Vec<_>>();
    if names.iter().any(|p| s.entries[p].writers != 0) {
        return Err(invalid("cannot remove an active output writer"));
    }
    let result = remove(path);
    for p in names {
        match fs::symlink_metadata(&p) {
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                if let Some(e) = s.entries.remove(&p) {
                    s.used -= e.bytes;
                }
            }
            Ok(m) if m.is_file() && !reparse(&m) && m.len() == s.entries[&p].bytes => (),
            _ => s.poisoned = true,
        }
    }
    result
}

#[cfg(test)]
mod tests;
