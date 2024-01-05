# Actuate
A Synthesizer, Sampler, and Granulizer built in Rust + Nih-Plug
Written by Ardura

Join the discord! https://discord.com/invite/hscQXkTdfz

![actuate_gui](https://github.com/ardura/Actuate/assets/31751444/be396ad9-b67a-4a67-b457-c8a9911414e5)
![actuate_gui_2](https://github.com/ardura/Actuate/assets/31751444/4be89297-8833-4463-bcd5-ce30ef197450)


## Features
- Two SVF Filters that can be parallel, serial, or bypassed with ADSR Envelopes
- 7 Filter resonance approximations for different sweeps
- 9 Different FX for post processing
- 3 LFO controllers
- 4 Modulators that can be linked to multiple things
- Sampler with pitch shifting or resample stretching
- Sampler supports single cycle waveforms for wavetable-like functions
- Granulizer with ADSR and crossfading between grains
- Any generator can do to any filter
- Samples can be saved into presets
- Stereo width and ultra wide controls

## Signal Path
![actuate](https://github.com/ardura/Actuate/assets/31751444/9066cf62-5077-41be-ade3-da4a51dc46e8)

## Known Issues
- Saving and loading happens twice
- Naming presets and info may be unstable in non-windows environments as I have not tested those sorry
