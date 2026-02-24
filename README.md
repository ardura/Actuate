# Actuate (Latest is v1.4.3)

A Subtractive and Additive Synthesizer, Sampler, and Granulizer built in Rust + Nih-Plug
Written by Ardura

**Please note this project is still a work in progress/alpha - I got a lot of traction once I posted on KVR and wanted to clarify that!**

Join the discord! https://discord.com/invite/hscQXkTdfz
Check out the KVR Page: https://www.kvraudio.com/product/actuate-by-ardura

![image](https://github.com/ardura/Actuate/assets/31751444/9b4cb9fe-de11-4242-a5c0-a0c5b724443d)

[Shortcut to troubleshooting section](#Troubleshooting)

## Features
Hover over any knob (or some labels) for an explanation!

![image](https://github.com/ardura/Actuate/assets/31751444/6c455635-8f03-49b5-bce1-c665d437d2fe)


- Two SVF Filters with unique resonance features, a VCF inspired filter, Tilt inspired filters, and three other analog-inspired filters that can be parallel, serial, or bypassed with ADSR Envelopes
- Pitch modulation with ASDR

![image](https://github.com/ardura/Actuate/assets/31751444/accd4727-975a-4266-a82a-180c55db628d)


- 17 Subtractive Oscillator shapes:
  - The standard: Sine, Triangle, Saw, Ramp, Square, Pulse, Noise
  - WSaw - Saw with noise variance to create crispyness
  - SSaw - Saw with small variance to create shimmer
  - RSaw - Rounded saw wave
  - RASaw - Rounded saw wave with random variances
  - RSquare - Rounded square wave
  - SkewSaw - A Saw with the rise skewed in one direction
  - Bent Saw - A Saw wave with an incomplete bend starting another saw in the middle
  - Step Saw - Looks like a staircase
  - ScSaw - An 'S' shaped Saw with a Cubic for the curve
  - AsymSaw - An asymmetrical saw shape
- Additive Oscillators with up to 16 harmonics
- FM Supported between Oscillators/samples/granulizer
- 5 Main Filter Algorithms
  - SVF - 7 Filter resonance approximations for different sweeps in SVF filters
    - Default - Allegedly the "ideal" response when tying Q to angular sin response
    - Moog - Allegedly a Moog Ladder Q approximation further modified
    - TB - Allegedly an approximation of a TB-303 LP further modified
    - Arp - Allegedly an approximation of an Arp 2600 further modified
    - Res - I made this up - kind of a hyper resonance while still being gentle
    - Bump - I made this up - a gentle bump resonance different from the others
    - Powf - I made this up - Curves based on Powf math function as it scales
  - Tilt Filter
  - VCF Filter
  - V4 - Analog inspired filter idea (Ardura's V4 - Use this one for adding tone rather than filtering)
  - A4I - Analog inspired filter idea (Ardura's A4I)
  - A4II - Analog inspired filter idea (Ardura's A4I Take II)
  - A4III - Analog inspired filter idea (Ardura's A4II with some tweaks)
- 11 Different FX for post processing

![image](https://github.com/ardura/Actuate/assets/31751444/c13b62bb-a29e-420c-9f3a-764950cbd4a2)

- 3 LFO controllers

![image](https://github.com/ardura/Actuate/assets/31751444/22499e32-50e4-4724-9483-de5ceb43751a)

- 4 Modulators that can be linked to multiple things

![image](https://github.com/ardura/Actuate/assets/31751444/67d7cdeb-9214-4eef-ad8b-63b6a03ceb60)

- Sampler with pitch shifting or resample stretching
- Sampler supports single cycle waveforms for wavetable-like functions
- Granulizer with ADSR and crossfading between grains
- Any generator can go to any filter
- Samples can be saved into presets
- Stereo width and ultra wide controls

## Signal Path
![actuate_flow](https://github.com/ardura/Actuate/assets/31751444/45ce1d56-d6c1-47b2-8bae-09633ecbbd2e)

## Plugin Preset Installation!!
*NOTE: Actuate has a "Download latest presets" Button in the browser!*

Actuate will install presets and banks here, where USER is your username on your system:

- Linux: `/home/USER/Documents/ActuateDB/`
- macOS: `/Users/USER/Documents/ActuateDB/`
- Windows: `C:\Users\USER\Documents\ActuateDB\`

## Troubleshooting
Since Actuate 1.2.8 the new file browser and UI use native text editing. This created some issues in some DAWs outlined here:

- **FL Studio:** No issues!
- **Ardour:** No issues!
- **Bitwig:** When using text input you need to use **<shift + spacebar>** for space key input
- **Reaper:** VST3 and CLAP text input works if you use "send all keyboard input to plugin" in the FX menu

![image](https://github.com/ardura/Actuate/assets/31751444/1664ef3f-ec4c-453b-81e8-d0b7e13a5811)

- **Ableton:** 
  - Text input works if you add "-_EnsureKeyMessagesForPlugins" to Options.txt in preferences. See https://forum.ableton.com/viewtopic.php?t=97318
  - Unfortunately I don't know where this would be on Linux or Mac so I'm open to help from Ableton users!

## DAWS and compatibility
- Compatible with Windows 10 and up
- Compatible with Linux
- Compatible with Mac
- DAWS
    - FL Studio tested compatible
    - Ableton tested compatible
    - Reaper tested compatible
    - Ardour tested **compatible with some reported performance issues**
    - Bitwig tested compatible
    - Cantibile tested **uncompatible and has issues**
    - VSTHost tested **compatible but has gui issues**

## Building/Compiling Actuate Manually
- You should do this if the precompiled binary fails or you have a unique system configuration (like linux)

After installing [Rust](https://rustup.rs/) on your system (and possibly restarting your terminal session), you can compile Actuate as follows:
1. Make sure your dependencies are installed. These are the packages you will need at minimum: `libgl1-mesa-dev libglu1-mesa-dev libxcursor-dev libxkbcommon-x11-dev libatk1.0-dev build-essential libgtk-3-dev libxcb-dri2-0-dev libxcb-icccm4-dev libx11-xcb-dev`
   - Note I have also found on some systems `libc6` or `glibc` needs to be installed depending on your configuration
2. Run the build process in a terminal from the Actuate root directory
```
cargo xtask bundle Actuate --profile release
```
Or use the following for debugging:
```
cargo xtask bundle Actuate --profile profiling
```
3. Your outputs will be in the Actuate/target/bundled directory.
4. the `*.clap` you can copy to your clap directory/path, the vst3 one needs the folder structure copied on linux

## Other Build information
The builds on GitHub and KVR are VST3 and CLAP format, and are compiled on the following machine types:
- Ubuntu 22.04
- Windows' 2022 build (Win10? The Github runner just lists "Windows-2022")
- MacOS 12
- The MacOS M1 build is on OS 14

## Known Issues
- Naming presets and info may be unstable in non-windows environments as I have not tested those sorry
