# Sprint 12: Optimization & Release

## Sprint Goal
Optimize performance, fix remaining bugs, complete documentation, and prepare the game for release across multiple platforms including web deployment.

## Deliverables

### 1. Performance Optimization
- [ ] Profile CPU and GPU usage
- [ ] Optimize render batching
- [ ] Implement LOD system
- [ ] Reduce memory allocations
- [ ] Asset compression

### 2. Build & Deployment
- [ ] WASM build optimization
- [ ] Native build configuration
- [ ] CI/CD pipeline setup
- [ ] Asset bundling
- [ ] Version management

### 3. Bug Fixes & Polish
- [ ] Fix all known bugs
- [ ] Edge case handling
- [ ] Error recovery
- [ ] Graceful degradation
- [ ] Final visual tweaks

### 4. Documentation
- [ ] Complete README
- [ ] API documentation
- [ ] Level design guide
- [ ] Contributing guidelines
- [ ] License files

### 5. Release Preparation
- [ ] Create release builds
- [ ] Package distributions
- [ ] Upload to platforms
- [ ] Marketing materials
- [ ] Launch checklist

## Technical Specifications

### Performance Profiling
```rust
use bevy::diagnostic::{
    DiagnosticsStore, 
    FrameTimeDiagnosticsPlugin,
    LogDiagnosticsPlugin,
};

pub struct PerformanceProfiler;

impl PerformanceProfiler {
    pub fn measure_systems(app: &mut App) {
        app.add_plugins((
            FrameTimeDiagnosticsPlugin,
            LogDiagnosticsPlugin::default(),
        ));
        
        #[cfg(debug_assertions)]
        app.add_systems(Update, profile_expensive_systems);
    }
}

fn profile_expensive_systems(
    diagnostics: Res<DiagnosticsStore>,
    time: Res<Time>,
) {
    if let Some(fps) = diagnostics.get(&FrameTimeDiagnosticsPlugin::FPS) {
        if let Some(average) = fps.average() {
            if average < 55.0 {
                warn!("Low FPS detected: {:.1}", average);
                // Log system timings
            }
        }
    }
}
```

### WASM Optimization
```toml
# Cargo.toml profile for WASM
[profile.wasm-release]
inherits = "release"
opt-level = "z"     # Optimize for size
lto = true          # Link-time optimization
codegen-units = 1   # Single codegen unit
strip = true        # Strip symbols
panic = "abort"     # Smaller panic handler

[dependencies.bevy]
version = "0.14"
default-features = false
features = [
    "bevy_asset",
    "bevy_winit",
    "bevy_core_pipeline",
    "bevy_render",
    "bevy_sprite",
    "webgl2",       # Use WebGL2 for web
    "bevy_ui",
    "bevy_text",
]
```

### Build Script
```rust
// build.rs
use std::env;

fn main() {
    let target = env::var("TARGET").unwrap();
    
    if target.contains("wasm") {
        println!("cargo:rustc-env=CARGO_CFG_TARGET_WASM=1");
        
        // WASM-specific optimizations
        println!("cargo:rustc-link-arg=--no-entry");
        println!("cargo:rustc-link-arg=--export-dynamic");
        println!("cargo:rustc-link-arg=-zstack-size=1048576");
    }
    
    // Embed git version
    let output = std::process::Command::new("git")
        .args(&["rev-parse", "--short", "HEAD"])
        .output()
        .unwrap();
    let git_hash = String::from_utf8(output.stdout).unwrap();
    println!("cargo:rustc-env=GIT_HASH={}", git_hash);
}
```

### Asset Optimization
```rust
pub struct AssetOptimizer;

impl AssetOptimizer {
    pub fn compress_textures() {
        // Convert to compressed formats
        // PNG → WebP for smaller size
        // Use basis-universal for GPU compression
    }
    
    pub fn optimize_audio() {
        // Convert to Ogg Vorbis
        // Reduce sample rate where appropriate
        // Compress with quality level 6
    }
    
    pub fn bundle_assets() -> AssetBundle {
        AssetBundle {
            textures: compress_and_pack_textures(),
            audio: compress_and_pack_audio(),
            levels: minify_level_data(),
            shaders: precompile_shaders(),
        }
    }
}
```

### Memory Optimization
```rust
pub fn configure_allocator() {
    #[cfg(target_arch = "wasm32")]
    {
        // Use wee_alloc for smaller WASM binary
        #[global_allocator]
        static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;
    }
    
    #[cfg(not(target_arch = "wasm32"))]
    {
        // Use mimalloc for better performance
        #[global_allocator]
        static ALLOC: mimalloc::MiMalloc = mimalloc::MiMalloc;
    }
}

// Object pooling for frequently allocated objects
pub struct ObjectPools {
    particles: Pool<Particle>,
    trail_points: Pool<TrailPoint>,
    collision_events: Pool<CollisionEvent>,
}
```

