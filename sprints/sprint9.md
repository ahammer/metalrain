# Sprint 9: UI/HUD System

## Sprint Goal
Implement the user interface layer including HUD elements, game state overlays, pause menu, and settings interface to provide players with essential information and control.

## Deliverables

### 1. UI Framework Setup
- [ ] Integrate bevy_egui for immediate mode UI
- [ ] Create UI theme and styling system
- [ ] Set up font loading pipeline
- [ ] Implement UI scaling for different resolutions
- [ ] Create reusable UI components

### 2. In-Game HUD
- [ ] Ball counter with visual indicators
- [ ] Target counter display
- [ ] Level name/number display
- [ ] Timer (if applicable)
- [ ] Score display (future-ready)

### 3. Game State Overlays
- [ ] Victory screen with stats
- [ ] Defeat screen with retry option
- [ ] Ready/countdown overlay
- [ ] Level complete summary
- [ ] Transition animations

### 4. Pause Menu
- [ ] Resume button
- [ ] Restart level option
- [ ] Settings access
- [ ] Return to main menu
- [ ] Visual game dimming

### 5. Settings Menu
- [ ] Audio volume sliders
- [ ] Graphics quality presets
- [ ] Input remapping interface
- [ ] Accessibility options
- [ ] Apply/cancel functionality

## Technical Specifications

### UI Theme Configuration
```rust
pub struct UITheme {
    // Colors
    pub primary: Color,
    pub secondary: Color,
    pub background: Color,
    pub text: Color,
    pub accent: Color,
    pub danger: Color,
    pub success: Color,
    
    // Typography
    pub font_regular: Handle<Font>,
    pub font_bold: Handle<Font>,
    pub font_size_small: f32,
    pub font_size_normal: f32,
    pub font_size_large: f32,
    pub font_size_title: f32,
    
    // Spacing
    pub padding: f32,
    pub margin: f32,
    pub corner_radius: f32,
}

impl Default for UITheme {
    fn default() -> Self {
        Self {
            primary: Color::rgb(0.2, 0.6, 1.0),
            secondary: Color::rgb(0.8, 0.8, 0.9),
            background: Color::rgba(0.05, 0.05, 0.1, 0.9),
            text: Color::WHITE,
            accent: Color::rgb(1.0, 0.8, 0.2),
            danger: Color::rgb(1.0, 0.3, 0.2),
            success: Color::rgb(0.2, 1.0, 0.3),
            font_size_normal: 16.0,
            padding: 8.0,
            margin: 12.0,
            corner_radius: 4.0,
            ..default()
        }
    }
}
```

### HUD Components
```rust
pub fn render_game_hud(
    mut egui_ctx: ResMut<EguiContext>,
    game_state: Res<GameState>,
    level_data: Res<CurrentLevel>,
    theme: Res<UITheme>,
) {
    let ctx = egui_ctx.ctx_mut();
    
    // Top bar
    egui::TopBottomPanel::top("hud_top")
        .frame(egui::Frame::none())
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                // Level info
                ui.label(
                    RichText::new(&level_data.name)
                        .size(theme.font_size_large)
                        .color(theme.text)
                );
                
                ui.separator();
                
                // Timer (if applicable)
                if let Some(time_limit) = level_data.time_limit {
                    let remaining = time_limit - game_state.play_time;
                    let color = if remaining < 10.0 { 
                        theme.danger 
                    } else { 
                        theme.text 
                    };
                    ui.label(
                        RichText::new(format_time(remaining))
                            .color(color)
                            .size(theme.font_size_large)
                    );
                }
            });
        });
    
    // Bottom bar with counters
    egui::TopBottomPanel::bottom("hud_bottom")
        .frame(egui::Frame::none())
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                // Ball counter
                render_ball_counter(ui, &game_state, &theme);
                
                ui.separator();
                
                // Target counter
                render_target_counter(ui, &game_state, &theme);
            });
        });
}

fn render_ball_counter(
    ui: &mut egui::Ui,
    state: &GameState,
    theme: &UITheme,
) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new("Balls:")
                .size(theme.font_size_normal)
        );
        
        // Visual ball indicators
        for i in 0..state.initial_balls {
            let color = if i < state.balls_remaining {
                theme.primary
            } else {
                theme.background
            };
            
            ui.add(
                egui::Label::new(
                    RichText::new("●")
                        .size(theme.font_size_large)
                        .color(color)
                )
            );
        }
        
        ui.label(
            RichText::new(format!("({})", state.balls_remaining))
                .size(theme.font_size_normal)
        );
    });
}
```

### Victory/Defeat Overlays
```rust
pub fn render_victory_overlay(
    mut egui_ctx: ResMut<EguiContext>,
    game_stats: Res<GameStats>,
    theme: Res<UITheme>,
    mut next_state: ResMut<NextState<GamePhase>>,
) {
    let ctx = egui_ctx.ctx_mut();
    
    egui::Window::new("Victory!")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .frame(create_overlay_frame(&theme))
        .show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                // Title
                ui.label(
                    RichText::new("Level Complete!")
                        .size(theme.font_size_title)
                        .color(theme.success)
                );
                
                ui.separator();
                
                // Stats
                ui.label(format!("Time: {}", format_time(game_stats.completion_time)));
                ui.label(format!("Balls Saved: {}", game_stats.balls_saved));
                ui.label(format!("Targets Destroyed: {}", game_stats.targets_destroyed));
                
                // Star rating
                render_star_rating(ui, calculate_stars(&game_stats), &theme);
                
                ui.separator();
                
                // Buttons
                ui.horizontal(|ui| {
                    if ui.button("Next Level").clicked() {
                        next_state.set(GamePhase::LoadingNext);
                    }
                    
                    if ui.button("Replay").clicked() {
                        next_state.set(GamePhase::Setup);
                    }
                    
                    if ui.button("Main Menu").clicked() {
                        next_state.set(GamePhase::MainMenu);
                    }
                });
            });
        });
}
```

