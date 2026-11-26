//! Native directory resolution and isolated navigation state.
//!
//! [`resolve_directory`] performs blocking filesystem I/O. Call it on a background
//! worker, never from a render method or UI event handler. State transitions do
//! not perform filesystem I/O; obsolete completions are ignored logically.
use std::{
    collections::VecDeque,
    error::Error,
    fmt, fs, io,
    num::NonZeroUsize,
    path::{Component, Path, PathBuf},
    sync::Arc,
};

/// A physically resolved, absolute directory that was openable when validated.
/// Later access can still fail if the filesystem or its permissions change.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DirectoryLocation {
    path: PathBuf,
}

impl DirectoryLocation {
    pub fn path(&self) -> &Path {
        &self.path
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NavigationErrorKind {
    EmptyInput,
    InvalidBase,
    AmbiguousPath,
    NotDirectory,
    Io(io::ErrorKind),
}

/// A recoverable error with the original native attempted path and I/O source.
#[derive(Debug)]
pub struct NavigationError {
    attempted: PathBuf,
    kind: NavigationErrorKind,
    source: Option<io::Error>,
}

impl NavigationError {
    pub fn from_io(attempted: PathBuf, source: io::Error) -> Self {
        Self {
            attempted,
            kind: NavigationErrorKind::Io(source.kind()),
            source: Some(source),
        }
    }

    fn invalid(attempted: &Path, kind: NavigationErrorKind) -> Self {
        Self {
            attempted: attempted.to_path_buf(),
            kind,
            source: None,
        }
    }

    pub fn attempted(&self) -> &Path {
        &self.attempted
    }

    pub fn kind(&self) -> NavigationErrorKind {
        self.kind
    }

    pub fn io_error(&self) -> Option<&io::Error> {
        self.source.as_ref()
    }
}

impl fmt::Display for NavigationError {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(output, "Cannot open '{}': ", self.attempted.display())?;
        match self.kind {
            NavigationErrorKind::EmptyInput => write!(output, "enter a directory path")?,
            NavigationErrorKind::InvalidBase => {
                write!(output, "the navigation base must be an absolute path")?;
            }
            NavigationErrorKind::AmbiguousPath => {
                write!(
                    output,
                    "use a fully qualified path or an ordinary relative path"
                )?;
            }
            NavigationErrorKind::NotDirectory => write!(output, "the location is not a directory")?,
            NavigationErrorKind::Io(_) => match &self.source {
                Some(source) => write!(output, "{source}")?,
                None => write!(output, "the directory is unavailable")?,
            },
        }
        write!(output, ". Choose another directory or try again.")
    }
}

impl Error for NavigationError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.source.as_ref().map(|source| source as &dyn Error)
    }
}

fn candidate_path(base: &Path, input: &Path) -> Result<PathBuf, NavigationError> {
    if input.as_os_str().is_empty() {
        return Err(NavigationError::invalid(
            input,
            NavigationErrorKind::EmptyInput,
        ));
    }
    if !base.is_absolute() {
        return Err(NavigationError::invalid(
            input,
            NavigationErrorKind::InvalidBase,
        ));
    }
    if input.is_absolute() {
        return Ok(input.to_path_buf());
    }
    // On Windows, C:child and \child depend on process/drive context. Unix does
    // not interpret those spellings as Windows paths and keeps native semantics.
    if input.has_root() || matches!(input.components().next(), Some(Component::Prefix(_))) {
        return Err(NavigationError::invalid(
            input,
            NavigationErrorKind::AmbiguousPath,
        ));
    }
    Ok(base.join(input))
}

/// Resolve and open a local directory without enumerating its entries.
///
/// Relative paths use the explicitly supplied absolute base. Canonicalization
/// follows symlinks and keeps native OS path data, including Windows prefixes.
/// Joining relative input to a Windows verbatim base normalizes dot components
/// before filesystem access, following Rust's native PathBuf behavior.
/// This is a synchronous worker boundary, not a UI-safe state operation.
pub fn resolve_directory(base: &Path, input: &Path) -> Result<DirectoryLocation, NavigationError> {
    let attempted = candidate_path(base, input)?;
    let path = fs::canonicalize(&attempted)
        .map_err(|source| NavigationError::from_io(attempted.clone(), source))?;
    let metadata = fs::metadata(&path)
        .map_err(|source| NavigationError::from_io(attempted.clone(), source))?;
    if !metadata.is_dir() {
        return Err(NavigationError::invalid(
            &attempted,
            NavigationErrorKind::NotDirectory,
        ));
    }
    let _directory =
        fs::read_dir(&path).map_err(|source| NavigationError::from_io(attempted, source))?;
    Ok(DirectoryLocation { path })
}

