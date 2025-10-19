# Using `stdout` and `ffmpeg`

This guide highlights using the `stdout` with `ffmpeg`. Once you're happy that a simulation looks good with `glrender`, you may want to encode it. `stdout` renders each frame of the simulation to stdout as 8bit RGBA pixels. `ffmpeg` can read this via a pipe. `stdout`'s properties are summarised below
```bash
$ physcan stdout
Overview
      Name - stdout
     Blurb - Render simulation to stdout as 8bit RGBA pixels for further processing by video software
      Kind - Render

Properties
resolution - Choices of 4k, 1080p and 720p
      zoom - Camera zoom (1.0 is default)
    shader - yellowblue, velocity, rgb-velocity, smoke, twinkle, id, orange-blue, hot
buffer_size - Number of frames to buffer before writing
     frame - If specified, only one frame will be generated at the timestep specified by frame

Meta data
   Authors - Joseph Briggs <jhbriggs23@gmail.com>
   License - MIT
   Version - 0.4.1
Repository - https://github.com/jhb123/physim
```
## Rendering a video

Since physim simulations have lots of small moving particles, you may need a high bitrate to reduce the arteficts due to compression.
```bash
$ physim global iterations=100 dt=0.1 ! cube ! astro theta=1.3 ! rk4 ! \
 stdout zoom=1.5 resolution=1080p | \
 ffmpeg -y -f rawvideo -pixel_format rgba -video_size 1920x1080 -framerate 60 -i pipe:0 -c:v libx265 -preset slow -crf 16 -x265-params "no-sao=1:deblock=-6,-6:aq-mode=3:keyint=30:level-idc=5.1:tier=high" -pix_fmt yuv420p10le -vf "eq=saturation=1.2" -b:v 50M cube.mp4
```
## Making a thumbnail

To make a thumbnail, you can use the `frame` parameter. The example below makes a screen shot of 50th iteration.
```bash
$ physim global iterations=100 dt=0.1 ! cube ! astro theta=1.3 ! rk4 ! \
 stdout zoom=1.5 resolution=1080p frame=50 | \
ffmpeg -f rawvideo -pix_fmt rgba -s 1920x1080 -i - -frames:v 1 -vf format=rgb24 cube.png
```

## Other handy ffmpeg commands

Add audio with `ffmpeg -i input.mp4 -i input.mp3 -c:v copy -c:a aac -shortest output.mp4`
