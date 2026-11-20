# incular-mobile

## Architecture and support

| Contract | Status |
| --- | --- |
| Ownership | Mobile embedder engine: view binding, surface lifecycle, touch, insets and frame pump shared by Android and iOS. |
| API class | backend; re-exports and path overrides follow the [architecture contract](https://github.com/FabortLabs/incular/blob/master/system-design/ARCHITECTURE.md). |
| Support | Partial; touch, surfaces, lifecycle and insets. Text input, accessibility bridges and native services follow the mobile spec phases. |

`MobileEngine` is the engine a native Android or iOS shell drives. It has no
FFI: `incular-android` and `incular-ios` translate native callbacks into calls
on it, and it presents through `incular-host`, like the Winit shell.

The engine binds the application's first window to the native view and
refuses further windows with a typed `unsupported` error. All methods run on
the platform main thread; `MobileEngine::frame` is called from the platform's
frame callback (Choreographer, `CADisplayLink`) and returns whether another
frame is needed.

On mobile the app's `main` does not own the event loop. `incular::run(app)`
calls `install_application`, and the shell takes the application with
`take_application`. See the [mobile spec](https://github.com/FabortLabs/incular/blob/master/system-design/MOBILE.md).