### Pause Menu
```rust
pub fn render_pause_menu(
    mut egui_ctx: ResMut<EguiContext>,
    theme: Res<UITheme>,
    mut next_state: ResMut<NextState<GamePhase>>,
    mut settings_open: Local<bool>,
) {
    let ctx = egui_ctx.ctx_mut();
    
    // Darken background
    egui::Area::new("pause_overlay")
        .fixed_pos([0.0, 0.0])
        .show(ctx, |ui| {
            let screen_rect = ui.ctx().screen_rect();
            ui.painter().rect_filled(
                screen_rect,
                0.0,
                Color32::from_rgba(0, 0, 0, 180),
            );
        });
    
    if !*settings_open {
        egui::Window::new("Paused")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    if ui.button("Resume").clicked() {
                        next_state.set(GamePhase::Playing);
                    }
                    
                    if ui.button("Settings").clicked() {
                        *settings_open = true;
                    }
                    
                    if ui.button("Restart Level").clicked() {
                        next_state.set(GamePhase::Setup);
                    }
                    
                    if ui.button("Main Menu").clicked() {
                        next_state.set(GamePhase::MainMenu);
                    }
                });
            });
    } else {
        render_settings_menu(egui_ctx, theme, settings_open);
    }
}
```

### Settings Interface
```rust
pub fn render_settings_menu(
    mut egui_ctx: ResMut<EguiContext>,
    mut settings: ResMut<GameSettings>,
    theme: Res<UITheme>,
    mut open: Local<bool>,
) {
    let ctx = egui_ctx.ctx_mut();
    
    egui::Window::new("Settings")
        .open(&mut *open)
        .resizable(false)
        .show(ctx, |ui| {
            ui.heading("Audio");
            ui.add(
                egui::Slider::new(&mut settings.master_volume, 0.0..=1.0)
                    .text("Master Volume")
            );
            ui.add(
                egui::Slider::new(&mut settings.sfx_volume, 0.0..=1.0)
                    .text("Effects Volume")
            );
            
            ui.separator();
            ui.heading("Graphics");
            
            ui.horizontal(|ui| {
                ui.label("Quality:");
                ui.selectable_value(&mut settings.quality, Quality::Low, "Low");
                ui.selectable_value(&mut settings.quality, Quality::Medium, "Medium");
                ui.selectable_value(&mut settings.quality, Quality::High, "High");
            });
            
            ui.checkbox(&mut settings.vsync, "VSync");
            ui.checkbox(&mut settings.fullscreen, "Fullscreen");
            
            ui.separator();
            ui.heading("Accessibility");
            
            ui.checkbox(&mut settings.screen_shake, "Screen Shake");
            ui.checkbox(&mut settings.high_contrast, "High Contrast Mode");
            ui.add(
                egui::Slider::new(&mut settings.ui_scale, 0.75..=1.5)
                    .text("UI Scale")
            );
            
            ui.separator();
            
            ui.horizontal(|ui| {
                if ui.button("Apply").clicked() {
                    apply_settings(&settings);
                }
                
                if ui.button("Reset").clicked() {
                    *settings = GameSettings::default();
                }
                
                if ui.button("Close").clicked() {
                    *open = false;
                }
            });
        });
}
```

### UI Animation System
```rust
pub struct UIAnimation {
    pub target: Entity,
    pub animation_type: UIAnimationType,
    pub duration: f32,
    pub elapsed: f32,
    pub easing: EasingFunction,
}

pub enum UIAnimationType {
    FadeIn,
    FadeOut,
    SlideIn(Direction),
    SlideOut(Direction),
    Scale(f32, f32),
    Bounce,
}

pub fn update_ui_animations(
    mut animations: Query<&mut UIAnimation>,
    mut transforms: Query<&mut Transform>,
    time: Res<Time>,
) {
    for mut anim in animations.iter_mut() {
        anim.elapsed += time.delta_seconds();
        let t = (anim.elapsed / anim.duration).clamp(0.0, 1.0);
        let eased = anim.easing.apply(t);
        
        if let Ok(mut transform) = transforms.get_mut(anim.target) {
            match anim.animation_type {
                UIAnimationType::Scale(from, to) => {
                    let scale = lerp(from, to, eased);
                    transform.scale = Vec3::splat(scale);
                }
                UIAnimationType::Bounce => {
                    let bounce = 1.0 + (t * PI * 2.0).sin() * 0.1 * (1.0 - t);
                    transform.scale = Vec3::splat(bounce);
                }
                // Other animation types...
            }
        }
    }
}
```

## Success Criteria

- ✅ HUD clearly displays game state
- ✅ Overlays are visually appealing
- ✅ Settings changes apply immediately
- ✅ UI scales properly at all resolutions
- ✅ Animations smooth at 60 FPS
- ✅ Accessibility options functional

## Definition of Done

- [ ] All UI elements implemented
- [ ] Theme system working
- [ ] Settings persist between sessions
- [ ] UI responsive at all resolutions
- [ ] Animations smooth
- [ ] Keyboard navigation works
- [ ] No UI glitches or overlaps
- [ ] README documents UI system

## Notes for Next Sprint

Sprint 10 will add effects and polish:
- Particle effects for impacts
- Screen shake on collisions
- Ball trail effects
- Target destruction animations
- Sound effect integration

The UI system will work with these effects to create a cohesive, polished experience.
