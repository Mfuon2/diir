# Core navigation contract — EX-001-T1

This is the core-only delivery for [issue #2](https://github.com/Mfuon2/diir/issues/2).
The parent [single-pane browsing story #1](https://github.com/Mfuon2/diir/issues/1)
stays open until its remaining subtasks and acceptance criteria are complete.
[Issue #3](https://github.com/Mfuon2/diir/issues/3) owns GPUI interaction, visible
recoverable messages, and focus restoration. [Issue #4](https://github.com/Mfuon2/diir/issues/4)
owns broader integration validation. No live browsing is advertised by this change.

## Path and filesystem boundary

`resolve_directory(base, input)` accepts native `Path` values and returns a
`DirectoryLocation` or `NavigationError`. `DirectoryLocation` has no public unchecked
constructor; its read-only path is absolute and canonical when resolution succeeds.

- The base must be explicitly absolute, even when the input is absolute. There is
  no process working-directory lookup or mutation.
- An ordinary relative input is joined to that base. Absolute input is used
  directly. Empty input is rejected. Spaces are filename data, not trimmed away.
- Windows drive-relative paths such as `C:child` and root-relative paths such as
  `\child` are rejected as ambiguous. Fully qualified drive, UNC, and supported
  native-prefixed paths retain OS semantics; availability is decided by the OS.
- Canonicalization follows physical filesystem resolution, including symlinks and
  `..`; it does not lexically remove components before following links. This can
  change display spelling and produce Windows extended path prefixes.
- The resolved target must be a directory. A directory-enumeration handle is
  opened to establish initial access, then dropped without iterating the entries.
- Shell expansion, environment variables, tilde shortcuts, and provider URLs are
  not interpreted. A literal name using those characters remains native path data.

This function performs **blocking filesystem I/O**. Later app integration must
schedule it on a bounded background worker, never in rendering or UI event handlers.
It does not scan or sort the directory, list files, or persist settings.

Canonicalization/opening is a preliminary check. Permissions, mount availability,
symlinks, and directory contents can change afterwards. Later enumeration must
report its own recoverable failures. A path that the OS refuses is an error even
when its bytes can be represented in `PathBuf`.

## Independent state and request ownership

`NavigationState::new(initial, history_limit)` requires a validated starting
location and an explicit `NonZeroUsize` capacity. There is no global pane/tab
context, default startup path, hidden history-size preference, or GPUI dependency.
Each future pane/tab must own a separate state value.

1. Call `begin(NavigationIntent)` on the state. It performs no filesystem I/O.
2. Dispatch the returned `NavigationRequest` to a worker. The worker calls
   `resolve_directory(request.base(), request.candidate())`.
3. Return the same request token and its matching result to `complete`.
4. `complete` returns `Applied` for the current owned request, including an applied
   failure, and `Ignored` for a superseded, foreign, or already completed request.

Private `Arc` identities scope each request to its state owner and exact generation.
They do not use a shared mutable registry or an integer counter that can collide
across contexts or wrap around. A new available request supersedes the old one;
at most one request is pending per state. State cannot be cloned to duplicate
ownership. Worker requests can be cloned for transport, but can be applied only
once while current.

This is **logical cancellation**, not interruption of an OS call. Issue #3 must
still bound active workers/queues and manage cancellation/backpressure. The core
does not create background tasks itself.

## History and error behavior

| Intent/result | Behavior |
| --- | --- |
| Successful `Open` or `Parent` to another location | Push previous location to bounded back history; clear forward history |
| Successful `Back` / `Forward` | Move one history position and preserve the opposite direction |
| Same canonical location | Clear pending/error state without duplicating or changing history |
| Failed request | Preserve the previous location and both histories; expose a typed, displayable error |
| `Retry` after failure | Repeat the failed candidate, base, and transition, including a failed Back/Forward |
| Back/Forward endpoint, Parent at root, Retry without failure | Return no request; leave current/error/pending state untouched |
| New available request | Clear the previous error/retry context and replace the pending request |
| Stale/foreign completion | Change nothing, including the current pending request and error |

Oldest history is evicted at the caller-supplied capacity. Transitions do not clone
whole history collections or read the filesystem. The only mutable history access
is inside the state; callers get read-only location/error/pending accessors and
back/forward availability.

`NavigationError` retains the attempted native path and either a structural reason
(empty input, invalid base, ambiguous path, non-directory) or the original
`io::Error`, its kind, and source details. `Display` offers a recoverable message
ending with choosing another directory or trying again. Any lossy native-path
formatting is limited to this display message; resolution, state, and error path
data never require UTF-8. The core message is not evidence of a visible GUI error.

## Tests and limitations

Core tests cover independent context ownership, foreign/stale success and failure,
repeated completion, pending supersession, history movement/truncation/eviction,
no-op boundaries, retry, deterministic PermissionDenied errors, and native path
resolution with actual temporary nested directories and files.

Unix tests preserve non-UTF-8 bytes in candidates and recoverable error paths;
Linux additionally creates and resolves an existing non-UTF-8 directory. The local
macOS filesystem rejected creation of a `0xff` filename with `EILSEQ`, so successful
non-UTF-8 directory traversal is not claimed there. Normal-name symlink traversal
and physical `..` semantics are tested on Unix. Windows-gated tests cover native
fully qualified/UNC syntax and ambiguous relative forms without requiring an SMB
server or administrator-only symlink creation. A recoverable synthetic UNC error
checks error representation; it does not prove a successful network-share visit.

Permission-denial behavior is tested deterministically by completing a request
with an actual `io::ErrorKind::PermissionDenied` error. There is no claim of a real
permission-denied fixture on privileged CI accounts. No GUI, provider, dual-pane,
tab UI, file-row virtualization, or persistence behavior is included.

## Attribution and platform scope

The parent inventory attributes this capability to **Finder** and **Windows
Explorer**, with the original topic references [Marta navigation](https://marta.sh/docs/navigation/base/),
[Marta documentation](https://marta.sh/docs/), and [Windows File Explorer](https://support.microsoft.com/en-us/windows/experience/fileexplorer/file-explorer-in-windows).
The [source inventory](https://explorer-atlas-mfuon.l3l-lny.chatgpt.site) is provenance,
not an assertion of identical behavior or an adopted edition/pricing policy.
This contract covers local filesystem paths using the host OS. Remote providers,
cloud accounts, edition-specific integrations, and network-share availability are
not asserted by this subtask. Rust + GPUI remains the approved product stack.
