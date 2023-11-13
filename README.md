# Actuate
A Synthesizer, Sampler, and Granulizer built in Rust + Nih-Plug
Written by Ardura

![actuate_gui](https://github.com/ardura/Actuate/assets/31751444/a6494f45-9808-4c6e-ac4e-a30a7d5b6537)
![actuate_gui_2](https://github.com/ardura/Actuate/assets/31751444/15610d24-e5c0-4cbf-ac45-f0052787c554)

## Features
- Two SVF Filters that can be parallel, serial, or bypassed with ADSR Envelopes
- 4 Filter resonance approximations for different sweeps
- 3 LFO controllers
- 4 Modulators that can be linked to multiple things
- Sampler with pitch shifting or resample stretching
- Sampler supports single cycle waveforms for wavetable-like functions
- Granulizer with ADSR and crossfading between grains
- Any generator can do to any filter
- Samples can be saved into presets
- Stereo width and ultra wide controls

## Signal Path
![actuate](https://github.com/ardura/Actuate/assets/31751444/f0c42227-ae31-4f96-815d-d55ddd92f20a)

## Known Issues
- Saving and loading happens twice
