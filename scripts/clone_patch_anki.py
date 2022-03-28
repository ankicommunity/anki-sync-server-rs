# this file is used for gitgub actions.

import subprocess
import os
from pathlib import Path
ANKI_REPO_URL="https://github.com/ankitects/anki"
ANKI_COMMIT='5dab7ed47ec6d17226d2fc0529c32a56e40e5f8a'

# get path 
# .../anki-sync-server-rs
PROJECT_ROOT=Path(os.getcwd())
ANKI_PATCH_FOLDER=PROJECT_ROOT.joinpath("anki_patch")
ANKI_FILE_SUFFIX= "_anki_rslib.patch"
ANKI_PATCH_FILE_PATH=ANKI_PATCH_FOLDER.joinpath(ANKI_COMMIT+ANKI_FILE_SUFFIX)

print(f"Cloning anki from {ANKI_REPO_URL}")
os.chdir(PROJECT_ROOT)
subprocess.run(['git','clone',ANKI_REPO_URL])
print(f"Checking out commit {ANKI_COMMIT} and applying patch")

os.chdir("anki")
subprocess.run(['git','checkout',ANKI_COMMIT])
subprocess.run(['git','apply',ANKI_PATCH_FILE_PATH])



