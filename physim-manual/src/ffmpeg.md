# Using `stdout` and FFmpeg

This guide demonstrates using the `stdout` with FFmpeg. Once you're happy that a simulation looks good with `glrender`, you may want to encode it. `stdout` renders each frame of the simulation to stdout as 8bit, RGBA pixels. FFmpeg can read this via a pipe. `stdout`'s can produce 720p, 1080p and 4K (3840 × 2160) video. The 
## Rendering a video
Since `physim` simulations can have lots of small moving particles, you may need a high bitrate to reduce the artefacts due to compression.
```bash
$ physim global iterations=100 dt=0.1 ! cube ! astro theta=1.3 ! rk4 ! \
 stdout zoom=1.5 resolution=1080p | \
 ffmpeg -y -f rawvideo -pixel_format rgba -video_size 1920x1080 -framerate 60 -i pipe:0 -c:v libx265 -preset slow -crf 16 -x265-params "no-sao=1:deblock=-6,-6:aq-mode=3:keyint=30:level-idc=5.1:tier=high" -pix_fmt yuv420p10le -vf "eq=saturation=1.2" -b:v 50M cube.mp4
```
## Making a thumbnail
To make a thumbnail, you can use the `frame` parameter. The example below makes a screenshot of the 50th iteration of the simulation.
```bash
$ physim global iterations=100 dt=0.1 ! cube ! astro theta=1.3 ! rk4 ! \
 stdout zoom=1.5 resolution=1080p frame=50 | \
ffmpeg -f rawvideo -pix_fmt rgba -s 1920x1080 -i - -frames:v 1 -vf format=rgb24 cube.png
```
## Other handy ffmpeg commands

Add audio with `ffmpeg -i input.mp4 -i input.mp3 -c:v copy -c:a aac -shortest output.mp4`
