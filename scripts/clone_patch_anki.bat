@REM How to apply a patch
@REM try running git diff origin/master after switching to a new commit(update anki lib)

@REM following line command will not echo every line command
@echo off
@REM define variable
set ANKI_REPO_URL="https://github.com/ankitects/anki"
set ANKI_COMMIT=5dab7ed47ec6d17226d2fc0529c32a56e40e5f8a
@REM set ANKI_COMMIT=44342660d834e5a966c18f6984bac0369139e1bf
@REM here e.g. D:\software\vscode_project\anki_sync\anki-sync-server-rs
set PROJECT_ROOT= %CD%
set ANKI_PATCH_FOLDER=%PROJECT_ROOT%\anki_patch
set ANKI_FILE_SUFFIX=_anki_rslib.patch

@REM return variable with %v%
echo "Cloning anki from %ANKI_REPO_URL%"
cd %PROJECT_ROOT%
git clone %ANKI_REPO_URL%
echo "Checking out commit %ANKI_COMMIT% and applying patch"
cd anki
git checkout %ANKI_COMMIT%
git apply %ANKI_PATCH_FOLDER%\%ANKI_COMMIT%%ANKI_FILE_SUFFIX%



@REM How to create a patch file

@REM clone anki repo

@REM get original commit ID 5dab7ed47ec6d17226d2fc0529c32a56e40e5f8a
@REM git rev-parse HEAD

@REM make changes to anki lib ,e.g.add pub to structs...
@REM git commit

@REM get current commit ID  480a572137a51316bab2d97f2435cdfe328c462c
@REM git rev-parse HEAD

@REM use current commit ID to patch
@REM git format-patch 480a572137a51316bab2d97f2435cdfe328c462c -1

@REM you can rename patch file with original commit ID
@REM ren .\0001-create-patch.patch 5dab7ed47ec6d17226d2fc0529c32a56e40e5f8a_anki_rslib.patch

@REM last and not least, convert CRLF in patch if it is to LF using dos2unix
@REM dos2unix %ANKI_PATCH_FOLDER%\%ANKI_COMMIT%%ANKI_FILE_SUFFIX%