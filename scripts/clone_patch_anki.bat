@REM the file was created by @dobefore and @redmie
@REM following line command will not echo every line command
@echo off
@REM Allow external definition of anki repository URL
IF "%ANKI_REPO_URL%" == "" GOTO NOURLSET
GOTO END
:NOURLSET
set ANKI_REPO_URL="https://github.com/ankitects/anki"
:END

@REM here e.g. D:\software\vscode_project\anki_sync\anki-sync-server-rs
set PROJECT_ROOT= %CD%
set ANKI_PATCH_FOLDER=%PROJECT_ROOT%\anki_patch
set ANKI_FILE_SUFFIX=_anki_rslib.patch

@REM Set up other variables
set ANKI_TAG=2.1.46
set ANKI_COMMIT=7437ce41ec66a81b0e78096649a503b364a4025a

@REM Clone & patch
echo "Cloning anki from %ANKI_REPO_URL%"
cd %PROJECT_ROOT%
git clone %ANKI_REPO_URL%
echo "Checking out commit %ANKI_COMMIT% and applying patch"
cd anki
git checkout %ANKI_COMMIT%
@REM convert CRLF TO LF
dos2unix %ANKI_PATCH_FOLDER%\%ANKI_COMMIT%%ANKI_FILE_SUFFIX%
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