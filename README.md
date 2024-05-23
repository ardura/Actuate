# Actuate
A Synthesizer, Sampler, and Granulizer built in Rust + Nih-Plug
Written by Ardura

**Please note this project is still a work in progress/alpha - I got a lot of traction once I posted on KVR and wanted to clarify that!**

Join the discord! https://discord.com/invite/hscQXkTdfz
Check out the KVR Page: https://www.kvraudio.com/product/actuate-by-ardura

![image](https://github.com/ardura/Actuate/assets/31751444/9b4cb9fe-de11-4242-a5c0-a0c5b724443d)

[Shortcut to troubleshooting section](##Troubleshooting)

## Features
Hover over any knob (or some labels) for an explanation!

![image](https://github.com/ardura/Actuate/assets/31751444/6c455635-8f03-49b5-bce1-c665d437d2fe)


- Two SVF Filters, a VCF inspired filter, and Tilt inspired filters that can be parallel, serial, or bypassed with ADSR Envelopes
- Pitch modulation with ASDR

![image](https://github.com/ardura/Actuate/assets/31751444/accd4727-975a-4266-a82a-180c55db628d)


- 12 Oscillator shapes:
  - The standard: Sine, Triangle, Saw, Ramp, Square, Pulse, Noise
  - WSaw - Saw with noise variance to create crispyness
  - SSaw - Saw with small variance to create shimmer
  - RSaw - Rounded saw wave
  - RASaw - Rounded saw wave with random variances
  - RSquare - Rounded square wave
- 7 Filter resonance approximations for different sweeps in SVF filters
  - Default - Allegedly the "ideal" response when tying Q to angular sin response
  - Moog - Allegedly a Moog Ladder Q approximation further modified
  - TB - Allegedly an approximation of a TB-303 LP further modified
  - Arp - Allegedly an approximation of an Arp 2600 further modified
  - Res - I made this up - kind of a hyper resonance while still being gentle
  - Bump - I made this up - a gentle bump resonance different from the others
  - Powf - I made this up - Curves based on Powf math function as it scales
- 10 Different FX for post processing

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

## Roadmap
- [x] Create a Preset Browser
- [x] Add more reverb styles
- [ ] Add more decay styles
- [ ] Fix some bandpass glitching on certain filter types
- [ ] Create different stereo spreading algorithms
- [x] Make the GUI nicer - see Discussion https://github.com/ardura/Actuate/discussions/26
- [x] Look into making the preset loading more reliable
- [ ] Fix text input not working (right now it's a OS safe workaround)
- [ ] Fix file dialog in the process thread (right now it's a OS safe workaround)

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
 
## Other Build information
The builds on GitHub and KVR are VST3 and CLAP format, and are compiled on the following machine types:
- Ubuntu 22.04
- Windows' 2022 build (Win10? The Github runner just lists "Windows-2022")
- MacOS 12 (Other MacOS versions are available but I picked 12 for compatibility for now. I have not tested on M1 Macs.)

## Known Issues
- Naming presets and info may be unstable in non-windows environments as I have not tested those sorry
