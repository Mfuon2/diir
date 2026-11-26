# Architecture

## Approved decisions

- Product: **Diir**, targeting macOS, Linux (Wayland and X11), and Windows.
- Language and rendering: Rust + GPUI, using the exact published GPUI 0.2.2 API.
- Default appearance: Frosted Glass; alternatives: Dracula, Monokai, and Windows Explorer.
- Compact components, clean contrast, responsiveness, and bounded resource use take priority.
- Scope: application/appearance foundation plus the EX-001-T1 core navigation contract.
  Live browsing, focus integration, and visible navigation errors belong to EX-001-T2.

## Boundaries

| Layer | Owns | Must not own |
| --- | --- | --- |
| `diir-core` | Preferences, semantic design tokens, native navigation state/resolution | GPUI, OS window handles, rendering |
| App shell | Entities, action routing, window lifecycle, UI composition | Filesystem algorithms or long-running work |
| Components | Small presentation functions and explicit callbacks | Global service access or hidden I/O |
| Platform boundary | Blur policy and future OS-specific adapters | Product logic duplicated by OS |

The app depends on the core, never the reverse. Additional crates and abstractions
should follow actual requirements, rather than speculative plugin/service scaffolding.
Future operations should publish bounded progress/state updates from background
tasks to GPUI entities. Rendering consumes ready state only. The core navigation contract is documented in
[navigation.md](navigation.md): pure request/history transitions are separate from
the synchronous filesystem resolver, which must run on a background worker. Each
pane/tab owns its own state; opaque owner/generation tokens reject stale and
foreign completions. Logical invalidation does not interrupt filesystem calls.

## Appearance

- `ThemeId` resolves to semantic tokens; components do not choose their own palettes.
- Only the outer window tint can be translucent (208/255 alpha). The appearance-preview column exposes the native backdrop; content and controls stay opaque. Frosted Glass text and borders use brighter tokens to preserve contrast over light backdrops.
- Native blur is requested through GPUI on macOS and Windows; Linux deliberately uses an opaque fallback.
- Settings → Appearance holds theme and reduced-transparency choices. Menu navigation state lives in `diir-core`; GPUI event handling and the popup live in `diir-app`.
- Windows Explorer switches to an opaque light palette and an Explorer-style shell, including navigation, address/command bars, column headers, and squared controls. These remain appearance-only components.
- GPUI 0.2.2 does not expose a blur-radius control. Stronger frosting comes from the outer tint; native blur is not stacked or emulated.
- The reduced-transparency control overrides the glass request. Solid themes always use opaque windows.
- Theme switches notify the root entity once; no polling, shader loop, or background animation.
- Automated contrast tests cover body/muted/accent text and control boundaries, including glass compositing over white and black.
- Token checks do not prove pixel-level accessibility or correct compositor behavior; native smoke tests remain required.

## State and lifecycle

The root entity owns a small session `Preferences` value and its focus handle.
Theme and transparency actions update that state and request redraw. Native
titlebars retain OS window controls. Closing the only window exits the process.
Session preferences reset on restart; persistence and system accessibility-setting
integration remain future work.

## Dependency policy

GPUI is pre-1.0. Use the published pinned release and its documentation/source,
not assumptions from the upstream main branch. Upgrade in an explicit change with
native matrix checks and a theme/window smoke test. Keep the dependency surface
small. The repository owner delegated the licensing choice for distribution;
Diir uses Apache-2.0. Dependencies retain their own licenses.
