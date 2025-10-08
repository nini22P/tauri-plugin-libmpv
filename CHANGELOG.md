# Changelog

## v0.2.2

- Simplify property getter implementations using macros.
- Improve error handling.

## v0.2.1

- fix memory leak.

## v0.2.0

- Moved to `tauri-plugin-libmpv`.
- Re-license to LGPL-2.1.

## v0.1.1

- Remove unused rendering modes.

## v0.1.0

- Replace the `libmpv2` dependency with a custom `libmpv-sys` binding.
- **BREAKING:** Overhauled the event type definitions to refine and update parameter structures. **Users must adapt event handlers to the new parameter layout.**
- Set the `LC_NUMERIC` on setup.
- Enhance type inference for `observeProperties` and `getProperty`.
- Allow overriding `wid` option.

## v0.0.1

- Frist release.
