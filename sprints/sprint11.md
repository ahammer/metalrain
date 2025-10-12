# Sprint 11: Audio System

## Sprint Goal
Implement comprehensive audio system with sound effects, background music, dynamic mixing, and basic positional audio to complete the game's sensory experience.

## Deliverables

### 1. Audio Foundation
- [ ] Integrate bevy_audio/bevy_kira_audio
- [ ] Set up audio asset loading pipeline
- [ ] Create audio manager system
- [ ] Implement volume control
- [ ] Build audio pooling system

### 2. Sound Effects
- [ ] Ball-wall impact sounds
- [ ] Target hit sounds
- [ ] Target destruction sound
- [ ] Ball elimination sound
- [ ] Menu interaction sounds
- [ ] Victory/defeat jingles

### 3. Background Music
- [ ] Menu theme music
- [ ] Gameplay background tracks
- [ ] Dynamic music intensity
- [ ] Smooth track transitions
- [ ] Loop point management

### 4. Audio Mixing
- [ ] Master volume control
- [ ] SFX/Music channel separation
- [ ] Dynamic range compression
- [ ] Audio ducking system
- [ ] Priority-based playback

### 5. Demo: Audio Test
- [ ] Sound effect triggers
- [ ] Music track switching
- [ ] Volume mixer interface
- [ ] Positional audio demo
- [ ] Performance monitoring

## Technical Specifications

### Audio Manager
```rust
use bevy_kira_audio::prelude::*;

#[derive(Resource)]
pub struct AudioManager {
    pub sfx_channel: AudioChannel,
    pub music_channel: AudioChannel,
    pub ui_channel: AudioChannel,
    pub master_volume: f32,
    pub sfx_volume: f32,
    pub music_volume: f32,
}

#[derive(Resource)]
pub struct AudioAssets {
    // Sound Effects
    pub ball_wall_hit: Handle<AudioSource>,
    pub ball_target_hit: Handle<AudioSource>,
    pub target_destroy: Handle<AudioSource>,
    pub ball_eliminate: Handle<AudioSource>,
    pub menu_hover: Handle<AudioSource>,
    pub menu_click: Handle<AudioSource>,
    pub victory: Handle<AudioSource>,
    pub defeat: Handle<AudioSource>,
    
    // Music
    pub menu_theme: Handle<AudioSource>,
    pub gameplay_ambient: Handle<AudioSource>,
    pub gameplay_intense: Handle<AudioSource>,
}

impl AudioManager {
    pub fn play_sfx(&self, audio: &Audio, sound: Handle<AudioSource>, volume: f32) {
        audio.play_in_channel(sound, &self.sfx_channel)
            .with_volume(volume * self.sfx_volume * self.master_volume);
    }
    
    pub fn play_positional(
        &self,
        audio: &Audio,
        sound: Handle<AudioSource>,
        position: Vec2,
        listener_pos: Vec2,
        max_distance: f32,
    ) {
        let distance = position.distance(listener_pos);
        let volume = (1.0 - (distance / max_distance).min(1.0)) 
            * self.sfx_volume 
            * self.master_volume;
        
        if volume > 0.01 {
            // Calculate pan based on position
            let delta = position - listener_pos;
            let pan = (delta.x / max_distance).clamp(-1.0, 1.0);
            
            audio.play_in_channel(sound, &self.sfx_channel)
                .with_volume(volume)
                .with_panning(pan);
        }
    }
}
```

### Impact Sound System
```rust
pub fn handle_collision_sounds(
    mut collision_events: EventReader<CollisionEvent>,
    audio_manager: Res<AudioManager>,
    audio_assets: Res<AudioAssets>,
    audio: Res<Audio>,
    balls: Query<&Velocity, With<Ball>>,
    targets: Query<Entity, With<Target>>,
    walls: Query<Entity, With<Wall>>,
) {
    for event in collision_events.read() {
        if let CollisionEvent::Started(e1, e2, flags) = event {
            // Ball-Wall collision
            if balls.contains(*e1) && walls.contains(*e2) {
                if let Ok(velocity) = balls.get(*e1) {
                    let impact_force = velocity.linvel.length() / 500.0;
                    let volume = (impact_force * 0.5).clamp(0.1, 1.0);
                    
                    audio_manager.play_sfx(
                        &audio,
                        audio_assets.ball_wall_hit.clone(),
                        volume,
                    );
                }
            }
            
            // Ball-Target collision
            if balls.contains(*e1) && targets.contains(*e2) {
                audio_manager.play_sfx(
                    &audio,
                    audio_assets.ball_target_hit.clone(),
                    0.8,
                );
            }
        }
    }
}

pub fn handle_target_destruction_sound(
    mut events: EventReader<TargetDestroyed>,
    audio_manager: Res<AudioManager>,
    audio_assets: Res<AudioAssets>,
    audio: Res<Audio>,
    transforms: Query<&Transform>,
    camera: Query<&Transform, With<Camera>>,
) {
    let listener_pos = camera.single().translation.xy();
    
    for event in events.read() {
        if let Ok(transform) = transforms.get(event.entity) {
            audio_manager.play_positional(
                &audio,
                audio_assets.target_destroy.clone(),
                transform.translation.xy(),
                listener_pos,
                1000.0,
            );
        }
    }
}
```

