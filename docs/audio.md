# Audio System

Forge2D provides audio support through the `AudioSystem`, which wraps `rodio` for cross-platform audio playback.

## Checking Audio Availability

The audio system may not be available on all systems. Always check before use:

```rust
if ctx.audio().is_available() {
    // Audio is available, safe to use
} else {
    // Audio not available (graceful degradation)
}
```

## Playing Sound Effects

### From Bytes

```rust
if ctx.audio().is_available() {
    // Load sound bytes (WAV, OGG, MP3, etc.)
    let sound_bytes = include_bytes!("assets/jump.wav");
    ctx.audio().play_sound_from_bytes(sound_bytes)?;
}
```

### Preload and Reuse (Recommended)

```rust
struct MyGame {
    jump_sound: Option<forge2d::SoundHandle>,
}

impl Game for MyGame {
    fn init(&mut self, ctx: &mut EngineContext) -> Result<()> {
        if ctx.audio().is_available() {
            self.jump_sound = Some(
                ctx.audio().load_sound("assets/jump.wav")?
            );
        }
        Ok(())
    }

    fn update(&mut self, ctx: &mut EngineContext) -> Result<()> {
        if ctx.input().is_key_pressed(VirtualKeyCode::Space) {
            if let Some(handle) = self.jump_sound {
                ctx.audio().play_sound_handle(handle)?;
            }
        }
        Ok(())
    }
}
```

### Supported Formats

The audio system supports formats supported by `rodio`:
- WAV
- OGG Vorbis
- MP3
- FLAC
- And more (see rodio documentation)

## Playing Background Music

### Looping Music

```rust
if ctx.audio().is_available() {
    let music_bytes = include_bytes!("assets/background_music.ogg");
    ctx.audio().play_music_loop_from_bytes(music_bytes)?;
}
```

### Stopping Music

```rust
ctx.audio().stop_music();
```

## AudioSystem API

### Methods

- **`is_available() -> bool`** - Check if audio system is available
- **`play_sound_from_bytes(bytes: &[u8]) -> Result<()>`** - Play sound effect from bytes
- **`load_sound(path: impl AsRef<Path>) -> Result<SoundHandle>`** - Preload a sound effect from disk
- **`load_sound_from_bytes(key: &str, bytes: &[u8]) -> Result<SoundHandle>`** - Preload a sound effect from bytes
- **`play_sound_handle(handle: SoundHandle) -> Result<()>`** - Play a preloaded sound effect
- **`play_music_loop_from_bytes(bytes: &[u8]) -> Result<()>`** - Play looping background music
- **`stop_music()`** - Stop currently playing music

## Graceful Degradation

If audio initialization fails (e.g., no audio device), the engine continues to run without audio. Your game should check `is_available()` before using audio features.

## Best Practices

1. **Check Availability** - Always check `is_available()` before using audio
2. **Load in init()** - Consider preloading audio files in `init()`
3. **Embed Small Sounds** - Small sound effects can be embedded using `include_bytes!`
4. **Handle Errors** - Audio playback may fail, handle errors gracefully

## Example

```rust
impl Game for MyGame {
    fn init(&mut self, ctx: &mut EngineContext) -> Result<()> {
        // Check audio availability
        if ctx.audio().is_available() {
            println!("Audio system is available!");
        } else {
            println!("Audio system is not available (this is okay)");
        }
        Ok(())
    }
    
    fn update(&mut self, ctx: &mut EngineContext) -> Result<()> {
        // Play sound on jump
        if ctx.input().is_key_pressed(VirtualKeyCode::Space) {
            if ctx.audio().is_available() {
                let jump_sound = include_bytes!("assets/jump.wav");
                ctx.audio().play_sound_from_bytes(jump_sound)?;
            }
        }
        Ok(())
    }
}
```

## Future Enhancements

Potential future additions to the audio system:
- Volume control
- Sound effect pooling
- 3D positional audio
- Audio streaming for large files
