# this file is for removing unused files e.g.`.py`... after patching to anki lib
# and should be run in script/clone_patch_anki
# working dir should be under ./anki
import os
from pathlib import Path
def recurive_remove_files(root_dir):
    print(f'walk dir {root_dir}')
    for root, dirs, files in os.walk(root_dir):
        if files!=[]:
            for f in files:
                file_path=Path(root).joinpath(f)
                print(f'remove file {file_path}')
                os.remove(file_path)
        # have second dir
        if dirs!=[]:
            for d in dirs:
                rootdir=Path(root).joinpath(d)
                recurive_remove_files(rootdir)
# get project dir root of anki-sync-server-rs
# first remove folder .git manially

dir_list=["sass","ts","python","pylib","qt","tools"]
anki_dir=Path(os.getcwd()).joinpath("anki")

for i in dir_list:
    d=anki_dir.joinpath(i)
    recurive_remove_files(d)



