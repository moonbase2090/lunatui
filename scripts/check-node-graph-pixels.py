#!/usr/bin/env python3
# Copyright (c) 2026 Moonbase 2090 LLC
# SPDX-License-Identifier: MIT
"""Verify actual paused pixel/cell/pixel and theme transitions in the Kitty capture."""
import argparse
import hashlib,json,subprocess
from pathlib import Path
parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument("directory",type=Path)
root=parser.parse_args().directory
path=root/'terminal.mkv'
frames=[]
for second in [3.5,4.5,5.5,6.5]:
 raw=subprocess.check_output(['ffmpeg','-loglevel','error','-i',str(path),'-ss',str(second),'-frames:v','1','-pix_fmt','rgb24','-f','rawvideo','-'])
 assert raw,'missing real framebuffer frame'
 frames.append(hashlib.sha256(raw).hexdigest())
assert frames[0]!=frames[1], 'pixel toggle did not change real displayed geometry'
assert frames[0]==frames[2], 're-enabled pixels did not restore the same paused scene'
assert frames[2]!=frames[3], 'theme switch did not change real displayed frame'
print(json.dumps({'sample_seconds':[3.5,4.5,5.5,6.5],'frame_sha256':frames,'pixel_toggle_restores':True,'theme_switch_changes':True},indent=2))
