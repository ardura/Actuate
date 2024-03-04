# Actuate
A Synthesizer, Sampler, and Granulizer built in Rust + Nih-Plug
Written by Ardura

Join the discord! https://discord.com/invite/hscQXkTdfz
Check out the KVR Page: https://www.kvraudio.com/product/actuate-by-ardura

![Screenshot 2024-03-01 085443](https://github.com/ardura/Actuate/assets/31751444/9c06b017-99c9-44b7-9dd6-e994c3f3db77)

## Features
- Two SVF Filters, a VCF inspired filter, and Tilt inspired filters that can be parallel, serial, or bypassed with ADSR Envelopes
![Screenshot 2024-03-01 085849](https://github.com/ardura/Actuate/assets/31751444/5940f589-63f3-40c8-b639-a8c20c76a32a)

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
![Screenshot 2024-03-01 085630](https://github.com/ardura/Actuate/assets/31751444/2b398ff3-2a17-4ea8-81a2-c9b3d44dfaeb)

- 3 LFO controllers
![2](https://github.com/ardura/Actuate/assets/31751444/b9904160-5a66-400a-8e66-2a77ba9743f4)

- 4 Modulators that can be linked to multiple things
![Screenshot 2024-03-01 085601](https://github.com/ardura/Actuate/assets/31751444/f1d0e4a8-f77f-40d3-b754-b6e28b9c9152)

- Sampler with pitch shifting or resample stretching
- Sampler supports single cycle waveforms for wavetable-like functions
- Granulizer with ADSR and crossfading between grains
- Any generator can go to any filter
- Samples can be saved into presets
- Stereo width and ultra wide controls

## Signal Path
![actuate_flow](https://github.com/ardura/Actuate/assets/31751444/45ce1d56-d6c1-47b2-8bae-09633ecbbd2e)


## Known Issues
- Naming presets and info may be unstable in non-windows environments as I have not tested those sorry