### Error Handling
```rust
pub enum GameError {
    AssetLoadFailed(String),
    AudioInitFailed(String),
    RenderError(String),
    SaveLoadError(String),
}

pub fn setup_error_handling(app: &mut App) {
    app.add_systems(Update, handle_errors);
    
    // Panic handler for graceful degradation
    std::panic::set_hook(Box::new(|info| {
        error!("Game panic: {}", info);
        
        #[cfg(target_arch = "wasm32")]
        web_sys::console::error_1(&format!("Panic: {}", info).into());
        
        // Try to save game state before exit
        if let Ok(state) = save_emergency_state() {
            info!("Emergency save successful: {:?}", state);
        }
    }));
}
```

### CI/CD Configuration
```yaml
# .github/workflows/release.yml
name: Release

on:
  push:
    tags:
      - 'v*'

jobs:
  build-native:
    strategy:
      matrix:
        os: [ubuntu-latest, windows-latest, macos-latest]
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo build --release
      - uses: actions/upload-artifact@v4
        with:
          name: ${{ matrix.os }}-build
          path: target/release/ball_matcher*

  build-wasm:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: |
          cargo install wasm-bindgen-cli
          cargo build --target wasm32-unknown-unknown --profile wasm-release
          wasm-bindgen --out-dir web --target web target/wasm32-unknown-unknown/wasm-release/ball_matcher.wasm
      - run: |
          # Optimize WASM
          wasm-opt -Oz -o web/ball_matcher_opt.wasm web/ball_matcher_bg.wasm
      - uses: actions/upload-artifact@v4
        with:
          name: wasm-build
          path: web/
```

### Platform-Specific Features
```rust
#[cfg(target_arch = "wasm32")]
pub fn configure_web_platform(app: &mut App) {
    app.add_plugins(bevy_web_fullscreen::FullscreenPlugin);
    
    // Prevent right-click context menu
    app.add_systems(Startup, |mut windows: Query<&mut Window>| {
        if let Ok(mut window) = windows.get_single_mut() {
            window.prevent_default_event_handling = true;
        }
    });
    
    // Add web-specific features
    app.add_systems(Update, handle_web_visibility);
}

#[cfg(not(target_arch = "wasm32"))]
pub fn configure_native_platform(app: &mut App) {
    app.add_plugins(bevy_embedded_assets::EmbeddedAssetPlugin);
    
    // Steam integration (if applicable)
    #[cfg(feature = "steam")]
    app.add_plugins(SteamIntegrationPlugin);
}
```

## Release Checklist

### Pre-Release
- [ ] All tests passing
- [ ] No compiler warnings
- [ ] Performance targets met (60 FPS)
- [ ] Memory usage < 500MB
- [ ] WASM build < 10MB
- [ ] All assets licensed properly

### Documentation
- [ ] README.md complete
- [ ] CHANGELOG.md updated
- [ ] API docs generated
- [ ] Examples working
- [ ] Screenshots/GIFs created

### Platform Builds
- [ ] Windows x64 tested
- [ ] Linux x64 tested
- [ ] macOS ARM64 tested
- [ ] WASM tested in Chrome/Firefox/Safari
- [ ] Mobile web tested

### Distribution
- [ ] GitHub release created
- [ ] Itch.io page ready
- [ ] Web hosting configured
- [ ] Analytics integrated

## Performance Targets

| Metric | Target | Acceptable |
|--------|--------|------------|
| FPS (Native) | 120 | 60 |
| FPS (WASM) | 60 | 30 |
| Load Time | < 3s | < 5s |
| Memory Usage | < 300MB | < 500MB |
| WASM Size | < 5MB | < 10MB |
| Audio Latency | < 20ms | < 50ms |

## Known Issues & Solutions

### WASM Audio Delay
- Pre-load audio on user interaction
- Use Web Audio API directly for critical sounds

### Mobile Performance
- Reduce particle count
- Lower resolution textures
- Simplified shaders

### Safari Compatibility
- Test WebGL2 fallback
- Polyfill for missing features

## Success Criteria

- ✅ All platforms build successfully
- ✅ Performance targets achieved
- ✅ No critical bugs remain
- ✅ Documentation complete
- ✅ Release builds under size limits
- ✅ Playable on all target platforms

## Definition of Done

- [ ] All optimizations implemented
- [ ] Builds automated via CI/CD
- [ ] Performance profiled and acceptable
- [ ] Documentation finalized
- [ ] Release notes written
- [ ] Marketing materials prepared
- [ ] Game published to platforms
- [ ] Post-launch monitoring setup

## Post-Launch Plans

### Week 1
- Monitor crash reports
- Gather player feedback
- Hot-fix critical issues

### Month 1
- Analyze gameplay metrics
- Plan content updates
- Community engagement

### Future Updates
- Level editor improvements
- Workshop/sharing support
- Additional level packs
- Seasonal events

---

## Final Notes

This sprint marks the transition from development to release. The focus shifts from adding features to ensuring quality, performance, and stability across all platforms.

Key priorities:
1. **Stability**: No crashes or game-breaking bugs
2. **Performance**: Smooth experience on all platforms
3. **Polish**: Professional presentation
4. **Documentation**: Clear and comprehensive
5. **Distribution**: Easy to access and play

The game should feel complete, polished, and ready for players to enjoy.