#[derive(Debug)]
pub enum NavigationIntent {
    Open(PathBuf),
    Back,
    Forward,
    Parent,
    Retry,
}

#[derive(Clone, Copy, Debug)]
enum Transition {
    NewLocation,
    Back,
    Forward,
}

/// A cloneable worker request. Private tokens bind a result to its originating
/// state instance and exact generation. Tokens are opaque, and a completion is
/// accepted only while that exact request is pending.
#[derive(Clone, Debug)]
pub struct NavigationRequest {
    owner: Arc<()>,
    generation: Arc<()>,
    base: PathBuf,
    candidate: PathBuf,
    transition: Transition,
}

impl NavigationRequest {
    pub fn base(&self) -> &Path {
        &self.base
    }

    pub fn candidate(&self) -> &Path {
        &self.candidate
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompletionOutcome {
    Applied,
    Ignored,
}

#[derive(Debug)]
struct RetryOperation {
    base: PathBuf,
    candidate: PathBuf,
    transition: Transition,
}

/// One pane/tab's navigation context. Own a separate value for each context.
/// State is deliberately not Clone, so cloning cannot duplicate request ownership.
#[derive(Debug)]
pub struct NavigationState {
    owner: Arc<()>,
    current: DirectoryLocation,
    back: VecDeque<DirectoryLocation>,
    forward: VecDeque<DirectoryLocation>,
    history_limit: NonZeroUsize,
    pending: Option<NavigationRequest>,
    error: Option<NavigationError>,
    retry: Option<RetryOperation>,
}

impl NavigationState {
    pub fn new(initial: DirectoryLocation, history_limit: NonZeroUsize) -> Self {
        Self {
            owner: Arc::new(()),
            current: initial,
            back: VecDeque::new(),
            forward: VecDeque::new(),
            history_limit,
            pending: None,
            error: None,
            retry: None,
        }
    }

    pub fn current(&self) -> &DirectoryLocation {
        &self.current
    }

    pub fn error(&self) -> Option<&NavigationError> {
        self.error.as_ref()
    }

    pub fn pending(&self) -> Option<&NavigationRequest> {
        self.pending.as_ref()
    }

    pub fn can_go_back(&self) -> bool {
        !self.back.is_empty()
    }

    pub fn can_go_forward(&self) -> bool {
        !self.forward.is_empty()
    }

    /// Start or supersede a request without doing I/O. Unavailable history/root/
    /// retry endpoints are true no-ops and do not disturb an existing request.
    pub fn begin(&mut self, intent: NavigationIntent) -> Option<NavigationRequest> {
        let (base, candidate, transition) = match intent {
            NavigationIntent::Open(candidate) => (
                self.current.path.clone(),
                candidate,
                Transition::NewLocation,
            ),
            NavigationIntent::Back => (
                self.current.path.clone(),
                self.back.back()?.path.clone(),
                Transition::Back,
            ),
            NavigationIntent::Forward => (
                self.current.path.clone(),
                self.forward.back()?.path.clone(),
                Transition::Forward,
            ),
            NavigationIntent::Parent => (
                self.current.path.clone(),
                self.current.path.parent()?.to_path_buf(),
                Transition::NewLocation,
            ),
            NavigationIntent::Retry => {
                let retry = self.retry.as_ref()?;
                (
                    retry.base.clone(),
                    retry.candidate.clone(),
                    retry.transition,
                )
            }
        };
        let request = NavigationRequest {
            owner: self.owner.clone(),
            generation: Arc::new(()),
            base,
            candidate,
            transition,
        };
        self.pending = Some(request.clone());
        self.error = None;
        self.retry = None;
        Some(request)
    }

    /// Apply only the latest owned request. Supply the worker result from
    /// resolve_directory(request.base(), request.candidate()).
    pub fn complete(
        &mut self,
        request: NavigationRequest,
        result: Result<DirectoryLocation, NavigationError>,
    ) -> CompletionOutcome {
        let current_request = self.pending.as_ref().is_some_and(|pending| {
            Arc::ptr_eq(&self.owner, &request.owner)
                && Arc::ptr_eq(&pending.generation, &request.generation)
        });
        if !current_request {
            return CompletionOutcome::Ignored;
        }
        self.pending = None;
        match result {
            Ok(location) => {
                self.error = None;
                self.retry = None;
                if location != self.current {
                    let previous = std::mem::replace(&mut self.current, location);
                    match request.transition {
                        Transition::NewLocation => {
                            Self::push_history(&mut self.back, previous, self.history_limit);
                            self.forward.clear();
                        }
                        Transition::Back => {
                            self.back.pop_back();
                            Self::push_history(&mut self.forward, previous, self.history_limit);
                        }
                        Transition::Forward => {
                            self.forward.pop_back();
                            Self::push_history(&mut self.back, previous, self.history_limit);
                        }
                    }
                }
            }
            Err(error) => {
                self.error = Some(error);
                self.retry = Some(RetryOperation {
                    base: request.base,
                    candidate: request.candidate,
                    transition: request.transition,
                });
            }
        }
        CompletionOutcome::Applied
    }

    fn push_history(
        history: &mut VecDeque<DirectoryLocation>,
        location: DirectoryLocation,
        limit: NonZeroUsize,
    ) {
        if history.len() == limit.get() {
            history.pop_front();
        }
        history.push_back(location);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        ffi::OsString,
        sync::atomic::{AtomicU64, Ordering},
        time::{SystemTime, UNIX_EPOCH},
    };

    fn location(name: &str) -> DirectoryLocation {
        // State-machine fixtures deliberately avoid filesystem I/O.
        DirectoryLocation {
            path: std::env::temp_dir().join("diir-state-fixture").join(name),
        }
    }

    fn state() -> NavigationState {
        NavigationState::new(location("a"), NonZeroUsize::new(8).expect("nonzero limit"))
    }

    fn navigate(state: &mut NavigationState, intent: NavigationIntent, target: &str) {
        let request = state.begin(intent).expect("available navigation");
        assert_eq!(
            state.complete(request, Ok(location(target))),
            CompletionOutcome::Applied
        );
        assert_eq!(state.current(), &location(target));
    }

    fn open(state: &mut NavigationState, target: &str) {
        navigate(state, NavigationIntent::Open(location(target).path), target);
    }

    fn denied(request: &NavigationRequest) -> NavigationError {
        NavigationError::from_io(
            request.candidate().to_path_buf(),
            io::Error::new(io::ErrorKind::PermissionDenied, "access denied fixture"),
        )
    }

    #[test]
    fn back_forward_and_new_navigation_commit_only_after_success() {
        let mut state = state();
        assert!(state.begin(NavigationIntent::Back).is_none());
        assert!(state.begin(NavigationIntent::Forward).is_none());
        open(&mut state, "b");
        open(&mut state, "c");
        navigate(&mut state, NavigationIntent::Back, "b");
        assert!(state.can_go_forward());
        let request = state
            .begin(NavigationIntent::Open(location("d").path))
            .unwrap();
        assert_eq!(state.current(), &location("b"));
        assert!(state.can_go_forward());
        state.complete(request, Ok(location("d")));
        assert!(!state.can_go_forward());
        navigate(&mut state, NavigationIntent::Back, "b");
        navigate(&mut state, NavigationIntent::Back, "a");
        assert!(!state.can_go_back());
        navigate(&mut state, NavigationIntent::Forward, "b");
        navigate(&mut state, NavigationIntent::Forward, "d");
        assert!(!state.can_go_forward());
    }

    #[test]
    fn foreign_requests_cannot_change_another_context() {
        let mut first = state();
        let mut second = state();
        let foreign = first
            .begin(NavigationIntent::Open(location("b").path))
            .unwrap();
        let own = second
            .begin(NavigationIntent::Open(location("c").path))
            .unwrap();
        assert_eq!(
            second.complete(foreign.clone(), Ok(location("b"))),
            CompletionOutcome::Ignored
        );
        assert_eq!(second.current(), &location("a"));
        assert!(second.pending().is_some());
        assert_eq!(
            second.complete(own, Ok(location("c"))),
            CompletionOutcome::Applied
        );
        assert_eq!(first.current(), &location("a"));
        assert_eq!(
            first.complete(foreign, Ok(location("b"))),
            CompletionOutcome::Applied
        );
        assert_eq!(second.current(), &location("c"));
        assert!(first.can_go_back());
        assert!(second.can_go_back());
    }

    #[test]
    fn superseded_success_and_failure_are_ignored_without_disturbing_pending() {
        let mut state = state();
        let old = state
            .begin(NavigationIntent::Open(location("b").path))
            .unwrap();
        let latest = state
            .begin(NavigationIntent::Open(location("c").path))
            .unwrap();
        let error = denied(&old);
        assert_eq!(
            state.complete(old.clone(), Err(error)),
            CompletionOutcome::Ignored
        );
        assert!(state.error().is_none());
        assert_eq!(state.pending().unwrap().candidate(), location("c").path());
        assert_eq!(
            state.complete(old, Ok(location("b"))),
            CompletionOutcome::Ignored
        );
        assert_eq!(state.current(), &location("a"));
        assert_eq!(
            state.complete(latest.clone(), Ok(location("c"))),
            CompletionOutcome::Applied
        );
        assert!(state.pending().is_none());
        assert_eq!(
            state.complete(latest, Ok(location("d"))),
            CompletionOutcome::Ignored
        );
        assert_eq!(state.current(), &location("c"));
    }

    #[test]
    fn failed_open_retains_history_and_retry_preserves_relative_base() {
        let mut state = state();
        open(&mut state, "b");
        open(&mut state, "c");
        navigate(&mut state, NavigationIntent::Back, "b");
        let request = state
            .begin(NavigationIntent::Open(PathBuf::from("child")))
            .unwrap();
        let error = denied(&request);
        state.complete(request, Err(error));
        assert_eq!(state.current(), &location("b"));
        assert!(state.can_go_back());
        assert!(state.can_go_forward());
        let error = state.error().unwrap();
        assert_eq!(
            error.kind(),
            NavigationErrorKind::Io(io::ErrorKind::PermissionDenied)
        );
        assert!(error.io_error().is_some());
        assert!(Error::source(error).is_some());
        assert!(
            error
                .to_string()
                .contains("Choose another directory or try again")
        );
        let retry = state.begin(NavigationIntent::Retry).unwrap();
        assert_eq!(retry.base(), location("b").path());
        assert_eq!(retry.candidate(), Path::new("child"));
        assert!(state.error().is_none());
        state.complete(retry, Ok(location("d")));
        assert!(!state.can_go_forward());
        navigate(&mut state, NavigationIntent::Back, "b");
        navigate(&mut state, NavigationIntent::Back, "a");
    }

    #[test]
    fn failed_back_and_forward_can_retry_without_losing_history_position() {
        for intent in [NavigationIntent::Back, NavigationIntent::Forward] {
            let mut state = state();
            open(&mut state, "b");
            open(&mut state, "c");
            navigate(&mut state, NavigationIntent::Back, "b");
            let back = matches!(intent, NavigationIntent::Back);
            let request = state.begin(intent).unwrap();
            let error = denied(&request);
            state.complete(request, Err(error));
            assert_eq!(state.current(), &location("b"));
            assert!(state.can_go_back());
            assert!(state.can_go_forward());
            let retry = state.begin(NavigationIntent::Retry).unwrap();
            assert_eq!(
                retry.candidate(),
                location(if back { "a" } else { "c" }).path()
            );
            state.complete(retry, Ok(location(if back { "a" } else { "c" })));
            if back {
                navigate(&mut state, NavigationIntent::Forward, "b");
                navigate(&mut state, NavigationIntent::Forward, "c");
            } else {
                navigate(&mut state, NavigationIntent::Back, "b");
                navigate(&mut state, NavigationIntent::Back, "a");
            }
        }
    }

    #[test]
    fn same_location_is_a_noop_for_history_and_forward_context() {
        let mut state = state();
        open(&mut state, "b");
        open(&mut state, "c");
        navigate(&mut state, NavigationIntent::Back, "b");
        open(&mut state, "b");
        assert!(state.can_go_forward());
        navigate(&mut state, NavigationIntent::Back, "a");
        assert!(!state.can_go_back());
    }

    #[test]
    fn oldest_history_is_evicted_at_the_explicit_capacity() {
        let mut state = NavigationState::new(location("a"), NonZeroUsize::new(2).unwrap());
        for target in ["b", "c", "d"] {
            open(&mut state, target);
        }
        navigate(&mut state, NavigationIntent::Back, "c");
        navigate(&mut state, NavigationIntent::Back, "b");
        assert!(state.begin(NavigationIntent::Back).is_none());
        navigate(&mut state, NavigationIntent::Forward, "c");
        navigate(&mut state, NavigationIntent::Forward, "d");
        assert!(state.begin(NavigationIntent::Forward).is_none());
    }

    #[test]
    fn unavailable_intents_do_not_cancel_a_pending_request() {
        let mut state = state();
        let request = state
            .begin(NavigationIntent::Open(location("b").path))
            .unwrap();
        assert!(state.begin(NavigationIntent::Back).is_none());
        assert!(state.begin(NavigationIntent::Forward).is_none());
        assert!(state.begin(NavigationIntent::Retry).is_none());
        assert_eq!(
            state.complete(request, Ok(location("b"))),
            CompletionOutcome::Applied
        );
    }

    #[test]
    fn parent_request_and_root_boundary_use_native_path_components() {
        let mut state = state();
        let request = state.begin(NavigationIntent::Parent).unwrap();
        assert_eq!(request.candidate(), location("a").path().parent().unwrap());
        let root = state
            .current()
            .path()
            .ancestors()
            .last()
            .unwrap()
            .to_path_buf();
        let mut state = NavigationState::new(
            DirectoryLocation { path: root },
            NonZeroUsize::new(2).unwrap(),
        );
        assert!(state.begin(NavigationIntent::Parent).is_none());
    }

    struct TempDirectory(PathBuf);

    impl TempDirectory {
        fn new() -> Self {
            static NEXT_ID: AtomicU64 = AtomicU64::new(0);
            let time = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "diir-navigation-{}-{time}-{}",
                std::process::id(),
                NEXT_ID.fetch_add(1, Ordering::Relaxed),
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TempDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn resolves_absolute_relative_and_nested_os_traversal_paths() {
        let fixture = TempDirectory::new();
        fs::create_dir_all(fixture.0.join("first").join("child")).unwrap();
        fs::create_dir(fixture.0.join("second")).unwrap();
        let relative = Path::new("first").join("child");
        let resolved = resolve_directory(&fixture.0, &relative).unwrap();
        assert_eq!(
            resolved.path(),
            fs::canonicalize(fixture.0.join(&relative)).unwrap()
        );
        assert_eq!(
            resolve_directory(&fixture.0, resolved.path()).unwrap(),
            resolved
        );
        let traversed = Path::new("first")
            .join("child")
            .join("..")
            .join("..")
            .join("second")
            .join(".");
        assert_eq!(
            resolve_directory(&fixture.0, &traversed).unwrap().path(),
            fs::canonicalize(fixture.0.join("second")).unwrap()
        );
    }

    #[test]
    fn rejects_missing_file_empty_and_relative_base_with_recoverable_messages() {
        let fixture = TempDirectory::new();
        fs::write(fixture.0.join("file"), b"fixture").unwrap();
        let cases = [
            (
                Path::new("missing"),
                NavigationErrorKind::Io(io::ErrorKind::NotFound),
            ),
            (Path::new("file"), NavigationErrorKind::NotDirectory),
            (Path::new(""), NavigationErrorKind::EmptyInput),
        ];
        for (input, expected) in cases {
            let error = resolve_directory(&fixture.0, input).unwrap_err();
            assert_eq!(error.kind(), expected);
            assert!(error.to_string().contains("try again"));
        }
        let error = resolve_directory(Path::new("relative-base"), &fixture.0).unwrap_err();
        assert_eq!(error.kind(), NavigationErrorKind::InvalidBase);
    }

    #[test]
    fn preserves_native_file_names() {
        let fixture = TempDirectory::new();
        let name = OsString::from("native-雪");
        fs::create_dir(fixture.0.join(&name)).unwrap();
        let resolved = resolve_directory(&fixture.0, Path::new(&name)).unwrap();
        assert_eq!(resolved.path().file_name(), Some(name.as_os_str()));
    }

    #[cfg(unix)]
    #[test]
    fn native_non_utf8_candidates_and_failures_keep_the_original_bytes() {
        use std::os::unix::ffi::OsStringExt;
        let fixture = TempDirectory::new();
        let name = OsString::from_vec(vec![b'n', 0xff, b'm']);
        let candidate = candidate_path(&fixture.0, Path::new(&name)).unwrap();
        assert_eq!(candidate.file_name(), Some(name.as_os_str()));
        let error = resolve_directory(&fixture.0, Path::new(&name)).unwrap_err();
        assert_eq!(error.attempted(), candidate);
        assert!(error.io_error().is_some());
        assert!(error.to_string().contains("try again"));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn linux_resolves_existing_non_utf8_directory_names() {
        use std::os::unix::ffi::OsStringExt;
        let fixture = TempDirectory::new();
        let name = OsString::from_vec(vec![b'n', 0xff, b'm']);
        fs::create_dir(fixture.0.join(&name)).unwrap();
        let resolved = resolve_directory(&fixture.0, Path::new(&name)).unwrap();
        assert_eq!(resolved.path().file_name(), Some(name.as_os_str()));
    }

    #[cfg(unix)]
    #[test]
    fn symlinks_resolve_to_the_physical_directory_without_lexical_normalization() {
        use std::os::unix::fs::symlink;
        let fixture = TempDirectory::new();
        fs::create_dir_all(fixture.0.join("physical").join("child")).unwrap();
        symlink(
            fixture.0.join("physical").join("child"),
            fixture.0.join("alias"),
        )
        .unwrap();
        let resolved = resolve_directory(&fixture.0, Path::new("alias")).unwrap();
        assert_eq!(
            resolved.path(),
            fs::canonicalize(fixture.0.join("physical").join("child")).unwrap()
        );
        let parent = resolve_directory(&fixture.0, &Path::new("alias").join("..")).unwrap();
        assert_eq!(
            parent.path(),
            fs::canonicalize(fixture.0.join("physical")).unwrap()
        );
    }

    #[cfg(windows)]
    #[test]
    fn windows_canonical_verbatim_base_normalizes_relative_dot_components() {
        use std::path::Prefix;
        let fixture = TempDirectory::new();
        fs::create_dir(fixture.0.join("target")).unwrap();
        let base = fs::canonicalize(&fixture.0).unwrap();
        assert!(matches!(
            base.components().next(),
            Some(Component::Prefix(prefix)) if matches!(
                prefix.kind(),
                Prefix::Verbatim(_) | Prefix::VerbatimDisk(_) | Prefix::VerbatimUNC(_, _)
            )
        ));
        assert!(!fixture.0.join("alias").exists());
        let input = Path::new(r"alias\..\target");
        assert_eq!(candidate_path(&base, input).unwrap(), base.join("target"));
        assert_eq!(
            resolve_directory(&base, input).unwrap().path(),
            fs::canonicalize(fixture.0.join("target")).unwrap()
        );
    }

    #[cfg(windows)]
    #[test]
    fn windows_rejects_ambiguous_paths_and_preserves_fully_qualified_syntax() {
        let base = Path::new(r"C:\base");
        for path in [r"C:child", r"\child", r"\"] {
            assert_eq!(
                candidate_path(base, Path::new(path)).unwrap_err().kind(),
                NavigationErrorKind::AmbiguousPath
            );
        }
        for path in [
            r"C:\folder\child",
            r"\\server\share\folder",
            r"\\?\C:\folder",
        ] {
            assert_eq!(
                candidate_path(base, Path::new(path)).unwrap(),
                PathBuf::from(path)
            );
        }
        let unc = PathBuf::from(r"\\server\share\unavailable");
        let error = NavigationError::from_io(
            unc.clone(),
            io::Error::new(io::ErrorKind::NotFound, "unavailable UNC fixture"),
        );
        assert_eq!(error.attempted(), unc);
        assert!(error.to_string().contains("try again"));
    }
}