### Background Music System
```rust
#[derive(Resource)]
pub struct MusicController {
    pub current_track: Option<Handle<AudioInstance>>,
    pub target_volume: f32,
    pub fade_speed: f32,
    pub intensity: f32,
}

pub fn update_music_intensity(
    mut music: ResMut<MusicController>,
    game_state: Res<GameState>,
    time: Res<Time>,
) {
    // Adjust intensity based on game state
    let target_intensity = calculate_music_intensity(&game_state);
    
    music.intensity = lerp(
        music.intensity,
        target_intensity,
        time.delta_seconds() * 2.0,
    );
}

fn calculate_music_intensity(state: &GameState) -> f32 {
    let ball_ratio = state.balls_remaining as f32 / state.initial_balls as f32;
    let target_ratio = state.targets_remaining as f32 / state.initial_targets as f32;
    
    // Intensity increases as balls decrease or targets decrease
    let danger = 1.0 - ball_ratio;
    let progress = 1.0 - target_ratio;
    
    (danger * 0.7 + progress * 0.3).clamp(0.0, 1.0)
}

pub fn manage_background_music(
    mut music: ResMut<MusicController>,
    audio: Res<AudioChannel<MusicChannel>>,
    audio_assets: Res<AudioAssets>,
    game_phase: Res<State<GamePhase>>,
    time: Res<Time>,
) {
    let (target_track, target_volume) = match game_phase.get() {
        GamePhase::MainMenu => (Some(audio_assets.menu_theme.clone()), 0.5),
        GamePhase::Playing => {
            let track = if music.intensity > 0.6 {
                audio_assets.gameplay_intense.clone()
            } else {
                audio_assets.gameplay_ambient.clone()
            };
            (Some(track), 0.3)
        }
        GamePhase::Paused => (None, 0.1), // Duck music when paused
        _ => (None, 0.0),
    };
    
    // Handle track transitions
    if let Some(new_track) = target_track {
        if music.current_track.is_none() {
            let instance = audio.play(new_track)
                .looped()
                .with_volume(0.0)
                .handle();
            music.current_track = Some(instance);
        }
    }
    
    // Smooth volume transitions
    if let Some(instance) = &music.current_track {
        let current_volume = audio.get_volume(instance);
        let new_volume = lerp(
            current_volume,
            target_volume,
            time.delta_seconds() * music.fade_speed,
        );
        audio.set_volume(instance, new_volume);
    }
}
```

### Audio Configuration
```toml
[audio]
master_volume = 0.8
sfx_volume = 1.0
music_volume = 0.6
ui_volume = 0.8

[audio.mixing]
compression_enabled = true
compression_threshold = 0.8
compression_ratio = 4.0

[audio.music]
fade_duration = 1.5
crossfade_enabled = true
dynamic_intensity = true

[audio.effects]
reverb_enabled = false
reverb_mix = 0.2
delay_enabled = false
delay_time = 0.25
```

### Audio Asset Loading
```rust
pub fn load_audio_assets(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    commands.insert_resource(AudioAssets {
        // Sound Effects
        ball_wall_hit: asset_server.load("audio/sfx/ball_wall_hit.ogg"),
        ball_target_hit: asset_server.load("audio/sfx/ball_target_hit.ogg"),
        target_destroy: asset_server.load("audio/sfx/target_destroy.ogg"),
        ball_eliminate: asset_server.load("audio/sfx/ball_eliminate.ogg"),
        menu_hover: asset_server.load("audio/sfx/menu_hover.ogg"),
        menu_click: asset_server.load("audio/sfx/menu_click.ogg"),
        victory: asset_server.load("audio/sfx/victory.ogg"),
        defeat: asset_server.load("audio/sfx/defeat.ogg"),
        
        // Music
        menu_theme: asset_server.load("audio/music/menu_theme.ogg"),
        gameplay_ambient: asset_server.load("audio/music/gameplay_ambient.ogg"),
        gameplay_intense: asset_server.load("audio/music/gameplay_intense.ogg"),
    });
}
```

### Sound Effect Variations
```rust
pub struct SoundVariation {
    pub base_sound: Handle<AudioSource>,
    pub pitch_range: Range<f32>,
    pub volume_range: Range<f32>,
}

impl SoundVariation {
    pub fn play(&self, audio: &Audio, channel: &AudioChannel) {
        let pitch = rand::thread_rng().gen_range(self.pitch_range.clone());
        let volume = rand::thread_rng().gen_range(self.volume_range.clone());
        
        audio.play_in_channel(self.base_sound.clone(), channel)
            .with_volume(volume)
            .with_playback_rate(pitch);
    }
}
```

## Audio Guidelines

### Sound Design Principles
1. **Clarity**: Each sound should be distinct and recognizable
2. **Feedback**: Immediate audio response to player actions
3. **Hierarchy**: Important sounds should cut through mix
4. **Consistency**: Similar actions produce similar sounds
5. **Polish**: No clicks, pops, or abrupt cuts

### Mix Levels
- UI sounds: -6 dB
- Impact sounds: -3 dB
- Destruction: 0 dB
- Music: -12 dB
- Victory/Defeat: -3 dB

## Success Criteria

- ✅ All game actions have appropriate sounds
- ✅ Music enhances atmosphere
- ✅ No audio latency or stuttering
- ✅ Volume controls work correctly
- ✅ Positional audio adds depth
- ✅ Performance impact minimal

## Definition of Done

- [ ] Audio system integrated
- [ ] All sound effects implemented
- [ ] Background music working
- [ ] Mixing system functional
- [ ] Settings integration complete
- [ ] No audio bugs or glitches
- [ ] Asset loading optimized
- [ ] README documents audio system

## Notes for Next Sprint

Sprint 12 (final) will focus on optimization and polish:
- Performance profiling
- Build optimization
- Final bug fixes
- Release preparation
- Documentation completion

The audio system completes the core game experience, ready for final optimization.
